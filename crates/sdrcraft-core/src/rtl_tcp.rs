//! Pure-Rust `rtl_tcp` IQ-over-TCP client (protocol v1, `RTL0` header).
//!
//! Connects to an `rtl_tcp` server (e.g. `127.0.0.1:1234`), validates the
//! 12-byte dongle header, issues center-frequency / sample-rate / gain / PPM
//! commands, and reads interleaved unsigned 8-bit I/Q bytes into `(f64, f64)`
//! complex samples in `[-1.0, 1.0]`.

use crate::creds::with_timeout_secs;
use crate::error::DeviceError;
use crate::sdr::Gain;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Magic bytes at the start of every `rtl_tcp` handshake.
pub const RTL0_MAGIC: &[u8; 4] = b"RTL0";

/// Parsed 12-byte `rtl_tcp` dongle info header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RtlTcpHeader {
    pub tuner_type: u32,
    pub gain_count: u32,
}

impl RtlTcpHeader {
    pub fn parse(buf: &[u8; 12]) -> Result<Self, DeviceError> {
        if &buf[0..4] != RTL0_MAGIC {
            return Err(DeviceError::Protocol(format!(
                "rtl_tcp bad magic: expected RTL0, got {:?}",
                &buf[0..4]
            )));
        }
        let tuner_type = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let gain_count = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
        Ok(Self {
            tuner_type,
            gain_count,
        })
    }

    pub fn tuner_name(&self) -> &'static str {
        match self.tuner_type {
            1 => "E4000",
            2 => "FC0012",
            3 => "FC0013",
            4 => "FC2580",
            5 => "R820T",
            6 => "R828D",
            _ => "Unknown",
        }
    }
}

/// `rtl_tcp` 5-byte big-endian wire command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RtlTcpCmd {
    SetFreq = 0x01,
    SetSampleRate = 0x02,
    SetGainMode = 0x03,
    SetGain = 0x04,
    SetFreqCorrection = 0x05,
}

pub fn encode_command(cmd: RtlTcpCmd, param: u32) -> [u8; 5] {
    let bytes = param.to_be_bytes();
    [cmd as u8, bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// Decode interleaved unsigned 8-bit I/Q pairs into normalized `(f64, f64)` samples.
pub fn decode_u8_iq(raw: &[u8]) -> Vec<(f64, f64)> {
    let pairs = raw.len() / 2;
    let mut out = Vec::with_capacity(pairs);
    for chunk in raw.as_chunks::<2>().0 {
        let i = (chunk[0] as f64 - 127.5) / 127.5;
        let q = (chunk[1] as f64 - 127.5) / 127.5;
        out.push((i, q));
    }
    out
}

/// Connect to `host:port`, apply tuning parameters, and capture `sample_count` complex IQ samples.
pub async fn capture_iq(
    host: &str,
    port: u16,
    center_freq_hz: f64,
    sample_rate_hz: f64,
    gain: &Gain,
    ppm: i32,
    sample_count: usize,
) -> Result<(RtlTcpHeader, Vec<(f64, f64)>), DeviceError> {
    if sample_count == 0 {
        return Err(DeviceError::InvalidInput(
            "sample_count must be positive".to_string(),
        ));
    }
    let addr = format!("{}:{}", host.trim(), port);
    let mut stream = with_timeout_secs("rtl_tcp.connect", 5, TcpStream::connect(&addr))
        .await?
        .map_err(|e| DeviceError::Offline(format!("rtl_tcp connect {addr}: {e}")))?;

    let mut hdr_buf = [0u8; 12];
    with_timeout_secs("rtl_tcp.header", 5, stream.read_exact(&mut hdr_buf))
        .await?
        .map_err(|e| DeviceError::Protocol(format!("rtl_tcp header read: {e}")))?;
    let header = RtlTcpHeader::parse(&hdr_buf)?;

    let mut cmds = Vec::with_capacity(5 * 5);
    cmds.extend_from_slice(&encode_command(
        RtlTcpCmd::SetSampleRate,
        sample_rate_hz.round().clamp(1.0, u32::MAX as f64) as u32,
    ));
    cmds.extend_from_slice(&encode_command(
        RtlTcpCmd::SetFreq,
        center_freq_hz.round().clamp(1.0, u32::MAX as f64) as u32,
    ));
    match gain {
        Gain::Auto => {
            cmds.extend_from_slice(&encode_command(RtlTcpCmd::SetGainMode, 0));
        }
        Gain::Db(db) => {
            cmds.extend_from_slice(&encode_command(RtlTcpCmd::SetGainMode, 1));
            let tenths = (db * 10.0).round().clamp(0.0, 1000.0) as u32;
            cmds.extend_from_slice(&encode_command(RtlTcpCmd::SetGain, tenths));
        }
    }
    if ppm != 0 {
        cmds.extend_from_slice(&encode_command(RtlTcpCmd::SetFreqCorrection, ppm as u32));
    }

    with_timeout_secs("rtl_tcp.commands", 5, stream.write_all(&cmds))
        .await?
        .map_err(|e| DeviceError::Protocol(format!("rtl_tcp command write: {e}")))?;

    let mut raw = vec![0u8; sample_count * 2];
    with_timeout_secs("rtl_tcp.iq_read", 5, stream.read_exact(&mut raw))
        .await?
        .map_err(|e| DeviceError::Protocol(format!("rtl_tcp iq read: {e}")))?;

    Ok((header, decode_u8_iq(&raw)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn header_parse_valid_and_invalid() {
        let mut raw = [0u8; 12];
        raw[0..4].copy_from_slice(b"RTL0");
        raw[4..8].copy_from_slice(&5u32.to_be_bytes());
        raw[8..12].copy_from_slice(&29u32.to_be_bytes());
        let hdr = RtlTcpHeader::parse(&raw);
        assert!(hdr.is_ok());
        if let Ok(h) = hdr {
            assert_eq!(h.tuner_type, 5);
            assert_eq!(h.gain_count, 29);
            assert_eq!(h.tuner_name(), "R820T");
        }

        let bad = [0u8; 12];
        assert!(RtlTcpHeader::parse(&bad).is_err());
    }

    #[test]
    fn command_encoding_is_big_endian() {
        let pkt = encode_command(RtlTcpCmd::SetFreq, 100_000_000);
        assert_eq!(pkt, [0x01, 0x05, 0xF5, 0xE1, 0x00]);
    }

    #[test]
    fn u8_iq_decoding_centers_near_zero() {
        let samples = decode_u8_iq(&[0, 255, 127, 128]);
        assert_eq!(samples.len(), 2);
        assert!((samples[0].0 + 1.0).abs() < 1e-6);
        assert!((samples[0].1 - 1.0).abs() < 1e-6);
        assert!(samples[1].0.abs() < 0.01);
        assert!(samples[1].1.abs() < 0.01);
    }

    #[tokio::test]
    async fn capture_iq_reads_mock_rtl_tcp_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await;
        assert!(listener.is_ok());
        let Ok(listener) = listener else { return };
        let Ok(addr) = listener.local_addr() else {
            return;
        };

        let server = tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut hdr = [0u8; 12];
                hdr[0..4].copy_from_slice(b"RTL0");
                hdr[4..8].copy_from_slice(&5u32.to_be_bytes());
                hdr[8..12].copy_from_slice(&29u32.to_be_bytes());
                let _ = sock.write_all(&hdr).await;
                let mut cmd_buf = [0u8; 15];
                let _ = sock.read_exact(&mut cmd_buf).await;
                let iq = vec![128u8; 64];
                let _ = sock.write_all(&iq).await;
            }
        });

        let res = capture_iq(
            "127.0.0.1",
            addr.port(),
            100_000_000.0,
            2_048_000.0,
            &Gain::Auto,
            0,
            32,
        )
        .await;
        assert!(res.is_ok());
        if let Ok((hdr, samples)) = res {
            assert_eq!(hdr.tuner_name(), "R820T");
            assert_eq!(samples.len(), 32);
        }
        let _ = server.await;
    }
}
