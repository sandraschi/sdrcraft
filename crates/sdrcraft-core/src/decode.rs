//! Symbol decoders for modes that are published in the clear.
//!
//! Morse, RTTY, and APRS are copied from a waveform or a frame we build in
//! this module. Live IQ from the stick is not in the window yet, so `copy_tapes`
//! says so and still runs the same functions a live stream will call.

/// Practice tapes: real modulators into the real decoders.
pub fn copy_tapes() -> (String, String, String) {
    let morse =
        decode_morse_envelope(&encode_morse("CQ DE SDR", 2_000.0, 20.0)).unwrap_or_else(|e| e);
    let rtty = decode_rtty_bits(&encode_rtty_bits("SDR")).unwrap_or_else(|e| e);
    let aprs = decode_ax25(&encode_ax25("SDR", "APRS", ">hi")).unwrap_or_else(|e| e);
    (morse, rtty, aprs)
}

pub fn encode_morse(text: &str, sample_rate: f64, wpm: f64) -> Vec<f32> {
    let dit = ((sample_rate * 1.2 / wpm).round() as usize).max(1);
    let mut out = Vec::new();
    push_level(&mut out, false, dit * 7);
    let mut first_word = true;
    for word in text.split(' ') {
        if word.is_empty() {
            continue;
        }
        if !first_word {
            // Previous letter already left 3 dits. 3 + 4 = the 7-dit word gap.
            push_level(&mut out, false, dit * 4);
        }
        first_word = false;
        for ch in word.chars() {
            let Some(pattern) = morse_pattern(ch) else {
                continue;
            };
            for (i, element) in pattern.chars().enumerate() {
                if i > 0 {
                    push_level(&mut out, false, dit);
                }
                let dits = if element == '-' { 3 } else { 1 };
                push_level(&mut out, true, dit * dits);
            }
            push_level(&mut out, false, dit * 3);
        }
    }
    out
}

pub fn decode_morse_envelope(samples: &[f32]) -> Result<String, String> {
    let mut runs = Vec::new();
    let mut current_on = false;
    let mut count = 0usize;
    let mut started = false;
    for sample in samples {
        let on = *sample >= 0.5;
        if !started {
            current_on = on;
            started = true;
        }
        if on == current_on {
            count += 1;
        } else {
            runs.push((current_on, count));
            current_on = on;
            count = 1;
        }
    }
    if started {
        runs.push((current_on, count));
    }
    let mut dit = 0usize;
    let mut saw_mark = false;
    for (on, len) in &runs {
        if *on && (!saw_mark || *len < dit) {
            dit = *len;
            saw_mark = true;
        }
    }
    if !saw_mark || dit == 0 {
        return Err("no keyed signal".to_string());
    }
    let mut symbols = String::new();
    let mut text = String::new();
    for (on, len) in &runs {
        if *on {
            if *len > dit * 2 {
                symbols.push('-');
            } else {
                symbols.push('.');
            }
        } else if *len >= dit * 6 {
            push_morse_letter(&mut text, &symbols);
            symbols.clear();
            if !text.ends_with(' ') {
                text.push(' ');
            }
        } else if *len >= dit * 2 {
            push_morse_letter(&mut text, &symbols);
            symbols.clear();
        }
    }
    push_morse_letter(&mut text, &symbols);
    let cleaned = text.trim().to_string();
    if cleaned.is_empty() {
        Err("no morse letters".to_string())
    } else {
        Ok(cleaned)
    }
}

/// RTTY as already-decided mark/space bits: start(0), 5 data LSB first, stop(1).
pub fn encode_rtty_bits(text: &str) -> Vec<u8> {
    let mut bits = vec![1, 1, 1];
    for ch in text.chars() {
        let Some(code) = baudot_letter(ch) else {
            continue;
        };
        bits.push(0);
        for i in 0..5 {
            bits.push((code >> i) & 1);
        }
        bits.push(1);
        bits.push(1);
    }
    bits
}

pub fn decode_rtty_bits(bits: &[u8]) -> Result<String, String> {
    let mut text = String::new();
    let mut i = 0usize;
    while i + 8 <= bits.len() {
        let start = match bits.get(i) {
            Some(b) => *b,
            None => break,
        };
        if start != 0 {
            i += 1;
            continue;
        }
        let mut code = 0u8;
        for k in 0..5 {
            let bit = match bits.get(i + 1 + k) {
                Some(b) => *b,
                None => return Err("short rtty character".to_string()),
            };
            code |= bit << k;
        }
        if let Some(ch) = baudot_char(code) {
            text.push(ch);
        }
        i += 8;
    }
    if text.is_empty() {
        Err("no rtty letters".to_string())
    } else {
        Ok(text)
    }
}

/// AX.25 UI frame, no flags. Callsigns are up to 6 letters.
pub fn encode_ax25(source: &str, dest: &str, info: &str) -> Vec<u8> {
    let mut frame = Vec::new();
    push_address(&mut frame, dest, false);
    push_address(&mut frame, source, true);
    frame.push(0x03);
    frame.push(0xf0);
    frame.extend(info.as_bytes());
    let crc = fcs(&frame);
    frame.push((crc & 0xff) as u8);
    frame.push((crc >> 8) as u8);
    frame
}

pub fn decode_ax25(frame: &[u8]) -> Result<String, String> {
    if frame.len() < 16 {
        return Err("ax25 frame too short".to_string());
    }
    let body_len = frame.len() - 2;
    let body = match frame.get(..body_len) {
        Some(b) => b,
        None => return Err("ax25 frame too short".to_string()),
    };
    let got = fcs(body);
    let lo = *frame.get(body_len).unwrap_or(&0) as u16;
    let hi = *frame.get(body_len + 1).unwrap_or(&0) as u16;
    if got != lo | (hi << 8) {
        return Err("ax25 fcs mismatch".to_string());
    }
    let dest = read_address(body, 0)?;
    let source = read_address(body, 7)?;
    let info = match body.get(16..) {
        Some(bytes) => String::from_utf8_lossy(bytes).to_string(),
        None => String::new(),
    };
    Ok(format!("{source}>{dest}:{info}"))
}

fn push_level(out: &mut Vec<f32>, on: bool, n: usize) {
    let level = if on { 1.0 } else { 0.0 };
    out.extend(std::iter::repeat_n(level, n));
}

fn push_morse_letter(text: &mut String, symbols: &str) {
    if symbols.is_empty() {
        return;
    }
    text.push(morse_letter(symbols).unwrap_or('?'));
}

fn push_address(frame: &mut Vec<u8>, call: &str, last: bool) {
    let mut chars = [' '; 6];
    for (i, ch) in call.chars().take(6).enumerate() {
        if let Some(slot) = chars.get_mut(i) {
            *slot = ch.to_ascii_uppercase();
        }
    }
    for ch in chars {
        frame.push((ch as u8) << 1);
    }
    let ssid = if last { 0x61 } else { 0x60 };
    frame.push(ssid);
}

fn read_address(frame: &[u8], at: usize) -> Result<String, String> {
    let mut call = String::new();
    for i in 0..6 {
        let byte = *frame
            .get(at + i)
            .ok_or_else(|| "short address".to_string())?;
        let ch = (byte >> 1) as char;
        if ch != ' ' {
            call.push(ch);
        }
    }
    Ok(call)
}

fn fcs(data: &[u8]) -> u16 {
    let mut crc = 0xffffu16;
    for byte in data {
        crc ^= *byte as u16;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0x8408;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn morse_pattern(ch: char) -> Option<&'static str> {
    Some(match ch.to_ascii_uppercase() {
        'A' => ".-",
        'B' => "-...",
        'C' => "-.-.",
        'D' => "-..",
        'E' => ".",
        'F' => "..-.",
        'G' => "--.",
        'H' => "....",
        'I' => "..",
        'J' => ".---",
        'K' => "-.-",
        'L' => ".-..",
        'M' => "--",
        'N' => "-.",
        'O' => "---",
        'P' => ".--.",
        'Q' => "--.-",
        'R' => ".-.",
        'S' => "...",
        'T' => "-",
        'U' => "..-",
        'V' => "...-",
        'W' => ".--",
        'X' => "-..-",
        'Y' => "-.--",
        'Z' => "--..",
        '0' => "-----",
        '1' => ".----",
        '2' => "..---",
        '3' => "...--",
        '4' => "....-",
        '5' => ".....",
        '6' => "-....",
        '7' => "--...",
        '8' => "---..",
        '9' => "----.",
        _ => return None,
    })
}

fn morse_letter(symbols: &str) -> Option<char> {
    const TABLE: &[(&str, char)] = &[
        (".-", 'A'),
        ("-...", 'B'),
        ("-.-.", 'C'),
        ("-..", 'D'),
        (".", 'E'),
        ("..-.", 'F'),
        ("--.", 'G'),
        ("....", 'H'),
        ("..", 'I'),
        (".---", 'J'),
        ("-.-", 'K'),
        (".-..", 'L'),
        ("--", 'M'),
        ("-.", 'N'),
        ("---", 'O'),
        (".--.", 'P'),
        ("--.-", 'Q'),
        (".-.", 'R'),
        ("...", 'S'),
        ("-", 'T'),
        ("..-", 'U'),
        ("...-", 'V'),
        (".--", 'W'),
        ("-..-", 'X'),
        ("-.--", 'Y'),
        ("--..", 'Z'),
        ("-----", '0'),
        (".----", '1'),
        ("..---", '2'),
        ("...--", '3'),
        ("....-", '4'),
        (".....", '5'),
        ("-....", '6'),
        ("--...", '7'),
        ("---..", '8'),
        ("----.", '9'),
    ];
    TABLE
        .iter()
        .find(|(pat, _)| *pat == symbols)
        .map(|(_, ch)| *ch)
}

fn baudot_letter(ch: char) -> Option<u8> {
    Some(match ch.to_ascii_uppercase() {
        'E' => 0b00001,
        'A' => 0b00011,
        ' ' => 0b00100,
        'S' => 0b00101,
        'I' => 0b00110,
        'U' => 0b00111,
        'D' => 0b01001,
        'R' => 0b01010,
        'N' => 0b01100,
        'F' => 0b01101,
        'C' => 0b01110,
        'K' => 0b01111,
        'T' => 0b10000,
        'Z' => 0b10001,
        'L' => 0b10010,
        'W' => 0b10011,
        'H' => 0b10100,
        'Y' => 0b10101,
        'P' => 0b10110,
        'Q' => 0b10111,
        'O' => 0b11000,
        'B' => 0b11001,
        'G' => 0b11010,
        'M' => 0b11100,
        'X' => 0b11101,
        'V' => 0b11110,
        _ => return None,
    })
}

fn baudot_char(code: u8) -> Option<char> {
    Some(match code & 0b11111 {
        0b00001 => 'E',
        0b00011 => 'A',
        0b00100 => ' ',
        0b00101 => 'S',
        0b00110 => 'I',
        0b00111 => 'U',
        0b01001 => 'D',
        0b01010 => 'R',
        0b01100 => 'N',
        0b01101 => 'F',
        0b01110 => 'C',
        0b01111 => 'K',
        0b10000 => 'T',
        0b10001 => 'Z',
        0b10010 => 'L',
        0b10011 => 'W',
        0b10100 => 'H',
        0b10101 => 'Y',
        0b10110 => 'P',
        0b10111 => 'Q',
        0b11000 => 'O',
        0b11001 => 'B',
        0b11010 => 'G',
        0b11100 => 'M',
        0b11101 => 'X',
        0b11110 => 'V',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn morse_roundtrip() {
        let wave = encode_morse("CQ DE SDR", 2_000.0, 20.0);
        assert_eq!(decode_morse_envelope(&wave).as_deref(), Ok("CQ DE SDR"));
    }

    #[test]
    fn morse_silence_is_an_error() {
        assert!(decode_morse_envelope(&[0.0; 100]).is_err());
    }

    #[test]
    fn rtty_roundtrip() {
        assert_eq!(
            decode_rtty_bits(&encode_rtty_bits("SDR")).as_deref(),
            Ok("SDR")
        );
    }

    #[test]
    fn aprs_roundtrip() {
        let frame = encode_ax25("SDR", "APRS", ">hi");
        assert_eq!(decode_ax25(&frame).as_deref(), Ok("SDR>APRS:>hi"));
    }

    #[test]
    fn aprs_rejects_a_bad_fcs() {
        let mut frame = encode_ax25("SDR", "APRS", ">hi");
        if let Some(byte) = frame.last_mut() {
            *byte ^= 0xff;
        }
        assert!(decode_ax25(&frame).is_err());
    }
}
