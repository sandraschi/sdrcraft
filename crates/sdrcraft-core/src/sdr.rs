//! L1 SDR domain core, ported from the legacy wire essence:
//! `capture.py` (config + validation ranges), `mock_capture.py` +
//! `mock_iq.py` (deterministic synthetic IQ), `processor.py` (2048-pt
//! Hamming FFT, peak-agnostic dB spectrum, 100-line waterfall),
//! `gnuradio_client.py` (sidecar HTTP shapes), `handlers/device.py`
//! (scan planner, gain/preset semantics), `handlers/state.py` (constants).
//!
//! Direct-USB access (`pyrtlsdr` equivalent) intentionally lands in Phase 3
//! (`rusb` crate): until then the hardware paths are `rtl_tcp` (TCP IQ) and
//! the GNU Radio sidecar (HTTP), plus the deterministic Demo oracle.

use crate::creds::with_timeout_secs;
use crate::error::DeviceError;
use crate::import::{LegacyConfig, MockSetting};
use crate::stations;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::time::Duration;

/// Lowest tune for an RTL-SDR Blog v4 (direct sampling / built-in HF).
/// A plain R820T2 stick still starts at 24 MHz. Longwave is below this.
pub const MIN_FREQ_MHZ: f64 = 0.5;
pub const MAX_FREQ_MHZ: f64 = 1766.0;
/// Legacy `scan_frequencies` step cap (`max_steps = 100`).
pub const MAX_SCAN_STEPS: usize = 100;
/// Scan dwell per step, verbatim (`asyncio.sleep(0.1)`).
pub const SCAN_DWELL: Duration = Duration::from_millis(100);
/// FFT + sample defaults, verbatim (`capture.py` / `processor.py`).
pub const FFT_SIZE: usize = 2048;
pub const DEFAULT_SAMPLE_RATE: f64 = 2_048_000.0;
pub const DEFAULT_CENTER_FREQ_HZ: f64 = 227_000_000.0;
pub const DEFAULT_FREQ_CORRECTION_PPM: i32 = 60;
/// RTL-SDR R820T2 gain range in dB, verbatim (`set_gain`).
pub const GAIN_MAX_DB: f32 = 49.6;
pub const MAX_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;
pub const SIDECAR_TIMEOUT_SECS: u64 = 10;

/// Demod modes + IQ sources, verbatim (`SUPPORTED_MODES/SOURCES`).
pub const DEMOD_MODES: &[&str] = &["fm", "am", "usb", "lsb"];
pub const DEMOD_SOURCES: &[&str] = &["rtl_tcp", "hackrf"];

/// Gain selector: `auto` or a manual dB value.
#[derive(Debug, Clone, PartialEq)]
pub enum Gain {
    Auto,
    Db(f32),
}

impl Gain {
    pub fn parse(raw: &str) -> Result<Self, DeviceError> {
        let text = raw.trim();
        if text.eq_ignore_ascii_case("auto") {
            return Ok(Self::Auto);
        }
        match text.parse::<f32>() {
            Ok(v) if v.is_finite() && (0.0..=GAIN_MAX_DB).contains(&v) => Ok(Self::Db(v)),
            Ok(_) => Err(DeviceError::InvalidInput(format!(
                "Gain value out of range (0-{GAIN_MAX_DB} dB)"
            ))),
            Err(_) => Err(DeviceError::InvalidInput(format!(
                "Invalid gain value: {raw}. Gain must be 'auto' or a numeric value in dB"
            ))),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Self::Auto => "auto".to_string(),
            Self::Db(v) => format!("{v}"),
        }
    }
}

/// In-memory device configuration (mirrors `SDRCapture` fields).
#[derive(Debug, Clone)]
pub struct DeviceState {
    pub device_index: u32,
    pub center_freq_hz: f64,
    pub sample_rate: f64,
    pub gain: Gain,
    pub freq_correction_ppm: i32,
    pub mock: MockSetting,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            device_index: 0,
            center_freq_hz: DEFAULT_CENTER_FREQ_HZ,
            sample_rate: DEFAULT_SAMPLE_RATE,
            gain: Gain::Auto,
            freq_correction_ppm: DEFAULT_FREQ_CORRECTION_PPM,
            mock: MockSetting::Auto,
        }
    }
}

pub fn validate_frequency_mhz(mhz: f64) -> Result<f64, DeviceError> {
    if !mhz.is_finite() || mhz > MAX_FREQ_MHZ {
        return Err(DeviceError::InvalidInput(format!(
            "Frequency must be between {MIN_FREQ_MHZ} MHz and {MAX_FREQ_MHZ} MHz"
        )));
    }
    if mhz < MIN_FREQ_MHZ {
        return Err(DeviceError::InvalidInput(format!(
            "Frequency {mhz} MHz is below {MIN_FREQ_MHZ} MHz. An RTL-SDR Blog v4 hears from 0.5 MHz. A Ham It Up v1.3 or Nano hears from 100 kHz: tune the stick to 125 MHz plus the station (BBC 198 kHz is 125.198 MHz, AM). The Plus model starts at 300 Hz, same 125 MHz shift."
        )));
    }
    Ok(mhz)
}

/// Resolve Demo-vs-hardware like `should_use_mock`: explicit setting wins;
/// `Auto` probes `rtl_tcp` (the only hardware signal available before the
/// Phase-3 USB driver) and falls back to Demo.
pub async fn effective_mock(setting: MockSetting, cfg: &LegacyConfig) -> bool {
    match setting {
        MockSetting::Enabled => true,
        MockSetting::Disabled => false,
        MockSetting::Auto => !rtl_tcp_probe(&cfg.rtl_host, cfg.rtl_port).await,
    }
}

/// TCP reachability probe for the `rtl_tcp` IQ stream (connect only —
/// full IQ framing lands with the Phase-3 streaming work).
pub async fn rtl_tcp_probe(host: &str, port: u16) -> bool {
    let target = format!("{}:{}", host.trim(), port);
    with_timeout_secs("rtl_tcp.probe", 5, tokio::net::TcpStream::connect(target))
        .await
        .is_ok_and(|r| r.is_ok())
}

/// Longwave preset keys, verbatim (`LONGWAVE_PRESETS` + handler notes).
pub struct Preset {
    pub key: &'static str,
    pub name: &'static str,
    pub frequency_hz: f64,
    pub note: &'static str,
}

pub static LW_PRESETS: &[Preset] = &[
    Preset {
        key: "orf_longwave",
        name: "ORF Longwave",
        frequency_hz: 198000.0,
        note: "ORF longwave has been broadcasting since 1962. Best reception in evenings when ionospheric conditions enhance longwave signals.",
    },
    Preset {
        key: "bbc_radio4",
        name: "BBC Radio 4",
        frequency_hz: 198000.0,
        note: "BBC longwave service began in 1978. Excellent for news and current affairs; the longwave signal is remarkably stable.",
    },
    Preset {
        key: "france_inter",
        name: "France Inter",
        frequency_hz: 162000.0,
        note: "France Inter evolved from Radiodiffusion Francaise, maintaining longwave service since 1963.",
    },
    Preset {
        key: "rtl_luxembourg",
        name: "RTL Luxembourg",
        frequency_hz: 234000.0,
        note: "RTL began longwave broadcasting in 1955, pioneering commercial radio in Europe.",
    },
];

/// Verbatim `tune_preset` resolution: exact callsign → name/callsign
/// substring → longwave preset key. Returns (display name, freq Hz, note).
pub fn resolve_preset(preset_name: &str) -> Result<(String, f64, String), DeviceError> {
    let want = preset_name.trim().to_ascii_lowercase();
    if want.is_empty() {
        return Err(DeviceError::InvalidInput(
            "preset_name is required for tune_preset".to_string(),
        ));
    }
    let upper = preset_name.trim().to_ascii_uppercase();
    if let Some(s) = stations::lookup_callsign(&upper) {
        return Ok((
            s.name.to_string(),
            s.frequency_hz,
            s.description.to_string(),
        ));
    }
    let sub: Vec<&stations::Station> = stations::STATIONS
        .iter()
        .filter(|s| {
            s.name.to_ascii_lowercase().contains(&want)
                || s.callsign.to_ascii_lowercase().contains(&want)
        })
        .collect();
    if let Some(s) = sub.first() {
        return Ok((
            s.name.to_string(),
            s.frequency_hz,
            s.description.to_string(),
        ));
    }
    let key = want.replace(' ', "_");
    if let Some(p) = LW_PRESETS.iter().find(|p| p.key == key) {
        return Ok((p.name.to_string(), p.frequency_hz, p.note.to_string()));
    }
    let mut available: Vec<String> = stations::by_band(stations::Band::LW)
        .iter()
        .map(|s| s.callsign.to_string())
        .collect();
    for p in LW_PRESETS {
        available.push(p.key.to_string());
    }
    Err(DeviceError::InvalidInput(format!(
        "Preset '{preset_name}' not found. Available: {}",
        available.join(", ")
    )))
}

/// Scan planner, verbatim (`scan_frequencies` validation + 100-step cap).
/// Returns the center frequencies (MHz) to dwell on.
pub fn scan_plan(start_mhz: f64, end_mhz: f64, step_mhz: f64) -> Result<Vec<f64>, DeviceError> {
    if !start_mhz.is_finite()
        || !end_mhz.is_finite()
        || start_mhz < MIN_FREQ_MHZ
        || end_mhz > MAX_FREQ_MHZ
    {
        return Err(DeviceError::InvalidInput(format!(
            "Frequency range {start_mhz:.1}-{end_mhz:.1} MHz exceeds RTL-SDR limits ({MIN_FREQ_MHZ}-{MAX_FREQ_MHZ} MHz)"
        )));
    }
    if start_mhz >= end_mhz {
        return Err(DeviceError::InvalidInput(format!(
            "Start frequency ({start_mhz:.1} MHz) must be less than end frequency ({end_mhz:.1} MHz)"
        )));
    }
    if !step_mhz.is_finite() || step_mhz <= 0.0 {
        return Err(DeviceError::InvalidInput(format!(
            "Step size must be positive (got {step_mhz})"
        )));
    }
    let step_count = ((end_mhz - start_mhz) / step_mhz) as usize + 1;
    if step_count > MAX_SCAN_STEPS {
        return Err(DeviceError::InvalidInput(format!(
            "Scan would take {step_count} steps (max {MAX_SCAN_STEPS}). Increase step_size or narrow the range."
        )));
    }
    let mut freqs = Vec::with_capacity(step_count);
    let mut current = start_mhz;
    while current <= end_mhz {
        freqs.push(current);
        current += step_mhz;
    }
    Ok(freqs)
}

/// Deterministic synthetic IQ, verbatim (`mock_iq.py`): the four demo tones
/// plus the 50 kHz sinusoidal drift on the 300 kHz tone. `xorshift64*` +
/// Box-Muller replaces `numpy.random.default_rng(42)` with no new crates.
pub struct MockIq {
    pub sample_rate: f64,
    pub center_freq: f64,
    rng_state: u64,
    time_offset: f64,
}

const MOCK_TONES: &[(f64, f64)] = &[
    (200_000.0, 0.30),
    (-150_000.0, 0.15),
    (500_000.0, 0.08),
    (300_000.0, 0.20),
];

impl MockIq {
    pub fn new(sample_rate: f64, center_freq: f64, seed: u64) -> Self {
        Self {
            sample_rate,
            center_freq,
            rng_state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
            time_offset: 0.0,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform in (0, 1] — never zero, so `ln` below cannot diverge.
    fn next_unit_open(&mut self) -> f64 {
        ((self.next_u64() >> 11) + 1) as f64 / 9007199254740992.0
    }

    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_unit_open();
        let u2 = self.next_unit_open();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    /// Generate `n` complex samples (interleaved I/Q as `(re, im)` pairs).
    pub fn generate(&mut self, n: usize) -> Vec<(f64, f64)> {
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            let t = self.time_offset + k as f64 / self.sample_rate;
            let mut re = 0.01 * self.next_normal();
            let mut im = 0.01 * self.next_normal();
            for (offset_hz, amplitude) in MOCK_TONES {
                let mut drift = 0.0;
                if *offset_hz == 300_000.0 {
                    drift = 50_000.0 * (2.0 * std::f64::consts::PI * 0.2 * t).sin();
                }
                let phase = 2.0 * std::f64::consts::PI * (*offset_hz + drift) * t;
                re += amplitude * phase.cos();
                im += amplitude * phase.sin();
            }
            out.push((re, im));
        }
        self.time_offset += n as f64 / self.sample_rate;
        out
    }
}

/// 2048-pt Hamming-windowed power spectrum, verbatim (`processor.py`):
/// most-recent window, FFT, `fftshift`, `20*log10(|x|+1e-12)`, centered
/// frequency axis. Naive DFT (Phase 3 upgrades to `realfft`); short input
/// yields empty vectors exactly like the legacy guard.
pub fn compute_spectrum(samples: &[(f64, f64)], sample_rate: f64) -> (Vec<f64>, Vec<f64>) {
    if samples.len() < FFT_SIZE || !sample_rate.is_finite() || sample_rate <= 0.0 {
        return (Vec::new(), Vec::new());
    }
    let window: Vec<(f64, f64)> = samples[samples.len() - FFT_SIZE..].to_vec();
    // Hamming window + DFT via twiddle recurrence (no trig in the inner loop).
    // Phase 3 upgrades this O(n^2) pass to `realfft`; the window, shift and
    // dB scaling already match `processor.py` exactly.
    let mut mags = vec![0.0f64; FFT_SIZE];
    for (k, mag) in mags.iter_mut().enumerate() {
        let angle = -2.0 * std::f64::consts::PI * k as f64 / FFT_SIZE as f64;
        let (sin_a, cos_a) = angle.sin_cos();
        let mut cur_re = 1.0;
        let mut cur_im = 0.0;
        let mut acc_re = 0.0;
        let mut acc_im = 0.0;
        for (j, (re, im)) in window.iter().enumerate() {
            let w = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * j as f64 / FFT_SIZE as f64).cos();
            let (x_re, x_im) = (re * w, im * w);
            acc_re += x_re * cur_re - x_im * cur_im;
            acc_im += x_re * cur_im + x_im * cur_re;
            let next_re = cur_re * cos_a - cur_im * sin_a;
            cur_im = cur_re * sin_a + cur_im * cos_a;
            cur_re = next_re;
        }
        *mag = acc_re.hypot(acc_im);
    }
    // fftshift + dB.
    let mut power = vec![0.0f64; FFT_SIZE];
    for (i, mag) in mags.iter().enumerate() {
        let shifted = (i + FFT_SIZE / 2) % FFT_SIZE;
        power[shifted] = 20.0 * (mag + 1e-12).log10();
    }
    let step = sample_rate / FFT_SIZE as f64;
    let freqs: Vec<f64> = (0..FFT_SIZE)
        .map(|i| (i as f64 - FFT_SIZE as f64 / 2.0) * step)
        .collect();
    (freqs, power)
}

/// 100-line waterfall history, verbatim (`max_history = 100`).
pub struct Waterfall {
    lines: VecDeque<Vec<f64>>,
    pub max_history: usize,
}

impl Waterfall {
    pub fn new() -> Self {
        Self {
            lines: VecDeque::new(),
            max_history: 100,
        }
    }

    pub fn push(&mut self, power: Vec<f64>) {
        if power.is_empty() {
            return;
        }
        self.lines.push_back(power);
        while self.lines.len() > self.max_history {
            self.lines.pop_front();
        }
    }

    pub fn lines(&self) -> Vec<Vec<f64>> {
        self.lines.iter().cloned().collect()
    }

    pub fn latest(&self) -> Option<Vec<f64>> {
        self.lines.back().cloned()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

impl Default for Waterfall {
    fn default() -> Self {
        Self::new()
    }
}

/// GNU Radio sidecar configuration (verbatim `get_gnuradio_config`).
#[derive(Debug, Clone)]
pub struct GnuradioCfg {
    pub service_url: String,
    pub rtl_host: String,
    pub rtl_port: u16,
}

impl GnuradioCfg {
    pub fn resolve(
        service_url: Option<String>,
        rtl_host: Option<String>,
        rtl_port: Option<u16>,
    ) -> Self {
        let cfg = LegacyConfig::resolve();
        Self {
            service_url: service_url
                .map(|u| u.trim_end_matches('/').to_string())
                .filter(|u| !u.is_empty())
                .unwrap_or(cfg.gnuradio_url),
            rtl_host: rtl_host
                .filter(|h| !h.trim().is_empty())
                .unwrap_or(cfg.rtl_host),
            rtl_port: rtl_port.unwrap_or(cfg.rtl_port),
        }
    }
}

/// Async sidecar client — verbatim `GnuradioClient` shapes
/// (`GET /health|/status|/devices`, `POST /demod/start|/demod/stop`).
pub struct GnuradioClient {
    base_url: String,
}

impl GnuradioClient {
    pub fn new(cfg: &GnuradioCfg) -> Self {
        Self {
            base_url: cfg.service_url.clone(),
        }
    }

    fn http(&self) -> Result<reqwest::Client, DeviceError> {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(SIDECAR_TIMEOUT_SECS))
            .build()
            .map_err(|e| DeviceError::Protocol(format!("gnuradio http client: {e}")))
    }

    async fn get_json(&self, path: &str) -> Result<Value, DeviceError> {
        let http = self.http()?;
        let resp = with_timeout_secs(
            "gnuradio.get",
            SIDECAR_TIMEOUT_SECS,
            http.get(format!("{}{path}", self.base_url)).send(),
        )
        .await?
        .map_err(|e| DeviceError::Protocol(format!("gnuradio.get: {e}")))?;
        decode_capped("gnuradio.get", resp).await
    }

    pub async fn health(&self) -> Result<Value, DeviceError> {
        self.get_json("/health").await
    }

    pub async fn status(&self) -> Result<Value, DeviceError> {
        self.get_json("/status").await
    }

    pub async fn devices(&self) -> Result<Value, DeviceError> {
        self.get_json("/devices").await
    }

    pub async fn is_reachable(&self) -> bool {
        self.health().await.is_ok()
    }

    pub async fn stop_demod(&self) -> Result<Value, DeviceError> {
        let http = self.http()?;
        let resp = with_timeout_secs(
            "gnuradio.stop",
            SIDECAR_TIMEOUT_SECS,
            http.post(format!("{}/demod/stop", self.base_url)).send(),
        )
        .await?
        .map_err(|e| DeviceError::Protocol(format!("gnuradio.stop: {e}")))?;
        decode_capped("gnuradio.stop", resp).await
    }

    /// Verbatim `start_demod` validation + payload
    /// (`frequency_hz`, `sample_rate = 2.0e6`, `gain = 20.0`).
    pub async fn start_demod(
        &self,
        mode: &str,
        source: &str,
        frequency_mhz: f64,
        rtl_host: Option<String>,
        rtl_port: Option<u16>,
        gain: f64,
    ) -> Result<Value, DeviceError> {
        validate_frequency_mhz(frequency_mhz)?;
        let mode = mode.trim().to_ascii_lowercase();
        if !DEMOD_MODES.contains(&mode.as_str()) {
            return Err(DeviceError::InvalidInput(format!(
                "Unsupported mode: {mode}. Supported: {}",
                DEMOD_MODES.join(", ")
            )));
        }
        let source = source.trim().to_ascii_lowercase();
        if !DEMOD_SOURCES.contains(&source.as_str()) {
            return Err(DeviceError::InvalidInput(format!(
                "Unsupported source: {source}. Supported: {}",
                DEMOD_SOURCES.join(", ")
            )));
        }
        if !gain.is_finite() {
            return Err(DeviceError::InvalidInput("gain must be finite".to_string()));
        }
        let cfg = GnuradioCfg::resolve(None, rtl_host, rtl_port);
        let payload = json!({
            "mode": mode,
            "source": source,
            "frequency_hz": frequency_mhz * 1e6,
            "rtl_tcp_host": cfg.rtl_host,
            "rtl_tcp_port": cfg.rtl_port,
            "sample_rate": 2.0e6,
            "gain": gain,
        });
        let http = self.http()?;
        let resp = with_timeout_secs(
            "gnuradio.start",
            SIDECAR_TIMEOUT_SECS,
            http.post(format!("{}/demod/start", self.base_url))
                .json(&payload)
                .send(),
        )
        .await?
        .map_err(|e| DeviceError::Protocol(format!("gnuradio.start: {e}")))?;
        decode_capped("gnuradio.start", resp).await
    }
}

async fn decode_capped(op: &str, resp: reqwest::Response) -> Result<Value, DeviceError> {
    if !resp.status().is_success() {
        return Err(DeviceError::Protocol(format!(
            "{op} HTTP {}",
            resp.status()
        )));
    }
    let text = with_timeout_secs(op, SIDECAR_TIMEOUT_SECS, resp.text())
        .await?
        .map_err(|e| DeviceError::Protocol(format!("{op} body: {e}")))?;
    if text.len() > MAX_PAYLOAD_BYTES {
        return Err(DeviceError::Protocol(format!(
            "{op} payload {} bytes exceeds cap",
            text.len()
        )));
    }
    serde_json::from_str(&text).map_err(|e| DeviceError::Protocol(format!("{op} decode: {e}")))
}

pub fn device_info_json(state: &DeviceState, available: bool, mock: bool) -> Value {
    json!({
        "device_index": state.device_index,
        "center_freq": state.center_freq_hz,
        "sample_rate": state.sample_rate,
        "gain": state.gain.as_str(),
        "freq_correction": state.freq_correction_ppm,
        "available": available,
        "mock": mock,
    })
}

/// Process-wide singletons mirroring the legacy handler globals
/// (`_sdr_capture`, `_sdr_processor`). Phase 4 replaces ad-hoc locking with
/// the `LoadState` latch + `mpsc` job channel on the UI side; the L1 API
/// shape stays identical.
use std::sync::{Mutex, MutexGuard, OnceLock};

static DEVICE: OnceLock<Mutex<DeviceState>> = OnceLock::new();
static WATERFALL_STATE: OnceLock<Mutex<Waterfall>> = OnceLock::new();
static MOCK_GEN: OnceLock<Mutex<MockIq>> = OnceLock::new();

fn poisoned(what: &str) -> DeviceError {
    DeviceError::Protocol(format!("{what} state lock poisoned"))
}

pub fn device() -> Result<MutexGuard<'static, DeviceState>, DeviceError> {
    DEVICE
        .get_or_init(|| Mutex::new(DeviceState::default()))
        .lock()
        .map_err(|_| poisoned("device"))
}

pub fn waterfall() -> Result<MutexGuard<'static, Waterfall>, DeviceError> {
    WATERFALL_STATE
        .get_or_init(|| Mutex::new(Waterfall::new()))
        .lock()
        .map_err(|_| poisoned("waterfall"))
}

pub fn mock_iq() -> Result<MutexGuard<'static, MockIq>, DeviceError> {
    MOCK_GEN
        .get_or_init(|| Mutex::new(MockIq::new(DEFAULT_SAMPLE_RATE, DEFAULT_CENTER_FREQ_HZ, 42)))
        .lock()
        .map_err(|_| poisoned("mock-iq"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_range_matches_rtlsdr() {
        assert!(validate_frequency_mhz(101.5).is_ok());
        assert!(validate_frequency_mhz(0.5).is_ok());
        assert!(validate_frequency_mhz(0.909).is_ok());
        assert!(validate_frequency_mhz(23.9).is_ok());
        assert!(validate_frequency_mhz(0.1).is_err());
        assert!(validate_frequency_mhz(1766.1).is_err());
        assert!(validate_frequency_mhz(f64::NAN).is_err());
    }

    #[test]
    fn gain_truth_table() {
        assert_eq!(Gain::parse("auto"), Ok(Gain::Auto));
        assert_eq!(Gain::parse("AUTO"), Ok(Gain::Auto));
        assert_eq!(Gain::parse("20"), Ok(Gain::Db(20.0)));
        assert!(Gain::parse("99").is_err());
        assert!(Gain::parse("-1").is_err());
        assert!(Gain::parse("loud").is_err());
    }

    #[test]
    fn presets_resolve() {
        let bbc = resolve_preset("BBC LW");
        assert!(bbc.is_ok());
        if let Ok((name, freq, _)) = bbc {
            assert_eq!(name, "BBC Radio 4");
            assert_eq!(freq, 198000.0);
        }
        let orf = resolve_preset("orf_longwave");
        assert!(orf.is_ok());
        if let Ok((_, freq, _)) = orf {
            assert_eq!(freq, 198000.0);
        }
        let fi = resolve_preset("france inter");
        assert!(fi.is_ok());
        if let Ok((_, freq, _)) = fi {
            assert_eq!(freq, 162000.0);
        }
        assert!(resolve_preset("no such station").is_err());
        assert!(resolve_preset("").is_err());
    }

    #[test]
    fn scan_plan_caps_at_100_steps() {
        assert_eq!(scan_plan(88.0, 89.0, 1.0).map(|v| v.len()), Ok(2));
        assert!(scan_plan(88.0, 200.0, 1.0).is_err());
        assert!(scan_plan(100.0, 90.0, 1.0).is_err());
        assert!(scan_plan(88.0, 89.0, 0.0).is_err());
        assert!(scan_plan(0.1, 0.4, 0.1).is_err());
    }

    #[test]
    fn mock_is_deterministic_per_seed() {
        let mut a = MockIq::new(DEFAULT_SAMPLE_RATE, DEFAULT_CENTER_FREQ_HZ, 42);
        let mut b = MockIq::new(DEFAULT_SAMPLE_RATE, DEFAULT_CENTER_FREQ_HZ, 42);
        assert_eq!(a.generate(64), b.generate(64));
    }

    #[test]
    fn mock_tone_visible_near_plus_200khz() {
        let mut gen = MockIq::new(DEFAULT_SAMPLE_RATE, DEFAULT_CENTER_FREQ_HZ, 42);
        let samples = gen.generate(FFT_SIZE * 2);
        let (freqs, power) = compute_spectrum(&samples, DEFAULT_SAMPLE_RATE);
        assert_eq!(freqs.len(), FFT_SIZE);
        let mut best = 0usize;
        for (i, p) in power.iter().enumerate() {
            if *p > power[best] {
                best = i;
            }
        }
        let peak_hz = freqs[best].abs();
        assert!(
            (peak_hz - 200_000.0).abs() < 5_000.0,
            "peak at {peak_hz} Hz, expected ~200 kHz demo tone"
        );
    }

    #[test]
    fn short_input_yields_empty_spectrum() {
        let (f, p) = compute_spectrum(&vec![(0.0, 0.0); 100], DEFAULT_SAMPLE_RATE);
        assert!(f.is_empty() && p.is_empty());
    }

    #[test]
    fn waterfall_caps_at_100_lines() {
        let mut wf = Waterfall::new();
        for _ in 0..150 {
            wf.push(vec![0.0; 8]);
        }
        assert_eq!(wf.lines().len(), 100);
        assert!(wf.latest().is_some());
        wf.clear();
        assert!(wf.latest().is_none());
    }

    #[test]
    fn demod_whitelists_reject_garbage() {
        assert!(DEMOD_MODES.contains(&"fm"));
        assert!(!DEMOD_MODES.contains(&"dapsk"));
        assert!(DEMOD_SOURCES.contains(&"rtl_tcp"));
    }
}
