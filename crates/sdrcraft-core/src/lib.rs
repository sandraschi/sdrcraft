#![forbid(unsafe_code)]
//! L1 `sdrcraft-core`: SDR domain drivers + universal [`Command::dispatch`] bus.
//!
//! Phase-2: full 1:1 operation table from `docs/CONVERSION_DOSSIER.md` §3.
//! Device/spectrum/stations/online/gnuradio are real; direct-USB capture,
//! WebSocket streaming and audio relay are typed `Unsupported` with the
//! Phase-3 crate named (no silent stubs — the error says what and when).

pub mod bridge;
pub mod creds;
pub mod decode;
pub mod error;
pub mod import;
pub mod llm;
pub mod online;
pub mod sdr;
pub mod stations;

use error::DeviceError;
use serde_json::{json, Value};

/// Universal command bus: every CLI subcommand, MCP tool and (from Phase 4)
/// L3 `egui` button funnels through [`dispatch`]. Variants mirror the legacy
/// portmanteau `operation` strings 1:1 (see `docs/CONVERSION_DOSSIER.md`).
#[derive(Debug, Clone)]
pub enum Command {
    // -- device (sdr_device) --
    DeviceList,
    DeviceInit {
        index: u32,
    },
    DeviceStatus,
    DeviceHealth,
    DeviceSetFreq {
        mhz: f64,
    },
    DeviceSetGain {
        gain: String,
    },
    DeviceTunePreset {
        name: String,
    },
    DeviceScan {
        start_mhz: f64,
        end_mhz: f64,
        step_mhz: f64,
    },
    DeviceMockMode {
        enabled: Option<bool>,
    },
    // -- spectrum (sdr_spectrum) --
    SpectrumGet,
    SpectrumWaterfall,
    SpectrumWsStart {
        host: String,
        port: u16,
    },
    SpectrumWsStop,
    SpectrumWsStatus,
    SpectrumAudioStatus,
    // -- stations (sdr_stations) --
    StationsSearch {
        query: String,
        band: Option<String>,
        country: Option<String>,
    },
    StationsByBand {
        band: String,
    },
    StationsByCountry {
        country: String,
    },
    StationsSchedule {
        callsign: String,
        day: Option<String>,
    },
    StationsStats,
    // -- online (sdr_online) --
    OnlineSearch {
        query: String,
        country: Option<String>,
        language: Option<String>,
        tag: Option<String>,
        limit: u32,
    },
    OnlineSignalId {
        query: String,
    },
    // -- gnuradio (sdr_gnuradio) --
    GnuradioHealth,
    GnuradioStatus,
    GnuradioStart {
        mhz: f64,
        mode: String,
        source: String,
        host: Option<String>,
        port: Option<u16>,
        gain: f64,
    },
    GnuradioStop,
    GnuradioDevices {
        source: String,
    },
    // -- fleet kernel --
    Status,
    BridgeEvent {
        title: String,
        summary: Option<String>,
        urgency: Option<f64>,
    },
    BridgeSpeak {
        text: String,
    },
    BridgeWebhook {
        url: String,
        title: String,
        summary: Option<String>,
    },
    LlmChat {
        provider: Option<String>,
        model: Option<String>,
        prompt: String,
    },
    LlmModels {
        provider: Option<String>,
    },
    /// Morse, RTTY, and APRS on practice tapes. Not a live copy of the air.
    DecodeCopy,
}

fn ok(data: Value) -> Value {
    data
}

fn fail(e: DeviceError) -> Value {
    let kind = match &e {
        DeviceError::Timeout(_) => "timeout",
        DeviceError::Protocol(_) => "protocol",
        DeviceError::Offline(_) => "offline",
        DeviceError::InvalidInput(_) => "invalid_input",
        DeviceError::Auth(_) => "auth",
        DeviceError::Unsupported(_) => "unsupported",
    };
    json!({ "success": false, "status": "error", "kind": kind, "error": e.to_string() })
}

/// Execute one [`Command`] and return its JSON payload. Domain failures are
/// values (`success: false`), never panics.
pub async fn dispatch(cmd: Command) -> Value {
    match cmd {
        Command::Status => ok(json!({
            "success": true,
            "service": "sdrcraft-mcp",
            "stage": "phase-2 L1 domain core",
            "message": "sdrcraft-core alive",
        })),
        Command::DeviceList => ok(device_list().await),
        Command::DeviceInit { index } => ok(device_init(index)),
        Command::DeviceStatus => ok(device_status()),
        Command::DeviceHealth => ok(device_health().await),
        Command::DeviceSetFreq { mhz } => match sdr::validate_frequency_mhz(mhz) {
            Ok(v) => {
                if let Ok(mut dev) = sdr::device() {
                    dev.center_freq_hz = v * 1e6;
                    let path = if v < 24.0 {
                        " RTL-SDR Blog v4 direct sampling. A plain R820T2 stick still starts at 24 MHz."
                    } else {
                        ""
                    };
                    let info =
                        sdr::device_info_json(&dev, true, dev.mock == import::MockSetting::Enabled);
                    ok(
                        json!({ "success": true, "message": format!("Frequency set to {v:.3} MHz.{path}"), "device": info }),
                    )
                } else {
                    fail(DeviceError::Protocol(
                        "device state lock poisoned".to_string(),
                    ))
                }
            }
            Err(e) => fail(e),
        },
        Command::DeviceSetGain { gain } => match sdr::Gain::parse(&gain) {
            Ok(parsed) => {
                if let Ok(mut dev) = sdr::device() {
                    dev.gain = parsed.clone();
                    ok(
                        json!({ "success": true, "gain": parsed.as_str(), "message": format!("Gain set to {}", parsed.as_str()) }),
                    )
                } else {
                    fail(DeviceError::Protocol(
                        "device state lock poisoned".to_string(),
                    ))
                }
            }
            Err(e) => fail(e),
        },
        Command::DeviceTunePreset { name } => match sdr::resolve_preset(&name) {
            Ok((display, freq_hz, note)) => {
                if let Ok(mut dev) = sdr::device() {
                    dev.center_freq_hz = freq_hz;
                }
                let front_end = if freq_hz >= 24_000_000.0 {
                    "r820t2"
                } else if freq_hz >= 500_000.0 {
                    "rtl_blog_v4"
                } else {
                    "hf_receiver"
                };
                ok(
                    json!({ "success": true, "preset": display, "frequency_mhz": freq_hz / 1e6, "note": note, "front_end": front_end }),
                )
            }
            Err(e) => fail(e),
        },
        Command::DeviceScan {
            start_mhz,
            end_mhz,
            step_mhz,
        } => ok(device_scan(start_mhz, end_mhz, step_mhz).await),
        Command::DeviceMockMode { enabled } => ok(device_mock_mode(enabled).await),
        Command::SpectrumGet => ok(spectrum_get().await),
        Command::SpectrumWaterfall => match sdr::waterfall() {
            Ok(wf) => {
                ok(json!({ "success": true, "waterfall": wf.lines(), "lines": wf.lines().len() }))
            }
            Err(e) => fail(e),
        },
        Command::SpectrumWsStart { .. } | Command::SpectrumWsStop | Command::SpectrumWsStatus => {
            fail(DeviceError::Unsupported(
                "websocket streaming moves to tokio-tungstenite in Phase 3".to_string(),
            ))
        }
        Command::SpectrumAudioStatus => fail(DeviceError::Unsupported(
            "audio relay status moves to the cpal relay in Phase 3".to_string(),
        )),
        Command::StationsSearch {
            query,
            band,
            country,
        } => {
            let parsed_band = band.as_deref().and_then(stations::Band::parse);
            if band.is_some() && parsed_band.is_none() {
                return fail(DeviceError::InvalidInput(format!(
                    "Unknown band '{}'. Valid: LW, MW, SW, VHF, UHF",
                    band.unwrap_or_default()
                )));
            }
            let hits = stations::search(&query, parsed_band, country.as_deref());
            ok(json!({
                "success": true,
                "status": if hits.is_empty() { "no_results" } else { "found" },
                "total_results": hits.len(),
                "stations": hits.iter().map(|s| stations::station_json(s)).collect::<Vec<_>>(),
            }))
        }
        Command::StationsByBand { band } => match stations::Band::parse(&band) {
            Some(b) => {
                let hits = stations::by_band(b);
                ok(json!({
                    "success": true,
                    "status": if hits.is_empty() { "no_stations" } else { "success" },
                    "band": b.as_str(),
                    "range": b.range(),
                    "total_stations": hits.len(),
                    "stations": hits.iter().map(|s| stations::station_json(s)).collect::<Vec<_>>(),
                }))
            }
            None => fail(DeviceError::InvalidInput(format!(
                "'{band}' is not a valid band. Available bands: LW, MW, SW, VHF, UHF"
            ))),
        },
        Command::StationsByCountry { country } => {
            let hits = stations::by_country(&country);
            ok(json!({
                "success": true,
                "status": if hits.is_empty() { "no_stations" } else { "success" },
                "country": country,
                "total_stations": hits.len(),
                "stations": hits.iter().map(|s| stations::station_json(s)).collect::<Vec<_>>(),
            }))
        }
        Command::StationsSchedule { callsign, day } => {
            let Some(station) = stations::lookup_callsign(callsign.trim()) else {
                return fail(DeviceError::InvalidInput(format!(
                    "Station '{callsign}' not found in database"
                )));
            };
            let sched = stations::schedule(callsign.trim(), day.as_deref());
            let current = stations::current_program_now(callsign.trim());
            ok(json!({
                "success": true,
                "station": stations::station_json(station),
                "current_program": current.map(|p| p.name),
                "programs": sched.iter().map(|p| json!({
                    "name": p.name, "description": p.description,
                    "start": format!("{:02}:{:02}", p.start.0, p.start.1),
                    "end": format!("{:02}:{:02}", p.end.0, p.end.1),
                    "days": p.days, "language": p.language,
                })).collect::<Vec<_>>(),
            }))
        }
        Command::StationsStats => ok(stations::stats_json()),
        Command::OnlineSearch {
            query,
            country,
            language,
            tag,
            limit,
        } => {
            match online::search_radio_browser(
                &query,
                limit,
                "name",
                country.as_deref(),
                language.as_deref(),
                tag.as_deref(),
            )
            .await
            {
                Ok(hits) => ok(json!({
                    "success": true,
                    "total_results": hits.len(),
                    "stations": hits.iter().map(online::station_json).collect::<Vec<_>>(),
                })),
                Err(e) => fail(e),
            }
        }
        Command::OnlineSignalId { query } => match online::get_signal_info(&query).await {
            Ok(info) => ok(json!({ "success": true, "signal": info })),
            Err(e) => fail(e),
        },
        Command::GnuradioHealth => {
            let cfg = sdr::GnuradioCfg::resolve(None, None, None);
            let client = sdr::GnuradioClient::new(&cfg);
            ok(json!({
                "success": true,
                "reachable": client.is_reachable().await,
                "service_url": cfg.service_url,
            }))
        }
        Command::GnuradioStatus => {
            let cfg = sdr::GnuradioCfg::resolve(None, None, None);
            let client = sdr::GnuradioClient::new(&cfg);
            match client.status().await {
                Ok(demod) => ok(json!({ "success": true, "demod": demod })),
                Err(e) => fail(e),
            }
        }
        Command::GnuradioStart {
            mhz,
            mode,
            source,
            host,
            port,
            gain,
        } => {
            let cfg = sdr::GnuradioCfg::resolve(None, None, None);
            let client = sdr::GnuradioClient::new(&cfg);
            match client
                .start_demod(&mode, &source, mhz, host, port, gain)
                .await
            {
                Ok(result) => ok(json!({ "success": true, "result": result })),
                Err(e) => fail(e),
            }
        }
        Command::GnuradioStop => {
            let cfg = sdr::GnuradioCfg::resolve(None, None, None);
            let client = sdr::GnuradioClient::new(&cfg);
            match client.stop_demod().await {
                Ok(result) => ok(json!({ "success": true, "result": result })),
                Err(e) => fail(e),
            }
        }
        Command::GnuradioDevices { source } => {
            let _ = source;
            let cfg = sdr::GnuradioCfg::resolve(None, None, None);
            let client = sdr::GnuradioClient::new(&cfg);
            match client.devices().await {
                Ok(devices) => ok(json!({ "success": true, "devices": devices })),
                Err(e) => fail(e),
            }
        }
        Command::BridgeEvent {
            title,
            summary,
            urgency,
        } => match bridge::aiwatcher_event(None, &title, summary, urgency).await {
            Ok(v) => ok(v),
            Err(e) => fail(e),
        },
        Command::BridgeSpeak { text } => match bridge::speak(None, &text, None).await {
            Ok(v) => ok(v),
            Err(e) => fail(e),
        },
        Command::BridgeWebhook {
            url,
            title,
            summary,
        } => match bridge::webhook(&url, &title, summary).await {
            Ok(v) => ok(v),
            Err(e) => fail(e),
        },
        Command::LlmChat {
            provider,
            model,
            prompt,
        } => {
            use llm::ChatMessage;
            match llm::chat(provider, None, model, None, vec![ChatMessage::user(prompt)]).await {
                Ok(reply) => ok(json!({ "success": true, "reply": reply })),
                Err(e) => fail(e),
            }
        }
        Command::LlmModels { provider } => match llm::models(provider, None, None).await {
            Ok(models) => ok(json!({ "success": true, "models": models })),
            Err(e) => fail(e),
        },
        Command::DecodeCopy => {
            let (morse, rtty, aprs) = decode::copy_tapes();
            ok(json!({
                "success": true,
                "live": false,
                "reason": "Practice tapes inside the program. The stick's samples are not in the window, so nothing on the air was copied.",
                "morse": morse,
                "rtty": rtty,
                "aprs": aprs,
            }))
        }
    }
}

async fn device_list() -> Value {
    let cfg = import::LegacyConfig::resolve();
    let mock = sdr::effective_mock(cfg_mock_setting(), &cfg).await;
    if mock {
        return json!({
            "success": true, "mock_mode": true,
            "devices": [{ "index": 0, "serial": "MOCK-0001", "mock": true }],
            "message": "Mock SDR active - synthetic IQ drives FFT and waterfall without a dongle.",
        });
    }
    let live = sdr::rtl_tcp_probe(&cfg.rtl_host, cfg.rtl_port).await;
    json!({
        "success": true, "mock_mode": false,
        "devices": if live { vec![json!({ "index": 0, "serial": "rtl-tcp", "host": cfg.rtl_host, "port": cfg.rtl_port })] } else { Vec::new() },
        "available": live,
        "note": "Direct-USB enumeration lands in Phase 3 (rusb); rtl_tcp is the Phase-2 hardware path.",
    })
}

fn cfg_mock_setting() -> import::MockSetting {
    import::LegacyConfig::resolve().mock
}

fn device_init(index: u32) -> Value {
    match sdr::device() {
        Ok(mut dev) => {
            dev.device_index = index;
            let info = sdr::device_info_json(&dev, true, dev.mock == import::MockSetting::Enabled);
            json!({ "success": true, "message": "Device initialized", "device": info })
        }
        Err(e) => fail(e),
    }
}

fn device_status() -> Value {
    match sdr::device() {
        Ok(dev) => {
            let info = sdr::device_info_json(&dev, true, dev.mock == import::MockSetting::Enabled);
            json!({ "success": true, "device": info, "mock_setting": dev.mock.as_str() })
        }
        Err(e) => fail(e),
    }
}

async fn device_health() -> Value {
    let cfg = import::LegacyConfig::resolve();
    let rtl = sdr::rtl_tcp_probe(&cfg.rtl_host, cfg.rtl_port).await;
    let sidecar_cfg = sdr::GnuradioCfg::resolve(None, None, None);
    let sidecar = sdr::GnuradioClient::new(&sidecar_cfg).is_reachable().await;
    json!({
        "success": true,
        "rtl_tcp": { "reachable": rtl, "host": cfg.rtl_host, "port": cfg.rtl_port },
        "gnuradio_sidecar": { "reachable": sidecar, "service_url": sidecar_cfg.service_url },
    })
}

async fn device_mock_mode(enabled: Option<bool>) -> Value {
    if let Ok(mut dev) = sdr::device() {
        if let Some(on) = enabled {
            dev.mock = if on {
                import::MockSetting::Enabled
            } else {
                import::MockSetting::Disabled
            };
        }
    }
    let cfg = import::LegacyConfig::resolve();
    let setting = sdr::device().map(|d| d.mock).unwrap_or(cfg.mock);
    let active = sdr::effective_mock(setting, &cfg).await;
    json!({
        "success": true,
        "mock_setting": setting.as_str(),
        "mock_active": active,
        "env_hint": "Set SDR_MCP_MOCK=enable|disable|auto for startup default",
    })
}

async fn device_scan(start_mhz: f64, end_mhz: f64, step_mhz: f64) -> Value {
    let freqs = match sdr::scan_plan(start_mhz, end_mhz, step_mhz) {
        Ok(f) => f,
        Err(e) => return fail(e),
    };
    let cfg = import::LegacyConfig::resolve();
    let setting = sdr::device().map(|d| d.mock).unwrap_or(cfg.mock);
    if !sdr::effective_mock(setting, &cfg).await {
        return fail(DeviceError::Offline(
            "scan needs hardware: no rtl_tcp stream reachable and direct USB lands in Phase 3"
                .to_string(),
        ));
    }
    let mut results = Vec::with_capacity(freqs.len());
    for freq in &freqs {
        if let Ok(mut gen) = sdr::mock_iq() {
            let samples = gen.generate(sdr::FFT_SIZE * 2);
            let (_, power) = sdr::compute_spectrum(&samples, sdr::DEFAULT_SAMPLE_RATE);
            let peak = power.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b));
            results.push(json!({ "frequency_mhz": freq, "peak_db": (peak * 10.0).round() / 10.0 }));
        }
        tokio::time::sleep(sdr::SCAN_DWELL).await;
    }
    json!({
        "success": true,
        "status": "scan_complete",
        "scan_parameters": { "start_mhz": start_mhz, "end_mhz": end_mhz, "step_mhz": step_mhz, "steps": freqs.len() },
        "scan_results": results,
        "source": "synthetic_iq",
    })
}

async fn spectrum_get() -> Value {
    let samples = match sdr::mock_iq() {
        Ok(mut gen) => gen.generate(sdr::FFT_SIZE * 2),
        Err(e) => return fail(e),
    };
    let (freqs, power) = sdr::compute_spectrum(&samples, sdr::DEFAULT_SAMPLE_RATE);
    if let Ok(mut wf) = sdr::waterfall() {
        wf.push(power.clone());
    }
    json!({
        "success": true,
        "frequencies": freqs,
        "spectrum": power,
        "source": "synthetic_iq",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_future_is_send() {
        fn assert_send<T: Send>(_: T) {}
        assert_send(dispatch(Command::Status));
    }

    #[tokio::test]
    async fn status_payload_is_success() {
        let value = dispatch(Command::Status).await;
        assert_eq!(value.get("success").and_then(Value::as_bool), Some(true));
    }

    #[tokio::test]
    async fn invalid_freq_is_typed_error() {
        let value = dispatch(Command::DeviceSetFreq { mhz: 0.1 }).await;
        assert_eq!(value.get("success").and_then(Value::as_bool), Some(false));
        assert_eq!(
            value.get("kind").and_then(Value::as_str),
            Some("invalid_input")
        );
    }

    #[tokio::test]
    async fn medium_wave_tunes_and_tapes_decode() {
        let tuned = dispatch(Command::DeviceSetFreq { mhz: 0.909 }).await;
        assert_eq!(tuned.get("success").and_then(Value::as_bool), Some(true));
        let copied = dispatch(Command::DecodeCopy).await;
        assert_eq!(copied.get("live").and_then(Value::as_bool), Some(false));
        assert_eq!(
            copied.get("morse").and_then(Value::as_str),
            Some("CQ DE SDR")
        );
        assert_eq!(copied.get("rtty").and_then(Value::as_str), Some("SDR"));
        assert_eq!(
            copied.get("aprs").and_then(Value::as_str),
            Some("SDR>APRS:>hi")
        );
    }

    #[tokio::test]
    async fn preset_and_band_queries_work_offline() {
        let tune = dispatch(Command::DeviceTunePreset {
            name: "BBC LW".to_string(),
        })
        .await;
        assert_eq!(tune.get("success").and_then(Value::as_bool), Some(true));
        let band = dispatch(Command::StationsByBand {
            band: "LW".to_string(),
        })
        .await;
        assert_eq!(band.get("total_stations").and_then(Value::as_u64), Some(4));
        let bad = dispatch(Command::StationsByBand {
            band: "FM".to_string(),
        })
        .await;
        assert_eq!(bad.get("success").and_then(Value::as_bool), Some(false));
    }

    #[tokio::test]
    async fn ws_ops_name_phase3() {
        let value = dispatch(Command::SpectrumWsStatus).await;
        assert_eq!(
            value.get("kind").and_then(Value::as_str),
            Some("unsupported")
        );
    }
}
