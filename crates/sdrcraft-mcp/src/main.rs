#![forbid(unsafe_code)]
//! L2 `sdrcraft-mcp`: `clap` CLI (JSON on every command) + dual-aliased
//! MCP `2024-11-05` stdio server over [`sdrcraft_core::dispatch`].
//!
//! Tool naming (contract §8 + `docs/CONVERSION_DOSSIER.md` §3): every
//! operation is reachable three ways — dotted ArtCraft (`sdr.list_devices`),
//! underscore ArtCraft (`sdr_list_devices`), and the legacy Python
//! portmanteau (`sdr_device` with `operation="list"`). Existing skills and
//! IDE configs keep working with zero edits.
//!
//! Wire contract: zero bytes on `stdout` except single-line JSON-RPC frames;
//! diagnostics on `stderr`; `stdin` EOF exits cleanly.

use clap::{Parser, Subcommand};
use sdrcraft_core::{dispatch, Command};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

/// CLI surface. Every command emits JSON on stdout.
#[derive(Debug, Parser)]
#[command(
    name = "sdrcraft-mcp",
    version,
    about = "Pure-Rust SDR MCP server + CLI (all output JSON)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Liveness probe (no hardware, no network).
    Status,
    /// MCP stdio server: line-delimited JSON-RPC 2.0 on stdin/stdout.
    McpStdio,
    /// Device management (list, init, tune, scan, mock).
    Device {
        #[command(subcommand)]
        op: DeviceCmd,
    },
    /// Spectrum analysis + streaming state.
    Spectrum {
        #[command(subcommand)]
        op: SpectrumCmd,
    },
    /// Seeded station database (offline).
    Stations {
        #[command(subcommand)]
        op: StationsCmd,
    },
    /// Online radio databases (network).
    Online {
        #[command(subcommand)]
        op: OnlineCmd,
    },
    /// GNU Radio demod sidecar (network).
    Gnuradio {
        #[command(subcommand)]
        op: GnuradioCmd,
    },
    /// Fleet crossconnects (aiwatcher / speech / webhook).
    Bridge {
        #[command(subcommand)]
        op: BridgeCmd,
    },
    /// LLM chat + model picker (Ollama default `gemma4:12b`).
    Llm {
        #[command(subcommand)]
        op: LlmCmd,
    },
    /// Multi-step workflow runbook for a goal (deterministic guidance).
    Assist {
        /// Objective, e.g. "what is playing on 89.5 MHz right now".
        goal: String,
    },
    /// First-tool hint for a topic (deterministic guidance).
    Hint {
        /// Topic, e.g. "BBC".
        topic: String,
    },
    /// Copy Morse, RTTY, and APRS practice tapes (offline decoder check).
    DecodeCopy,
}

#[derive(Debug, Subcommand)]
enum DeviceCmd {
    List,
    Init {
        #[arg(long, default_value_t = 0)]
        index: u32,
    },
    Status,
    Health,
    SetFreq {
        #[arg(long)]
        mhz: f64,
    },
    SetGain {
        #[arg(long, default_value = "auto")]
        gain: String,
    },
    TunePreset {
        #[arg(long)]
        name: String,
    },
    Scan {
        #[arg(long)]
        start: f64,
        #[arg(long)]
        end: f64,
        #[arg(long, default_value_t = 1.0)]
        step: f64,
    },
    MockMode {
        /// Omit to query; pass true/false to set.
        #[arg(long)]
        enabled: Option<bool>,
    },
}

#[derive(Debug, Subcommand)]
enum SpectrumCmd {
    Get,
    Waterfall,
    WsStart {
        #[arg(long, default_value = "localhost")]
        host: String,
        #[arg(long, default_value_t = 8765)]
        port: u16,
    },
    WsStop,
    WsStatus,
    AudioStatus,
}

#[derive(Debug, Subcommand)]
enum StationsCmd {
    Search {
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long)]
        band: Option<String>,
        #[arg(long)]
        country: Option<String>,
    },
    ByBand {
        #[arg(long)]
        band: String,
    },
    ByCountry {
        #[arg(long)]
        country: String,
    },
    Schedule {
        #[arg(long)]
        callsign: String,
        #[arg(long)]
        day: Option<String>,
    },
    Stats,
}

#[derive(Debug, Subcommand)]
enum OnlineCmd {
    Search {
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long)]
        country: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: u32,
    },
    SignalId {
        #[arg(long)]
        query: String,
    },
}

#[derive(Debug, Subcommand)]
enum GnuradioCmd {
    Health,
    Status,
    Start {
        #[arg(long)]
        mhz: f64,
        #[arg(long, default_value = "fm")]
        mode: String,
        #[arg(long, default_value = "rtl_tcp")]
        source: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long, default_value_t = 20.0)]
        gain: f64,
    },
    Stop,
    Devices {
        #[arg(long, default_value = "rtl_tcp")]
        source: String,
    },
}

#[derive(Debug, Subcommand)]
enum BridgeCmd {
    Event {
        #[arg(long)]
        title: String,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        urgency: Option<f64>,
    },
    Speak {
        #[arg(long)]
        text: String,
    },
    Webhook {
        #[arg(long)]
        url: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        summary: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum LlmCmd {
    Chat {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        prompt: String,
    },
    Models {
        #[arg(long)]
        provider: Option<String>,
    },
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    sdrcraft_core::error::install_panic_hook();
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Status => emit(dispatch(Command::Status).await),
        Cmd::McpStdio => {
            if let Err(e) = run_mcp_stdio().await {
                eprintln!("sdrcraft-mcp stdio error: {e}");
            }
        }
        Cmd::Device { op } => emit(device_dispatch(op).await),
        Cmd::Spectrum { op } => emit(spectrum_dispatch(op).await),
        Cmd::Stations { op } => emit(stations_dispatch(op).await),
        Cmd::Online { op } => emit(online_dispatch(op).await),
        Cmd::Gnuradio { op } => emit(gnuradio_dispatch(op).await),
        Cmd::Bridge { op } => emit(bridge_dispatch(op).await),
        Cmd::Llm { op } => emit(llm_dispatch(op).await),
        Cmd::Assist { goal } => emit(assist_payload(&goal)),
        Cmd::Hint { topic } => emit(hint_payload(&topic)),
        Cmd::DecodeCopy => emit(dispatch(Command::DecodeCopy).await),
    }
}

fn emit(value: Value) {
    println!("{value}");
}

async fn device_dispatch(op: DeviceCmd) -> Value {
    dispatch(match op {
        DeviceCmd::List => Command::DeviceList,
        DeviceCmd::Init { index } => Command::DeviceInit { index },
        DeviceCmd::Status => Command::DeviceStatus,
        DeviceCmd::Health => Command::DeviceHealth,
        DeviceCmd::SetFreq { mhz } => Command::DeviceSetFreq { mhz },
        DeviceCmd::SetGain { gain } => Command::DeviceSetGain { gain },
        DeviceCmd::TunePreset { name } => Command::DeviceTunePreset { name },
        DeviceCmd::Scan { start, end, step } => Command::DeviceScan {
            start_mhz: start,
            end_mhz: end,
            step_mhz: step,
        },
        DeviceCmd::MockMode { enabled } => Command::DeviceMockMode { enabled },
    })
    .await
}

async fn spectrum_dispatch(op: SpectrumCmd) -> Value {
    dispatch(match op {
        SpectrumCmd::Get => Command::SpectrumGet,
        SpectrumCmd::Waterfall => Command::SpectrumWaterfall,
        SpectrumCmd::WsStart { host, port } => Command::SpectrumWsStart { host, port },
        SpectrumCmd::WsStop => Command::SpectrumWsStop,
        SpectrumCmd::WsStatus => Command::SpectrumWsStatus,
        SpectrumCmd::AudioStatus => Command::SpectrumAudioStatus,
    })
    .await
}

async fn stations_dispatch(op: StationsCmd) -> Value {
    dispatch(match op {
        StationsCmd::Search {
            query,
            band,
            country,
        } => Command::StationsSearch {
            query,
            band,
            country,
        },
        StationsCmd::ByBand { band } => Command::StationsByBand { band },
        StationsCmd::ByCountry { country } => Command::StationsByCountry { country },
        StationsCmd::Schedule { callsign, day } => Command::StationsSchedule { callsign, day },
        StationsCmd::Stats => Command::StationsStats,
    })
    .await
}

async fn online_dispatch(op: OnlineCmd) -> Value {
    dispatch(match op {
        OnlineCmd::Search {
            query,
            country,
            language,
            tag,
            limit,
        } => Command::OnlineSearch {
            query,
            country,
            language,
            tag,
            limit,
        },
        OnlineCmd::SignalId { query } => Command::OnlineSignalId { query },
    })
    .await
}

async fn gnuradio_dispatch(op: GnuradioCmd) -> Value {
    dispatch(match op {
        GnuradioCmd::Health => Command::GnuradioHealth,
        GnuradioCmd::Status => Command::GnuradioStatus,
        GnuradioCmd::Start {
            mhz,
            mode,
            source,
            host,
            port,
            gain,
        } => Command::GnuradioStart {
            mhz,
            mode,
            source,
            host,
            port,
            gain,
        },
        GnuradioCmd::Stop => Command::GnuradioStop,
        GnuradioCmd::Devices { source } => Command::GnuradioDevices { source },
    })
    .await
}

async fn bridge_dispatch(op: BridgeCmd) -> Value {
    dispatch(match op {
        BridgeCmd::Event {
            title,
            summary,
            urgency,
        } => Command::BridgeEvent {
            title,
            summary,
            urgency,
        },
        BridgeCmd::Speak { text } => Command::BridgeSpeak { text },
        BridgeCmd::Webhook {
            url,
            title,
            summary,
        } => Command::BridgeWebhook {
            url,
            title,
            summary,
        },
    })
    .await
}

async fn llm_dispatch(op: LlmCmd) -> Value {
    dispatch(match op {
        LlmCmd::Chat {
            provider,
            model,
            prompt,
        } => Command::LlmChat {
            provider,
            model,
            prompt,
        },
        LlmCmd::Models { provider } => Command::LlmModels { provider },
    })
    .await
}

// ---------------------------------------------------------------------------
// Deterministic guidance payloads (sampling helpers without a host sampler)
// ---------------------------------------------------------------------------

fn assist_payload(goal: &str) -> Value {
    json!({
        "success": true,
        "goal": goal,
        "runbook": [
            "1. Probe hardware: sdr.list_devices, then sdr.device_health",
            "2. Unknown band? sdr.stations_search with the station name",
            "3. Known frequency? sdr.set_frequency, then sdr.spectrum",
            "4. History view: sdr.waterfall; program info: sdr.stations_schedule",
            "5. FM audio path: sdr.gnuradio_health, then sdr.gnuradio_start",
        ],
        "note": "Host-side MCP sampling (agentic assist) can execute this runbook step by step; each step is one tool call above.",
    })
}

fn hint_payload(topic: &str) -> Value {
    json!({
        "success": true,
        "topic": topic,
        "first_tool": "sdr.stations_search",
        "arguments": { "query": topic },
        "alternates": ["sdr.online_search", "sdr.set_frequency", "sdr.spectrum"],
    })
}

// ---------------------------------------------------------------------------
// MCP tool catalog
// ---------------------------------------------------------------------------

/// Minimal honest input schema from (name, json-type) pairs + required keys.
fn schema(props: &[(&str, &str)], required: &[&str]) -> Value {
    let mut properties = serde_json::Map::new();
    for (name, kind) in props {
        properties.insert((*name).to_string(), json!({ "type": kind }));
    }
    json!({ "type": "object", "properties": properties, "required": required })
}

fn tool(name: &str, description: &str, props: &[(&str, &str)], required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": schema(props, required) })
}

/// Full catalog: 35 ArtCraft tools + 5 legacy portmanteau aliases = 40.
fn tools_list() -> Value {
    json!([
        // -- device (9) --
        tool("sdr.list_devices", "List RTL-SDR devices (Demo MOCK-0001 when no dongle). Legacy: sdr_device(operation=\"list\").", &[], &[]),
        tool("sdr.init_device", "Initialize device by index.", &[("index", "integer")], &[]),
        tool("sdr.device_status", "Current center freq, gain, mock setting. Legacy: sdr_device(operation=\"status\").", &[], &[]),
        tool("sdr.device_health", "RTL-SDR + GNU Radio sidecar reachability. Legacy: sdr_device(operation=\"health\").", &[], &[]),
        tool("sdr.set_frequency", "Retune center frequency (0.5-1766 MHz). Under 24 MHz needs an RTL-SDR Blog v4. Longwave under 0.5 MHz is rejected.", &[("mhz", "number")], &["mhz"]),
        tool("sdr.set_gain", "Gain 'auto' or 0-49.6 dB.", &[("gain", "string")], &[]),
        tool("sdr.tune_preset", "Tune by callsign, name fragment, or LW preset key.", &[("name", "string")], &["name"]),
        tool("sdr.scan", "Sweep start-end MHz in step MHz (max 100 steps).", &[("start_mhz", "number"), ("end_mhz", "number"), ("step_mhz", "number")], &["start_mhz", "end_mhz"]),
        tool("sdr.mock_mode", "Query (omit enabled) or force synthetic IQ on/off.", &[("enabled", "boolean")], &[]),
        // -- spectrum (6) --
        tool("sdr.spectrum", "2048-pt power spectrum in dB (Demo synthetic IQ). Legacy: sdr_spectrum(operation=\"spectrum\").", &[], &[]),
        tool("sdr.waterfall", "100-line waterfall history.", &[], &[]),
        tool("sdr.ws_start", "Start the spectrum WebSocket (Phase 3 streams; state tracked now).", &[("host", "string"), ("port", "integer")], &[]),
        tool("sdr.ws_stop", "Stop the spectrum WebSocket.", &[], &[]),
        tool("sdr.ws_status", "WebSocket server state.", &[], &[]),
        tool("sdr.audio_status", "FM audio relay state.", &[], &[]),
        // -- stations (5, offline) --
        tool("sdr.stations_search", "Seeded DB search by name/callsign/description/country.", &[("query", "string"), ("band", "string"), ("country", "string")], &[]),
        tool("sdr.stations_by_band", "All seeded stations on LW|MW|SW|VHF|UHF.", &[("band", "string")], &["band"]),
        tool("sdr.stations_by_country", "All seeded stations for a country.", &[("country", "string")], &["country"]),
        tool("sdr.stations_schedule", "Program schedule + current program for a callsign.", &[("callsign", "string"), ("day", "string")], &["callsign"]),
        tool("sdr.stations_stats", "Seeded DB counts.", &[], &[]),
        // -- online (2, network) --
        tool("sdr.online_search", "radio-browser.info search (25k+ stations).", &[("query", "string"), ("country", "string"), ("language", "string"), ("tag", "string"), ("limit", "integer")], &[]),
        tool("sdr.online_signal_id", "Signal Identification Wiki lookup (e.g. FT8).", &[("query", "string")], &["query"]),
        // -- gnuradio (5, network) --
        tool("sdr.gnuradio_health", "Demod sidecar reachability.", &[], &[]),
        tool("sdr.gnuradio_status", "Active demod flowgraph state.", &[], &[]),
        tool("sdr.gnuradio_start", "Start FM/AM/USB/LSB demod at mhz.", &[("mhz", "number"), ("mode", "string"), ("source", "string"), ("host", "string"), ("port", "integer"), ("gain", "number")], &["mhz"]),
        tool("sdr.gnuradio_stop", "Stop the active demod.", &[], &[]),
        tool("sdr.gnuradio_devices", "Sidecar-visible SDR devices.", &[("source", "string")], &[]),
        // -- service + kernel (8) --
        tool("sdr.status", "Liveness probe (no hardware, no network).", &[], &[]),
        tool("bridge.event", "Ingest an event into aiwatcher.", &[("title", "string"), ("summary", "string"), ("urgency", "number")], &["title"]),
        tool("bridge.speak", "Speak text via speech-mcp (max 1000 chars).", &[("text", "string")], &["text"]),
        tool("bridge.webhook", "POST title/summary to an http(s) webhook.", &[("url", "string"), ("title", "string"), ("summary", "string")], &["url", "title"]),
        tool("llm.chat", "One chat turn (Ollama gemma4:12b default).", &[("provider", "string"), ("model", "string"), ("prompt", "string")], &["prompt"]),
        tool("llm.models", "List models on the provider.", &[("provider", "string")], &[]),
        tool("sdr.agentic_assist", "Deterministic runbook for a goal (host sampling executes it). Legacy: sdr_agentic_assist.", &[("goal", "string")], &[]),
        tool("sdr.sampling_hint", "First-tool hint for a topic. Legacy: sdr_sampling_hint.", &[("topic", "string")], &[]),
        tool("sdr.decode_copy", "Copy Morse, RTTY, and APRS practice tapes (live=false; not live air).", &[], &[]),
        // -- legacy portmanteau aliases (5) --
        tool("sdr_device", "LEGACY alias: device ops via operation=list|initialize|status|health|set_frequency|set_gain|tune_preset|scan|mock_mode.", &[("operation", "string")], &[]),
        tool("sdr_spectrum", "LEGACY alias: operation=spectrum|waterfall|start_websocket|stop_websocket|websocket_status|audio_status.", &[("operation", "string")], &[]),
        tool("sdr_stations", "LEGACY alias: operation=search|by_band|by_country|schedule|stats.", &[("operation", "string")], &[]),
        tool("sdr_online", "LEGACY alias: operation=search|signal_id.", &[("operation", "string")], &[]),
        tool("sdr_gnuradio", "LEGACY alias: operation=health|status|start|stop|list_devices.", &[("operation", "string")], &[]),
    ])
}

// ---------------------------------------------------------------------------
// Argument helpers: missing required params become isError content,
// never panics, never silent defaults.
// ---------------------------------------------------------------------------

fn get<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.get(key)
}

fn opt_string(args: &Value, key: &str) -> Option<String> {
    get(args, key).and_then(Value::as_str).map(str::to_string)
}

fn req_string(args: &Value, key: &str, owner: &str) -> Result<String, String> {
    match get(args, key).and_then(Value::as_str) {
        Some(v) if !v.trim().is_empty() => Ok(v.to_string()),
        _ => Err(format!("{key} is required for {owner}")),
    }
}

fn req_f64(args: &Value, key: &str, owner: &str) -> Result<f64, String> {
    match get(args, key).and_then(Value::as_f64) {
        Some(v) => Ok(v),
        None => Err(format!("{key} is required for {owner}")),
    }
}

fn opt_f64(args: &Value, key: &str, default: f64) -> f64 {
    get(args, key).and_then(Value::as_f64).unwrap_or(default)
}

fn opt_u16(args: &Value, key: &str, default: u16) -> u16 {
    get(args, key)
        .and_then(Value::as_u64)
        .and_then(|v| u16::try_from(v).ok())
        .unwrap_or(default)
}

fn opt_u32(args: &Value, key: &str, default: u32) -> u32 {
    get(args, key)
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(default)
}

fn opt_bool(args: &Value, key: &str) -> Option<bool> {
    get(args, key).and_then(Value::as_bool)
}

// ---------------------------------------------------------------------------
// Dual-aliased dispatcher
// ---------------------------------------------------------------------------

/// Map one `tools/call` invocation to [`dispatch`]. Unknown tools and
/// not-yet-ported operations become `isError` content, never a panic and
/// never a protocol-level error (the session stays up).
async fn call_tool(name: &str, args: &Value) -> Value {
    let err = |message: String| json!({ "content": [{ "type": "text", "text": message }], "isError": true });
    // Legacy portmanteau group: sdr_device | sdr_spectrum | sdr_stations |
    // sdr_online | sdr_gnuradio with an `operation` argument.
    if let Some(group) = name.strip_prefix("sdr_") {
        match group {
            "device" | "spectrum" | "stations" | "online" | "gnuradio" => {
                return legacy_dispatch(group, args).await;
            }
            _ => {}
        }
    }
    // ArtCraft forms: dotted `sdr.list_devices` or underscored
    // `sdr_list_devices` (plus bridge.* / llm.* equivalents). Normalize to
    // the dotted form and run the single dispatch table below.
    let (group, action) = match split_tool_name(name) {
        Some(pair) => pair,
        None => {
            return err(format!(
                "unknown tool '{name}'. 35 ArtCraft tools (sdr.*) + 5 legacy aliases (sdr_device, sdr_spectrum, sdr_stations, sdr_online, sdr_gnuradio)"
            ));
        }
    };
    let dotted = format!("{group}.{action}");
    dispatch_named(&dotted, args).await
}

/// Split `sdr.list_devices` / `sdr_list_devices` / `bridge.event` into
/// (group, action). Returns `None` for anything outside the fleet surface.
fn split_tool_name(name: &str) -> Option<(&str, &str)> {
    if let Some(rest) = name.strip_prefix("sdr.") {
        if rest.contains('.') || rest.is_empty() {
            return None;
        }
        return Some(("sdr", rest));
    }
    if let Some(rest) = name.strip_prefix("bridge.") {
        if rest.contains('.') || rest.is_empty() {
            return None;
        }
        return Some(("bridge", rest));
    }
    if let Some(rest) = name.strip_prefix("llm.") {
        if rest.contains('.') || rest.is_empty() {
            return None;
        }
        return Some(("llm", rest));
    }
    // Underscore forms. The five legacy portmanteau names are handled by the
    // caller before this runs; anything else shaped sdr_* is an action.
    if let Some(rest) = name.strip_prefix("sdr_") {
        if rest.contains('.') || rest.is_empty() {
            return None;
        }
        return Some(("sdr", rest));
    }
    if let Some(rest) = name.strip_prefix("bridge_") {
        if rest.contains('.') || rest.is_empty() {
            return None;
        }
        return Some(("bridge", rest));
    }
    if let Some(rest) = name.strip_prefix("llm_") {
        if rest.contains('.') || rest.is_empty() {
            return None;
        }
        return Some(("llm", rest));
    }
    None
}

/// Legacy portmanteau routing: `sdr_device(operation="list")` etc., with the
/// verbatim `valid_operations` arrays on unknown operations.
async fn legacy_dispatch(group: &str, args: &Value) -> Value {
    let err = |message: String| json!({ "content": [{ "type": "text", "text": message }], "isError": true });
    let op = opt_string(args, "operation").unwrap_or_default();
    let op = op.trim().to_ascii_lowercase();
    // Reuse the ArtCraft router by rewriting to the dotted form.
    let (dotted_name, dotted_args) = match group {
        "device" => match op.as_str() {
            "list" => ("sdr.list_devices", args.clone()),
            "initialize" => ("sdr.init_device", args.clone()),
            "status" => ("sdr.device_status", args.clone()),
            "health" => ("sdr.device_health", args.clone()),
            "set_frequency" => (
                "sdr.set_frequency",
                rename_arg(args, "frequency_mhz", "mhz"),
            ),
            "set_gain" => ("sdr.set_gain", args.clone()),
            "tune_preset" => ("sdr.tune_preset", rename_arg(args, "preset_name", "name")),
            "scan" => (
                "sdr.scan",
                rename_args(
                    args,
                    &[
                        ("start_freq", "start_mhz"),
                        ("end_freq", "end_mhz"),
                        ("step_size", "step_mhz"),
                    ],
                ),
            ),
            "mock_mode" => ("sdr.mock_mode", rename_arg(args, "mock_enabled", "enabled")),
            _ => {
                return err(format!(
                    "Unknown operation: {op}. valid_operations: list, initialize, status, health, set_frequency, set_gain, tune_preset, scan, mock_mode"
                ));
            }
        },
        "spectrum" => match op.as_str() {
            "spectrum" => ("sdr.spectrum", args.clone()),
            "waterfall" => ("sdr.waterfall", args.clone()),
            "start_websocket" => ("sdr.ws_start", args.clone()),
            "stop_websocket" => ("sdr.ws_stop", args.clone()),
            "websocket_status" => ("sdr.ws_status", args.clone()),
            "audio_status" => ("sdr.audio_status", args.clone()),
            _ => {
                return err(format!(
                    "Unknown operation: {op}. valid_operations: spectrum, waterfall, start_websocket, stop_websocket, websocket_status, audio_status"
                ));
            }
        },
        "stations" => match op.as_str() {
            "search" => ("sdr.stations_search", args.clone()),
            "by_band" => ("sdr.stations_by_band", args.clone()),
            "by_country" => ("sdr.stations_by_country", args.clone()),
            "schedule" => (
                "sdr.stations_schedule",
                rename_arg(args, "station_callsign", "callsign"),
            ),
            "stats" => ("sdr.stations_stats", args.clone()),
            _ => {
                return err(format!(
                    "Unknown operation: {op}. valid_operations: search, by_band, by_country, schedule, stats"
                ));
            }
        },
        "online" => match op.as_str() {
            "search" => ("sdr.online_search", args.clone()),
            "signal_id" => ("sdr.online_signal_id", args.clone()),
            _ => {
                return err(format!(
                    "Unknown operation: {op}. valid_operations: search, signal_id"
                ));
            }
        },
        "gnuradio" => match op.as_str() {
            "health" => ("sdr.gnuradio_health", args.clone()),
            "status" => ("sdr.gnuradio_status", args.clone()),
            "start" => (
                "sdr.gnuradio_start",
                rename_arg(args, "frequency_mhz", "mhz"),
            ),
            "stop" => ("sdr.gnuradio_stop", args.clone()),
            "list_devices" => ("sdr.gnuradio_devices", args.clone()),
            _ => {
                return err(format!(
                    "Unknown operation: {op}. valid_operations: health, status, start, stop, list_devices"
                ));
            }
        },
        _ => {
            return err(format!("unknown legacy group '{group}'"));
        }
    };
    // Single hop into the shared ArtCraft table (dotted form only).
    dispatch_named(dotted_name, &dotted_args).await
}

/// Single ArtCraft dispatch table (dotted names only), shared by direct
/// calls and the legacy rewrite above — one table, no drift.
async fn dispatch_named(name: &str, args: &Value) -> Value {
    let ok = |payload: Value| {
        let text =
            serde_json::to_string(&payload).unwrap_or_else(|_| "{\"success\":false}".to_string());
        json!({ "content": [{ "type": "text", "text": text }] })
    };
    let err = |message: String| json!({ "content": [{ "type": "text", "text": message }], "isError": true });
    let cmd = match name {
        "sdr.status" => Command::Status,
        "sdr.list_devices" => Command::DeviceList,
        "sdr.init_device" => Command::DeviceInit {
            index: opt_u32(args, "index", 0),
        },
        "sdr.device_status" => Command::DeviceStatus,
        "sdr.device_health" => Command::DeviceHealth,
        "sdr.spectrum" => Command::SpectrumGet,
        "sdr.waterfall" => Command::SpectrumWaterfall,
        "sdr.ws_stop" => Command::SpectrumWsStop,
        "sdr.ws_status" => Command::SpectrumWsStatus,
        "sdr.audio_status" => Command::SpectrumAudioStatus,
        "sdr.stations_stats" => Command::StationsStats,
        "sdr.gnuradio_health" => Command::GnuradioHealth,
        "sdr.gnuradio_status" => Command::GnuradioStatus,
        "sdr.gnuradio_stop" => Command::GnuradioStop,
        "sdr.decode_copy" => Command::DecodeCopy,
        "sdr.set_frequency" => match req_f64(args, "mhz", "set_frequency") {
            Ok(mhz) => Command::DeviceSetFreq { mhz },
            Err(message) => return err(message),
        },
        "sdr.set_gain" => Command::DeviceSetGain {
            gain: opt_string(args, "gain").unwrap_or_else(|| "auto".to_string()),
        },
        "sdr.tune_preset" => match req_string(args, "name", "tune_preset") {
            Ok(name) => Command::DeviceTunePreset { name },
            Err(message) => return err(message),
        },
        "sdr.scan" => {
            let start = match req_f64(args, "start_mhz", "scan") {
                Ok(v) => v,
                Err(message) => return err(message),
            };
            let end = match req_f64(args, "end_mhz", "scan") {
                Ok(v) => v,
                Err(message) => return err(message),
            };
            Command::DeviceScan {
                start_mhz: start,
                end_mhz: end,
                step_mhz: opt_f64(args, "step_mhz", 1.0),
            }
        }
        "sdr.mock_mode" => Command::DeviceMockMode {
            enabled: opt_bool(args, "enabled"),
        },
        "sdr.ws_start" => Command::SpectrumWsStart {
            host: opt_string(args, "host").unwrap_or_else(|| "localhost".to_string()),
            port: opt_u16(args, "port", 8765),
        },
        "sdr.stations_search" => Command::StationsSearch {
            query: opt_string(args, "query").unwrap_or_default(),
            band: opt_string(args, "band"),
            country: opt_string(args, "country"),
        },
        "sdr.stations_by_band" => match req_string(args, "band", "by_band") {
            Ok(band) => Command::StationsByBand { band },
            Err(message) => return err(message),
        },
        "sdr.stations_by_country" => match req_string(args, "country", "by_country") {
            Ok(country) => Command::StationsByCountry { country },
            Err(message) => return err(message),
        },
        "sdr.stations_schedule" => match req_string(args, "callsign", "schedule") {
            Ok(callsign) => Command::StationsSchedule {
                callsign,
                day: opt_string(args, "day"),
            },
            Err(message) => return err(message),
        },
        "sdr.online_search" => Command::OnlineSearch {
            query: opt_string(args, "query").unwrap_or_default(),
            country: opt_string(args, "country"),
            language: opt_string(args, "language"),
            tag: opt_string(args, "tag"),
            limit: opt_u32(args, "limit", 25),
        },
        "sdr.online_signal_id" => match req_string(args, "query", "signal_id") {
            Ok(query) => Command::OnlineSignalId { query },
            Err(message) => return err(message),
        },
        "sdr.gnuradio_start" => {
            let mhz = match req_f64(args, "mhz", "gnuradio start") {
                Ok(v) => v,
                Err(message) => return err(message),
            };
            Command::GnuradioStart {
                mhz,
                mode: opt_string(args, "mode").unwrap_or_else(|| "fm".to_string()),
                source: opt_string(args, "source").unwrap_or_else(|| "rtl_tcp".to_string()),
                host: opt_string(args, "host"),
                port: get(args, "port")
                    .and_then(Value::as_u64)
                    .and_then(|v| u16::try_from(v).ok()),
                gain: opt_f64(args, "gain", 20.0),
            }
        }
        "sdr.gnuradio_devices" => Command::GnuradioDevices {
            source: opt_string(args, "source").unwrap_or_else(|| "rtl_tcp".to_string()),
        },
        "sdr.agentic_assist" => {
            return ok(assist_payload(
                &opt_string(args, "goal").unwrap_or_default(),
            ));
        }
        "sdr.sampling_hint" => {
            return ok(hint_payload(&opt_string(args, "topic").unwrap_or_default()));
        }
        "bridge.event" => match req_string(args, "title", "bridge.event") {
            Ok(title) => Command::BridgeEvent {
                title,
                summary: opt_string(args, "summary"),
                urgency: get(args, "urgency").and_then(Value::as_f64),
            },
            Err(message) => return err(message),
        },
        "bridge.speak" => match req_string(args, "text", "bridge.speak") {
            Ok(text) => Command::BridgeSpeak { text },
            Err(message) => return err(message),
        },
        "bridge.webhook" => {
            let url = match req_string(args, "url", "bridge.webhook") {
                Ok(v) => v,
                Err(message) => return err(message),
            };
            let title = match req_string(args, "title", "bridge.webhook") {
                Ok(v) => v,
                Err(message) => return err(message),
            };
            Command::BridgeWebhook {
                url,
                title,
                summary: opt_string(args, "summary"),
            }
        }
        "llm.chat" => match req_string(args, "prompt", "llm.chat") {
            Ok(prompt) => Command::LlmChat {
                provider: opt_string(args, "provider"),
                model: opt_string(args, "model"),
                prompt,
            },
            Err(message) => return err(message),
        },
        "llm.models" => Command::LlmModels {
            provider: opt_string(args, "provider"),
        },
        _ => {
            return err(format!(
                "unknown tool '{name}'. 35 ArtCraft tools (sdr.*) + 5 legacy aliases (sdr_device, sdr_spectrum, sdr_stations, sdr_online, sdr_gnuradio)"
            ));
        }
    };
    ok(dispatch(cmd).await)
}

/// Rename one argument key (legacy names → ArtCraft names), dropping the
/// original. Extra keys pass through untouched.
fn rename_arg(args: &Value, from: &str, to: &str) -> Value {
    rename_args(args, &[(from, to)])
}

fn rename_args(args: &Value, pairs: &[(&str, &str)]) -> Value {
    let mut map = match args.as_object() {
        Some(obj) => obj.clone(),
        None => serde_json::Map::new(),
    };
    for (from, to) in pairs {
        if let Some(value) = map.remove(*from) {
            map.insert((*to).to_string(), value);
        }
    }
    Value::Object(map)
}

// ---------------------------------------------------------------------------
// stdio loop (wire purity per contract §7)
// ---------------------------------------------------------------------------

fn respond(id: &Value, result: Value) {
    let frame = json!({ "jsonrpc": "2.0", "id": id, "result": result });
    if let Ok(line) = serde_json::to_string(&frame) {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

fn respond_error(id: &Value, code: i64, message: &str) {
    let frame =
        json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } });
    if let Ok(line) = serde_json::to_string(&frame) {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

/// Infallible JSON-RPC line loop: empty lines ignored, malformed JSON gets
/// `-32700` without exiting, unknown methods get `-32601`, notifications
/// get no reply, EOF ends the session with `Ok(())`.
async fn run_mcp_stdio() -> Result<(), String> {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("stdin read: {e}"))?;
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => {
                respond_error(&Value::Null, -32700, "Parse error");
                continue;
            }
        };
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let is_notification = msg.get("id").is_none();
        let method = msg
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        match method.as_str() {
            "initialize" => respond(
                &id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "sdrcraft-mcp", "version": "0.1.0" },
                }),
            ),
            "notifications/initialized" => {}
            "tools/list" => respond(&id, json!({ "tools": tools_list() })),
            "tools/call" => {
                let params = msg.get("params").cloned().unwrap_or(json!({}));
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                respond(&id, call_tool(&name, &args).await);
            }
            _ => {
                if !is_notification {
                    respond_error(&id, -32601, "Method not found");
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_41_tools() {
        let tools = tools_list();
        let items = tools.as_array().cloned().unwrap_or_default();
        assert_eq!(items.len(), 41);
        let names: Vec<String> = items
            .iter()
            .filter_map(|t| t.get("name").and_then(Value::as_str))
            .map(str::to_string)
            .collect();
        for legacy in [
            "sdr_device",
            "sdr_spectrum",
            "sdr_stations",
            "sdr_online",
            "sdr_gnuradio",
        ] {
            assert!(names.contains(&legacy.to_string()), "missing {legacy}");
        }
        for artcraft in [
            "sdr.list_devices",
            "sdr.spectrum",
            "sdr.stations_search",
            "sdr.online_search",
            "sdr.gnuradio_start",
            "sdr.status",
            "sdr.decode_copy",
            "bridge.event",
            "llm.chat",
        ] {
            assert!(names.contains(&artcraft.to_string()), "missing {artcraft}");
        }
    }

    #[tokio::test]
    async fn unknown_tool_becomes_error_content_not_panic() {
        let out = call_tool("nope.missing", &json!({})).await;
        assert_eq!(out.get("isError").and_then(Value::as_bool), Some(true));
    }

    #[tokio::test]
    async fn legacy_portmanteau_alias_routes_correctly() {
        // Offline-safe aliases (no network): device status, waterfall, stats.
        let status = call_tool("sdr_device", &json!({ "operation": "status" })).await;
        assert!(status.get("isError").is_none());
        let waterfall = call_tool("sdr_spectrum", &json!({ "operation": "waterfall" })).await;
        assert!(waterfall.get("isError").is_none());
        let stats = call_tool("sdr_stations", &json!({ "operation": "stats" })).await;
        assert!(stats.get("isError").is_none());
    }

    #[tokio::test]
    async fn underscore_form_routes_like_dotted() {
        let dotted = call_tool("sdr.stations_stats", &json!({})).await;
        let underscored = call_tool("sdr_stations_stats", &json!({})).await;
        assert!(dotted.get("isError").is_none());
        assert!(underscored.get("isError").is_none());
    }

    #[tokio::test]
    async fn demo_round_trip() {
        // Pure-offline path: seeded DB search + Demo spectrum, no hardware.
        let search = call_tool("sdr.stations_search", &json!({ "query": "BBC" })).await;
        assert!(search.get("isError").is_none());
        let spectrum = call_tool("sdr.spectrum", &json!({})).await;
        assert!(spectrum.get("isError").is_none());
        let tune = call_tool("sdr.tune_preset", &json!({ "name": "BBC LW" })).await;
        assert!(tune.get("isError").is_none());
    }

    #[tokio::test]
    async fn required_params_reject_missing_values() {
        let no_mhz = call_tool("sdr.set_frequency", &json!({})).await;
        assert_eq!(no_mhz.get("isError").and_then(Value::as_bool), Some(true));
        let no_band = call_tool("sdr.stations_by_band", &json!({})).await;
        assert_eq!(no_band.get("isError").and_then(Value::as_bool), Some(true));
        let no_callsign = call_tool("sdr_stations", &json!({ "operation": "schedule" })).await;
        assert_eq!(
            no_callsign.get("isError").and_then(Value::as_bool),
            Some(true)
        );
        let no_gnuradio_mhz = call_tool("sdr_gnuradio", &json!({ "operation": "start" })).await;
        assert_eq!(
            no_gnuradio_mhz.get("isError").and_then(Value::as_bool),
            Some(true)
        );
        let no_scan_end = call_tool("sdr.scan", &json!({ "start_mhz": 88.0 })).await;
        assert_eq!(
            no_scan_end.get("isError").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[tokio::test]
    async fn unknown_legacy_operation_lists_valid_operations() {
        let out = call_tool("sdr_device", &json!({ "operation": "defrag" })).await;
        assert_eq!(out.get("isError").and_then(Value::as_bool), Some(true));
        let text = out
            .get("content")
            .and_then(Value::as_array)
            .and_then(|c| c.first().cloned())
            .and_then(|m| m.get("text").cloned())
            .and_then(|t| t.as_str().map(str::to_string))
            .unwrap_or_default();
        assert!(text.contains("valid_operations"));
    }
}
