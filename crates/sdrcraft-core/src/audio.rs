//! `cpal` audio relay + pure-Rust FM/AM IQ demodulator.
//!
//! Probes the host default audio output device via `cpal`, and provides
//! polar-discriminator FM and envelope AM demodulation from complex IQ samples.
//! Any active `cpal::Stream` is owned by a dedicated controller thread so the
//! public handle stays `Send + Sync` with `#![forbid(unsafe_code)]`.

use crate::error::DeviceError;
use crate::sdr::MockIq;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde_json::{json, Value};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioMode {
    Fm,
    Am,
}

impl AudioMode {
    pub fn parse(raw: &str) -> Result<Self, DeviceError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "fm" | "wfm" | "nfm" => Ok(Self::Fm),
            "am" => Ok(Self::Am),
            other => Err(DeviceError::InvalidInput(format!(
                "Unsupported audio mode '{other}'. Supported: fm, am"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fm => "fm",
            Self::Am => "am",
        }
    }
}

/// Polar discriminator FM demodulator: instantaneous phase delta scaled to `[-1, 1]`.
pub fn demod_fm(samples: &[(f64, f64)]) -> Vec<f32> {
    if samples.len() < 2 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(samples.len() - 1);
    let mut prev = samples[0];
    for &(re, im) in &samples[1..] {
        let conj_re = re * prev.0 + im * prev.1;
        let conj_im = im * prev.0 - re * prev.1;
        let phase_delta = conj_im.atan2(conj_re);
        out.push((phase_delta / std::f64::consts::PI) as f32);
        prev = (re, im);
    }
    out
}

/// Envelope AM demodulator with DC-blocking mean removal.
pub fn demod_am(samples: &[(f64, f64)]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let mags: Vec<f64> = samples.iter().map(|(re, im)| re.hypot(*im)).collect();
    let mean = mags.iter().sum::<f64>() / mags.len() as f64;
    mags.into_iter()
        .map(|m| ((m - mean) as f32).clamp(-1.0, 1.0))
        .collect()
}

/// Simple boxcar decimator from IQ sample rate down to audio rate.
pub fn decimate_mono(audio: &[f32], factor: usize) -> Vec<f32> {
    if factor <= 1 || audio.is_empty() {
        return audio.to_vec();
    }
    audio
        .chunks(factor)
        .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
        .collect()
}

enum AudioCtrl {
    Stop,
}

struct AudioState {
    running: bool,
    mode: AudioMode,
    device_name: Option<String>,
    sample_rate: u32,
    ctrl: Option<Sender<AudioCtrl>>,
}

static AUDIO_STATE: OnceLock<Mutex<AudioState>> = OnceLock::new();

fn audio_lock() -> &'static Mutex<AudioState> {
    AUDIO_STATE.get_or_init(|| {
        Mutex::new(AudioState {
            running: false,
            mode: AudioMode::Fm,
            device_name: None,
            sample_rate: 48_000,
            ctrl: None,
        })
    })
}

pub fn probe_default_output() -> (Option<String>, u32) {
    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else {
        return (None, 48_000);
    };
    let name = device.name().ok();
    let rate = device
        .default_output_config()
        .map(|c| c.sample_rate().0)
        .unwrap_or(48_000);
    (name, rate)
}

pub fn status() -> Value {
    let (probed_name, probed_rate) = probe_default_output();
    let guard = match audio_lock().lock() {
        Ok(g) => g,
        Err(_) => {
            return json!({ "success": false, "status": "error", "error": "audio state lock poisoned" });
        }
    };
    json!({
        "success": true,
        "running": guard.running,
        "mode": guard.mode.as_str(),
        "device": guard.device_name.clone().or(probed_name),
        "sample_rate": if guard.running { guard.sample_rate } else { probed_rate },
        "available": guard.device_name.is_some() || probed_rate > 0,
    })
}

pub fn start(mode_str: &str) -> Result<Value, DeviceError> {
    let mode = AudioMode::parse(mode_str)?;
    let mut guard = audio_lock()
        .lock()
        .map_err(|_| DeviceError::Protocol("audio state lock poisoned".to_string()))?;
    if guard.running {
        return Ok(json!({
            "success": true,
            "running": true,
            "already_running": true,
            "mode": guard.mode.as_str(),
            "device": guard.device_name,
            "sample_rate": guard.sample_rate,
        }));
    }

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| DeviceError::Offline("no default cpal audio output device".to_string()))?;
    let device_name = device.name().ok();
    let config = device
        .default_output_config()
        .map_err(|e| DeviceError::Protocol(format!("cpal default_output_config: {e}")))?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let (tx, rx) = mpsc::channel::<AudioCtrl>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

    std::thread::Builder::new()
        .name("sdrcraft-cpal".to_string())
        .spawn(move || {
            let mut iq = MockIq::new(sample_rate as f64 * 4.0, 227_000_000.0, 42);
            let stream_res = device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    let frames = data.len() / channels.max(1);
                    let raw = iq.generate(frames * 4 + 1);
                    let audio = match mode {
                        AudioMode::Fm => decimate_mono(&demod_fm(&raw), 4),
                        AudioMode::Am => decimate_mono(&demod_am(&raw), 4),
                    };
                    for (i, frame) in data.chunks_mut(channels.max(1)).enumerate() {
                        let sample = audio.get(i).copied().unwrap_or(0.0) * 0.15;
                        for ch in frame {
                            *ch = sample;
                        }
                    }
                },
                |err| {
                    eprintln!("cpal stream error: {err}");
                },
                None,
            );
            let stream = match stream_res {
                Ok(s) => s,
                Err(e) => {
                    let _ = ready_tx.send(Err(format!("cpal build_output_stream: {e}")));
                    return;
                }
            };
            if let Err(e) = stream.play() {
                let _ = ready_tx.send(Err(format!("cpal stream.play: {e}")));
                return;
            }
            let _ = ready_tx.send(Ok(()));
            let _ = rx.recv();
            drop(stream);
        })
        .map_err(|e| DeviceError::Protocol(format!("spawn audio thread: {e}")))?;

    match ready_rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(DeviceError::Protocol(e)),
        Err(_) => {
            return Err(DeviceError::Protocol(
                "audio thread exited before ready".to_string(),
            ))
        }
    }

    guard.running = true;
    guard.mode = mode;
    guard.device_name = device_name.clone();
    guard.sample_rate = sample_rate;
    guard.ctrl = Some(tx);

    Ok(json!({
        "success": true,
        "running": true,
        "mode": mode.as_str(),
        "device": device_name,
        "sample_rate": sample_rate,
    }))
}

pub fn stop() -> Result<Value, DeviceError> {
    let mut guard = audio_lock()
        .lock()
        .map_err(|_| DeviceError::Protocol("audio state lock poisoned".to_string()))?;
    if let Some(tx) = guard.ctrl.take() {
        let _ = tx.send(AudioCtrl::Stop);
    }
    guard.running = false;
    Ok(json!({
        "success": true,
        "running": false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fm_demod_extracts_constant_tone_offset() {
        let mut iq = Vec::new();
        for k in 0..64 {
            let phase = 0.25 * k as f64;
            iq.push((phase.cos(), phase.sin()));
        }
        let audio = demod_fm(&iq);
        assert_eq!(audio.len(), 63);
        let expected = (0.25 / std::f64::consts::PI) as f32;
        assert!((audio[10] - expected).abs() < 1e-4);
    }

    #[test]
    fn am_demod_and_decimate_work() {
        let iq: Vec<(f64, f64)> = (0..16)
            .map(|k| {
                let env = if k % 2 == 0 { 1.0 } else { 0.5 };
                (env, 0.0)
            })
            .collect();
        let am = demod_am(&iq);
        assert_eq!(am.len(), 16);
        assert!((am[0] - 0.25).abs() < 1e-5);
        assert!((am[1] + 0.25).abs() < 1e-5);
        let dec = decimate_mono(&am, 2);
        assert_eq!(dec.len(), 8);
        assert!(dec[0].abs() < 1e-5);
    }

    #[test]
    fn audio_mode_rejects_unknown() {
        assert_eq!(AudioMode::parse("FM").ok(), Some(AudioMode::Fm));
        assert_eq!(AudioMode::parse("am").ok(), Some(AudioMode::Am));
        assert!(AudioMode::parse("tetra").is_err());
    }
}
