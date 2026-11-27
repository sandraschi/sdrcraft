//! L3 `sdrcraft-app`: immediate-mode `egui` dashboard calling `sdrcraft-core` in-process.
//!
//! Chrome contract (fleet shell, adapted from the `rust-devices-mcp`
//! blueprint): `harsh_visuals` dark + `crisp_light_visuals` light, vector
//! Sun/Moon toggle, `Ctrl+Wheel` zoom + reset pill, `Chat` (`gemma4:12b`),
//! `Settings` (OS vault off the UI thread + JSON registry), `Help`,
//! `Raw JSON`. Domain pages mirror the 11 legacy webapp routes in nav order
//! (see `docs/UI_PARITY.md`); the blueprint `Crossconnect` page is merged
//! into `App Hub` (adopted targets + bridge quick-send).
//!
//! Thread law (contract sect. 5): zero sync I/O on `ui()` — every action goes
//! through [`SdrApp::spawn`] over the `mpsc` job channel; `vault` (OS
//! Credential Manager) is touched only inside background tasks.

use eframe::egui;
use sdrcraft_core::{dispatch, Command};
use serde_json::Value;
use std::collections::VecDeque;
use std::future::Future;
use std::sync::mpsc;

use crate::{registry, vault};

// ---------------------------------------------------------------------------
// Theme + chrome (fleet shell)
// ---------------------------------------------------------------------------

const ACCENT: egui::Color32 = egui::Color32::from_rgb(34, 211, 238);
const SKY: egui::Color32 = egui::Color32::from_rgb(56, 189, 248);
const EMERALD: egui::Color32 = egui::Color32::from_rgb(52, 211, 153);
const AMBER: egui::Color32 = egui::Color32::from_rgb(251, 191, 36);
const RED: egui::Color32 = egui::Color32::from_rgb(248, 113, 113);
const VIOLET: egui::Color32 = egui::Color32::from_rgb(167, 139, 250);
const SLATE: egui::Color32 = egui::Color32::from_rgb(148, 163, 184);

/// Harsh dark visuals: near-black panels, cyan accents, 12px card radius.
fn harsh_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(242, 246, 252));
    visuals.window_fill = egui::Color32::from_rgb(2, 6, 18);
    visuals.panel_fill = egui::Color32::from_rgb(10, 18, 34);
    visuals.extreme_bg_color = egui::Color32::from_rgb(2, 6, 18);
    visuals.code_bg_color = egui::Color32::from_rgb(4, 10, 24);
    visuals.faint_bg_color = egui::Color32::from_rgb(16, 28, 50);
    visuals.warn_fg_color = egui::Color32::from_rgb(255, 200, 120);
    visuals.error_fg_color = egui::Color32::from_rgb(255, 130, 130);
    visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 212, 230));
    visuals.widgets.inactive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(235, 242, 252));
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, egui::Color32::WHITE);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, egui::Color32::WHITE);
    visuals.selection.bg_fill = egui::Color32::from_rgb(36, 64, 107);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.hyperlink_color = egui::Color32::from_rgb(125, 180, 255);
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 54, 84));
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 66, 102));
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, ACCENT);
    visuals
}

/// Light companion (slate-50 panels, white cards).
fn crisp_light_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(egui::Color32::from_rgb(15, 23, 42));
    visuals.window_fill = egui::Color32::from_rgb(248, 250, 252);
    visuals.panel_fill = egui::Color32::from_rgb(248, 250, 252);
    visuals.extreme_bg_color = egui::Color32::from_rgb(241, 245, 249);
    visuals.code_bg_color = egui::Color32::from_rgb(241, 245, 249);
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 232, 240));
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(203, 213, 225));
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, ACCENT);
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);
    visuals
}

/// Theme + typography. Styles are per-theme (`style_of`/`set_style_of`);
/// fonts are re-applied on every toggle (called in `new` and on flip).
fn apply_theme(ctx: &egui::Context, dark_mode: bool) {
    let theme = if dark_mode {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_visuals(if dark_mode {
        harsh_visuals()
    } else {
        crisp_light_visuals()
    });
    let mut style = (*ctx.style_of(theme)).clone();
    {
        use egui::TextStyle::*;
        for (text_style, size) in [
            (Body, 16.0),
            (Button, 16.0),
            (Heading, 23.0),
            (Monospace, 14.0),
            (Small, 13.0),
        ] {
            if let Some(font) = style.text_styles.get_mut(&text_style) {
                font.size = size;
            }
        }
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
    }
    ctx.set_style_of(theme, style);
}

/// Ctrl+MouseWheel global zoom via egui's own zoom delta (the sanctioned
/// path). Read the delta inside `input`, mutate `ctx` outside — never nest
/// `ctx` reads inside the `input` closure (RwLock re-entrancy deadlock).
fn handle_zoom(ctx: &egui::Context) {
    let zoom = ctx.input(|input| input.zoom_delta());
    if (zoom - 1.0).abs() > f32::EPSILON {
        ctx.set_zoom_factor((ctx.zoom_factor() * zoom).clamp(0.6, 2.5));
    }
}

/// Theme toggle: vector sun / crescent moon (glyphs would be tofu).
fn theme_toggle_button(ui: &mut egui::Ui, dark_mode: &mut bool) {
    let size = egui::vec2(30.0, 30.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if resp.clicked() {
        *dark_mode = !*dark_mode;
    }
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&resp);
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(6),
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let center = rect.center();
        if *dark_mode {
            // Sun: disc + 8 rays.
            ui.painter().circle_filled(center, 6.0, AMBER);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::PI / 4.0;
                let (dx, dy) = (a.cos(), a.sin());
                ui.painter().line_segment(
                    [
                        center + egui::vec2(dx * 8.0, dy * 8.0),
                        center + egui::vec2(dx * 11.0, dy * 11.0),
                    ],
                    egui::Stroke::new(1.5, AMBER),
                );
            }
        } else {
            // Crescent: slate disc with panel-colored bite.
            ui.painter().circle_filled(center, 7.0, SLATE);
            ui.painter()
                .circle_filled(center + egui::vec2(3.0, -2.0), 6.0, visuals.bg_fill);
        }
    }
    resp.on_hover_text("Toggle dark / light theme");
}

/// Card container matching the legacy `Card` look (border + radius).
fn card_frame(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .corner_radius(egui::CornerRadius::same(12))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 66, 102)))
        .show(ui, add_contents);
}

/// One band on the Listen page. Returns true when its button is pressed.
fn listen_card(
    ui: &mut egui::Ui,
    title: &str,
    range: &str,
    blurb: &str,
    action: &str,
    color: egui::Color32,
) -> bool {
    let mut go = false;
    egui::Frame::group(ui.style())
        .corner_radius(egui::CornerRadius::same(12))
        .stroke(egui::Stroke::new(1.0, color))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(210.0);
            ui.set_max_width(250.0);
            ui.colored_label(color, egui::RichText::new(title).strong());
            ui.small(range);
            ui.label(blurb);
            if ui.button(action).clicked() {
                go = true;
            }
        });
    go
}

/// Hero KPI: small label over a big colored value.
fn hero_kpi(ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
    ui.vertical(|ui| {
        ui.small(label);
        ui.colored_label(color, egui::RichText::new(value).size(22.0).strong());
    });
}

/// Status pill: painted dot + label (dots are geometry, never glyphs).
fn status_pill(ui: &mut egui::Ui, label: &str, good: bool, warn: bool) {
    ui.horizontal(|ui| {
        let color = if good {
            EMERALD
        } else if warn {
            AMBER
        } else {
            SLATE
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(label);
    });
}

/// Labeled single-line text field.
fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.text_edit_singleline(value).on_hover_text(hint);
    });
}

// ---------------------------------------------------------------------------
// Navigation + jobs
// ---------------------------------------------------------------------------

/// Domain pages in legacy nav order, then the fleet `Raw JSON` page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Overview,
    Spectrum,
    Waterfall,
    Stations,
    Tools,
    Status,
    AppHub,
    Chat,
    Help,
    Settings,
    Logs,
    Raw,
}

impl Page {
    fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Listen",
            Self::Spectrum => "Spectrum",
            Self::Waterfall => "Waterfall",
            Self::Stations => "Stations",
            Self::Tools => "Tools",
            Self::Status => "Status",
            Self::AppHub => "App Hub",
            Self::Chat => "AI Command",
            Self::Help => "Help",
            Self::Settings => "Settings",
            Self::Logs => "Logs",
            Self::Raw => "Raw JSON",
        }
    }

    fn all() -> &'static [Page] {
        use Page::*;
        &[
            Overview, Spectrum, Waterfall, Stations, Tools, Status, AppHub, Chat, Help, Settings,
            Logs, Raw,
        ]
    }
}

/// Which help window is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    None,
    Palette,
    Shortcuts,
    Licenses,
    About,
}

/// Boot latch: initial/auto-load tasks fire exactly once (no per-frame
/// task storms — contract sect. 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadState {
    Unloaded,
    Loading,
    Loaded,
}

enum Job {
    Status(Value),
    Decode(Value),
    Devices(Value),
    Spectrum { freqs: Vec<f64>, power: Vec<f64> },
    Stations(Value),
    Online(Value),
    Gnuradio(Value),
    Chat(String),
    ChatModels(Value),
    Bridge(Value),
    Scan(Value),
    Audio(Value),
    VaultStatus { line: String, key_present: bool },
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub struct SdrApp {
    page: Page,
    dark_mode: bool,
    applied_dark: bool,
    zoom_saved: f32,
    boot: LoadState,
    rt: tokio::runtime::Runtime,
    tx: mpsc::Sender<Job>,
    rx: mpsc::Receiver<Job>,
    pending: u32,
    status_line: String,
    log: VecDeque<String>,
    raw: String,
    // Domain snapshots (latest payloads per area).
    status_json: Value,
    devices_json: Value,
    health_json: Value,
    spectrum_freqs: Vec<f64>,
    spectrum_power: Vec<f64>,
    waterfall_rows: VecDeque<Vec<f32>>,
    stations_json: Value,
    stations_query: String,
    stations_band: String,
    stations_detail: String,
    sched_callsign: String,
    sched_day: String,
    sched_json: Value,
    online_query: String,
    online_json: Value,
    signal_query: String,
    signal_json: Value,
    tune_mhz: f64,
    gain_auto: bool,
    gain_db: f32,
    preset_name: String,
    scan_start: f64,
    scan_end: f64,
    scan_step: f64,
    scan_json: Value,
    gnuradio_mode: String,
    gnuradio_mhz: f64,
    gnuradio_json: Value,
    // Chat.
    chat_log: Vec<(String, String)>,
    chat_input: String,
    // Fleet LLM card.
    llm_provider: String,
    llm_base: String,
    llm_model: String,
    llm_models: Vec<String>,
    llm_status: String,
    llm_key_present: bool,
    llm_key_input: String,
    // Targets registry.
    registry: registry::Registry,
    adopt_host: String,
    adopt_port: String,
    adopt_kind: String,
    adopt_label: String,
    // Bridge quick-send (App Hub).
    bridge_title: String,
    bridge_summary: String,
    bridge_url: String,
    speak_text: String,
    // Logs page.
    logs_filter: String,
    // Help page.
    help_tab: usize,
    lang_de: bool,
    overlay: Overlay,
    decode_report: String,
    decode_mode: String,
    decode_input: String,
    audio_running: bool,
    live_sweep: bool,
    last_sweep: std::time::Instant,
}

impl SdrApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let (dark_mode, zoom_saved, page_idx, llm_provider, llm_base, llm_model) = load_ui_state();
        apply_theme(ctx, dark_mode);
        ctx.set_zoom_factor(zoom_saved);
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("sdrcraft-app-worker")
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("sdrcraft-app: cannot build async runtime: {e}");
                std::process::exit(1);
            }
        };
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            page: Page::all().get(page_idx).copied().unwrap_or(Page::Overview),
            dark_mode,
            applied_dark: dark_mode,
            zoom_saved,
            boot: LoadState::Unloaded,
            rt,
            tx,
            rx,
            pending: 0,
            status_line: "ready".to_string(),
            log: VecDeque::new(),
            raw: String::new(),
            status_json: Value::Null,
            devices_json: Value::Null,
            health_json: Value::Null,
            spectrum_freqs: Vec::new(),
            spectrum_power: Vec::new(),
            waterfall_rows: VecDeque::new(),
            stations_json: Value::Null,
            stations_query: "BBC".to_string(),
            stations_band: "All".to_string(),
            stations_detail: String::new(),
            sched_callsign: "BBC LW".to_string(),
            sched_day: String::new(),
            sched_json: Value::Null,
            online_query: "jazz".to_string(),
            online_json: Value::Null,
            signal_query: "FT8".to_string(),
            signal_json: Value::Null,
            tune_mhz: 101.5,
            gain_auto: true,
            gain_db: 20.0,
            preset_name: "BBC LW".to_string(),
            scan_start: 88.0,
            scan_end: 89.0,
            scan_step: 1.0,
            scan_json: Value::Null,
            gnuradio_mode: "fm".to_string(),
            gnuradio_mhz: 101.5,
            gnuradio_json: Value::Null,
            chat_log: Vec::new(),
            chat_input: String::new(),
            llm_provider,
            llm_base,
            llm_model,
            llm_models: Vec::new(),
            llm_status: "not tested".to_string(),
            llm_key_present: false,
            llm_key_input: String::new(),
            registry: registry::load(),
            adopt_host: "127.0.0.1".to_string(),
            adopt_port: "1234".to_string(),
            adopt_kind: "rtl_tcp".to_string(),
            adopt_label: String::new(),
            bridge_title: String::new(),
            bridge_summary: String::new(),
            bridge_url: String::new(),
            speak_text: String::new(),
            logs_filter: String::new(),
            help_tab: 0,
            lang_de: false,
            overlay: Overlay::None,
            decode_report: String::new(),
            decode_mode: "morse".to_string(),
            decode_input: "CQ DE SDR".to_string(),
            audio_running: false,
            live_sweep: false,
            last_sweep: std::time::Instant::now(),
        };
        app.push_log("boot: UI state loaded");
        app.spawn_vault_probe();
        app.spawn_devices();
        app.spawn_health();
        app.spawn_spectrum();
        app
    }

    /// Boot full state without opening a window (used by `--smoke`).
    pub fn boot_headless() -> Self {
        let status_json = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(|rt| rt.block_on(dispatch(Command::Status)))
            .unwrap_or_else(|_| serde_json::json!({"success": false}));
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("smoke: cannot build runtime: {e}");
                std::process::exit(1);
            }
        };
        let (tx, rx) = mpsc::channel();
        Self::minimal(status_json, runtime, tx, rx)
    }

    /// Raw-payload length for `--smoke` diagnostics.
    pub fn status_len(&self) -> usize {
        self.raw.chars().count()
    }

    fn minimal(
        status_json: Value,
        rt: tokio::runtime::Runtime,
        tx: mpsc::Sender<Job>,
        rx: mpsc::Receiver<Job>,
    ) -> Self {
        // Built from caller-provided runtime + channel (no fallible builds
        // in here — the only failure exit lives in `boot_headless`).
        Self {
            page: Page::Overview,
            dark_mode: true,
            applied_dark: true,
            zoom_saved: 1.0,
            boot: LoadState::Loaded,
            rt,
            tx,
            rx,
            pending: 0,
            status_line: "headless smoke".to_string(),
            log: VecDeque::new(),
            raw: serde_json::to_string_pretty(&status_json).unwrap_or_default(),
            status_json,
            devices_json: Value::Null,
            health_json: Value::Null,
            spectrum_freqs: Vec::new(),
            spectrum_power: Vec::new(),
            waterfall_rows: VecDeque::new(),
            stations_json: Value::Null,
            stations_query: String::new(),
            stations_band: "All".to_string(),
            stations_detail: String::new(),
            sched_callsign: String::new(),
            sched_day: String::new(),
            sched_json: Value::Null,
            online_query: String::new(),
            online_json: Value::Null,
            signal_query: String::new(),
            signal_json: Value::Null,
            tune_mhz: 101.5,
            gain_auto: true,
            gain_db: 20.0,
            preset_name: String::new(),
            scan_start: 88.0,
            scan_end: 89.0,
            scan_step: 1.0,
            scan_json: Value::Null,
            gnuradio_mode: "fm".to_string(),
            gnuradio_mhz: 101.5,
            gnuradio_json: Value::Null,
            chat_log: Vec::new(),
            chat_input: String::new(),
            llm_provider: "ollama".to_string(),
            llm_base: String::new(),
            llm_model: "gemma4:12b".to_string(),
            llm_models: Vec::new(),
            llm_status: "not tested".to_string(),
            llm_key_present: false,
            llm_key_input: String::new(),
            registry: registry::Registry::default(),
            adopt_host: String::new(),
            adopt_port: String::new(),
            adopt_kind: "rtl_tcp".to_string(),
            adopt_label: String::new(),
            bridge_title: String::new(),
            bridge_summary: String::new(),
            bridge_url: String::new(),
            speak_text: String::new(),
            logs_filter: String::new(),
            help_tab: 0,
            lang_de: false,
            overlay: Overlay::None,
            decode_report: String::new(),
            decode_mode: "morse".to_string(),
            decode_input: "CQ DE SDR".to_string(),
            audio_running: false,
            live_sweep: false,
            last_sweep: std::time::Instant::now(),
        }
    }

    // ----- job plumbing -----

    fn spawn<F>(&mut self, make: impl FnOnce() -> F + Send + 'static)
    where
        F: Future<Output = Job> + Send + 'static,
    {
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let _ = tx.send(make().await);
        });
        self.pending += 1;
        self.save_fields();
    }

    fn push_log(&mut self, line: &str) {
        self.log.push_back(line.to_string());
        while self.log.len() > 200 {
            self.log.pop_front();
        }
    }

    fn note(&mut self, line: impl Into<String>) {
        let line = line.into();
        self.status_line = line.clone();
        self.push_log(&line);
    }

    fn show_raw(&mut self, value: &Value) {
        self.raw = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    }

    fn drain(&mut self, ctx: &egui::Context) {
        while let Ok(job) = self.rx.try_recv() {
            self.pending = self.pending.saturating_sub(1);
            match job {
                Job::Decode(value) => {
                    if let Some(decoded) = value.get("decoded").and_then(Value::as_str) {
                        let mode = value.get("mode").and_then(Value::as_str).unwrap_or("");
                        self.decode_report = format!(
                            "{} decoded: {decoded} (practice modem, live=false)",
                            mode.to_uppercase()
                        );
                    } else {
                        let morse = value.get("morse").and_then(Value::as_str).unwrap_or("");
                        let rtty = value.get("rtty").and_then(Value::as_str).unwrap_or("");
                        let aprs = value.get("aprs").and_then(Value::as_str).unwrap_or("");
                        let reason = value.get("reason").and_then(Value::as_str).unwrap_or("");
                        self.decode_report =
                            format!("Morse: {morse}\nRTTY: {rtty}\nAPRS: {aprs}\n{reason}");
                    }
                    self.show_raw(&value);
                    self.note("decoder tapes copied");
                }
                Job::Status(value) => {
                    self.status_json = value.clone();
                    self.show_raw(&value);
                    self.note("connected to sdrcraft-core (in-process)");
                }
                Job::Devices(value) => {
                    self.devices_json = value.clone();
                    self.show_raw(&value);
                    self.note("device list refreshed");
                }
                Job::Spectrum { freqs, power } => {
                    // Feed the waterfall history (newest at the bottom).
                    let row: Vec<f32> = if power.is_empty() {
                        Vec::new()
                    } else {
                        let step = (power.len() / 64).max(1);
                        power
                            .iter()
                            .step_by(step)
                            .map(|db| ((db + 120.0) / 120.0).clamp(0.0, 1.0) as f32)
                            .collect()
                    };
                    self.waterfall_rows.push_back(row);
                    while self.waterfall_rows.len() > 60 {
                        self.waterfall_rows.pop_front();
                    }
                    self.spectrum_freqs = freqs;
                    self.spectrum_power = power;
                    self.note("spectrum refreshed (Demo synthetic IQ)");
                }
                Job::Stations(value) => {
                    self.stations_json = value.clone();
                    self.show_raw(&value);
                    self.note("station query complete");
                }
                Job::Online(value) => {
                    self.online_json = value.clone();
                    self.show_raw(&value);
                    self.note("online lookup complete");
                }
                Job::Gnuradio(value) => {
                    self.gnuradio_json = value.clone();
                    self.show_raw(&value);
                    self.note("sidecar op complete");
                }
                Job::Chat(reply) => {
                    self.chat_log.push(("assistant".to_string(), reply));
                    self.note("chat reply received");
                }
                Job::ChatModels(value) => {
                    let mut models = Vec::new();
                    if let Some(items) = value.get("models").and_then(Value::as_array) {
                        for m in items {
                            if let Some(name) = m.as_str() {
                                models.push(name.to_string());
                            }
                        }
                    }
                    self.llm_models = models;
                    self.llm_status = "connected".to_string();
                    self.show_raw(&value);
                    self.note("model list elicited");
                }
                Job::Bridge(value) => {
                    self.show_raw(&value);
                    self.note("bridge call complete");
                }
                Job::Scan(value) => {
                    self.scan_json = value.clone();
                    self.show_raw(&value);
                    self.note("scan complete");
                }
                Job::Audio(value) => {
                    self.audio_running = value
                        .get("running")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    self.show_raw(&value);
                    self.note(if self.audio_running {
                        "demo FM audio playing"
                    } else {
                        "audio stopped"
                    });
                }
                Job::VaultStatus { line, key_present } => {
                    self.llm_status = line.clone();
                    self.llm_key_present = key_present;
                    self.note(line);
                }
            }
        }
        if self.pending > 0 {
            ctx.request_repaint();
        }
    }

    // ----- spawn helpers (one per domain action) -----

    fn spawn_status(&mut self) {
        self.spawn(|| async { Job::Status(dispatch(Command::Status).await) });
    }

    fn spawn_decode(&mut self) {
        self.spawn(|| async { Job::Decode(dispatch(Command::DecodeCopy).await) });
    }

    fn spawn_decode_text(&mut self, mode: String, text: String) {
        self.spawn(move || async move {
            Job::Decode(dispatch(Command::DecodeText { mode, text }).await)
        });
    }

    fn spawn_devices(&mut self) {
        self.spawn(|| async { Job::Devices(dispatch(Command::DeviceList).await) });
    }

    fn spawn_health(&mut self) {
        self.spawn(|| async { Job::Status(dispatch(Command::DeviceHealth).await) });
    }

    fn spawn_boot(&mut self) {
        self.spawn_status();
        self.spawn_devices();
        self.spawn_health();
    }

    fn spawn_spectrum(&mut self) {
        self.spawn(|| async {
            let value = dispatch(Command::SpectrumGet).await;
            let freqs = value
                .get("frequencies")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            let power = value
                .get("spectrum")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default();
            Job::Spectrum { freqs, power }
        });
    }

    fn spawn_tune(&mut self, mhz: f64) {
        self.spawn(
            move || async move { Job::Status(dispatch(Command::DeviceSetFreq { mhz }).await) },
        );
    }

    fn spawn_gain(&mut self, gain: String) {
        self.spawn(
            move || async move { Job::Status(dispatch(Command::DeviceSetGain { gain }).await) },
        );
    }

    fn spawn_preset(&mut self, name: String) {
        self.spawn(move || async move {
            Job::Status(dispatch(Command::DeviceTunePreset { name }).await)
        });
    }

    fn spawn_scan(&mut self, start_mhz: f64, end_mhz: f64, step_mhz: f64) {
        self.spawn(move || async move {
            Job::Scan(
                dispatch(Command::DeviceScan {
                    start_mhz,
                    end_mhz,
                    step_mhz,
                })
                .await,
            )
        });
    }

    fn spawn_mock(&mut self, enabled: Option<bool>) {
        self.spawn(move || async move {
            Job::Status(dispatch(Command::DeviceMockMode { enabled }).await)
        });
    }

    fn spawn_audio_toggle(&mut self) {
        let start = !self.audio_running;
        self.spawn(move || async move {
            if start {
                Job::Audio(
                    dispatch(Command::SpectrumAudioStart {
                        mode: "fm".to_string(),
                    })
                    .await,
                )
            } else {
                Job::Audio(dispatch(Command::SpectrumAudioStop).await)
            }
        });
    }

    fn spawn_stations_search(&mut self, query: String, band: Option<String>) {
        self.spawn(move || async move {
            Job::Stations(
                dispatch(Command::StationsSearch {
                    query,
                    band,
                    country: None,
                })
                .await,
            )
        });
    }

    fn spawn_schedule(&mut self, callsign: String, day: Option<String>) {
        self.spawn(move || async move {
            Job::Stations(dispatch(Command::StationsSchedule { callsign, day }).await)
        });
    }

    fn spawn_online_search(&mut self, query: String) {
        self.spawn(move || async move {
            Job::Online(
                dispatch(Command::OnlineSearch {
                    query,
                    country: None,
                    language: None,
                    tag: None,
                    limit: 25,
                })
                .await,
            )
        });
    }

    fn spawn_signal_id(&mut self, query: String) {
        self.spawn(move || async move {
            Job::Online(dispatch(Command::OnlineSignalId { query }).await)
        });
    }

    fn spawn_gnuradio_health(&mut self) {
        self.spawn(|| async { Job::Gnuradio(dispatch(Command::GnuradioHealth).await) });
    }

    fn spawn_gnuradio_start(&mut self, mhz: f64, mode: String) {
        self.spawn(move || async move {
            Job::Gnuradio(
                dispatch(Command::GnuradioStart {
                    mhz,
                    mode,
                    source: "rtl_tcp".to_string(),
                    host: None,
                    port: None,
                    gain: 20.0,
                })
                .await,
            )
        });
    }

    fn spawn_gnuradio_stop(&mut self) {
        self.spawn(|| async { Job::Gnuradio(dispatch(Command::GnuradioStop).await) });
    }

    fn spawn_chat(&mut self, prompt: String) {
        let provider = Some(self.llm_provider.clone());
        let model = Some(self.llm_model.clone());
        let base = if self.llm_base.trim().is_empty() {
            None
        } else {
            Some(self.llm_base.clone())
        };
        self.spawn(move || async move {
            // Key resolution happens here, off the UI thread (never on ui()).
            let key = vault::get(vault::LLM_API_KEY).ok().flatten().or_else(|| {
                std::env::var("LLM_API_KEY")
                    .ok()
                    .filter(|s| !s.trim().is_empty())
            });
            let resolved = sdrcraft_core::llm::resolve(provider, base, model, key);
            match resolved {
                Ok(spec) => {
                    let kind = match spec.kind {
                        sdrcraft_core::llm::Provider::Ollama => "ollama",
                        sdrcraft_core::llm::Provider::LmStudio => "lmstudio",
                        sdrcraft_core::llm::Provider::OpenAi => "openai",
                        sdrcraft_core::llm::Provider::Anthropic => "anthropic",
                        sdrcraft_core::llm::Provider::Custom => "custom",
                    };
                    let msgs = vec![sdrcraft_core::llm::ChatMessage::user(prompt)];
                    match sdrcraft_core::llm::chat(
                        Some(kind.to_string()),
                        Some(spec.base.clone()),
                        Some(spec.model.clone()),
                        spec.api_key.clone(),
                        msgs,
                    )
                    .await
                    {
                        Ok(reply) => Job::Chat(reply),
                        Err(e) => Job::Chat(format!("LLM error: {e}")),
                    }
                }
                Err(e) => Job::Chat(format!("LLM error: {e}")),
            }
        });
    }

    fn spawn_chat_models(&mut self) {
        let provider = Some(self.llm_provider.clone());
        let base = if self.llm_base.trim().is_empty() {
            None
        } else {
            Some(self.llm_base.clone())
        };
        self.spawn(move || async move {
            match sdrcraft_core::llm::models(provider, base, None).await {
                Ok(models) => {
                    Job::ChatModels(serde_json::json!({ "success": true, "models": models }))
                }
                Err(e) => {
                    Job::ChatModels(serde_json::json!({ "success": false, "error": e.to_string() }))
                }
            }
        });
    }

    fn spawn_vault_probe(&mut self) {
        self.spawn(|| async {
            let present = vault::get(vault::LLM_API_KEY).ok().flatten().is_some();
            Job::VaultStatus {
                line: if present {
                    "vault: API key stored".to_string()
                } else {
                    "vault: empty (session keys only)".to_string()
                },
                key_present: present,
            }
        });
    }

    fn spawn_vault_save(&mut self, secret: String) {
        self.spawn(move || async move {
            let outcome = match vault::set(vault::LLM_API_KEY, secret.trim()) {
                Ok(()) => (
                    "vault: API key saved".to_string(),
                    vault::get(vault::LLM_API_KEY).ok().flatten().is_some(),
                ),
                Err(e) => (format!("vault: save failed ({e})"), false),
            };
            Job::VaultStatus {
                line: outcome.0,
                key_present: outcome.1,
            }
        });
    }

    fn spawn_bridge_event(&mut self, title: String, summary: String) {
        self.spawn(move || async move {
            let summary = if summary.trim().is_empty() {
                None
            } else {
                Some(summary)
            };
            Job::Bridge(
                dispatch(Command::BridgeEvent {
                    title,
                    summary,
                    urgency: None,
                })
                .await,
            )
        });
    }

    fn spawn_speak(&mut self, text: String) {
        self.spawn(
            move || async move { Job::Bridge(dispatch(Command::BridgeSpeak { text }).await) },
        );
    }

    // ----- persistence -----

    fn save_fields(&self) {
        let page_idx = Page::all()
            .iter()
            .position(|p| *p == self.page)
            .unwrap_or(0);
        let doc = serde_json::json!({
            "dark_mode": self.dark_mode,
            "zoom": self.zoom_saved,
            "page": page_idx,
            "llm_provider": self.llm_provider,
            "llm_base": self.llm_base,
            "llm_model": self.llm_model,
        });
        if let Some(path) = ui_state_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(text) = serde_json::to_string_pretty(&doc) {
                let _ = std::fs::write(path, text);
            }
        }
    }

    fn nav_button(&mut self, ui: &mut egui::Ui, page: Page) {
        let selected = self.page == page;
        if ui
            .selectable_label(selected, page.label())
            .on_hover_text(format!("Open {}", page.label()))
            .clicked()
        {
            self.page = page;
            self.save_fields();
            if page == Page::Overview && self.boot == LoadState::Loaded {
                self.spawn_boot();
            }
        }
    }
}

fn ui_state_path() -> Option<std::path::PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(|base| {
        std::path::PathBuf::from(base)
            .join("sdrcraft-mcp")
            .join("ui.json")
    })
}

fn load_ui_state() -> (bool, f32, usize, String, String, String) {
    let mut dark = true;
    let mut zoom = 1.0f32;
    let mut page = 0usize;
    let mut provider = "ollama".to_string();
    let mut base = String::new();
    let mut model = "gemma4:12b".to_string();
    if let Some(path) = ui_state_path() {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(doc) = serde_json::from_str::<Value>(&text) {
                if let Some(v) = doc.get("dark_mode").and_then(Value::as_bool) {
                    dark = v;
                }
                if let Some(v) = doc.get("zoom").and_then(Value::as_f64) {
                    let v = v as f32;
                    if v.is_finite() && (0.6..=2.5).contains(&v) {
                        zoom = v;
                    }
                }
                if let Some(v) = doc.get("page").and_then(Value::as_u64) {
                    page = v as usize;
                }
                if let Some(v) = doc.get("llm_provider").and_then(Value::as_str) {
                    provider = v.to_string();
                }
                if let Some(v) = doc.get("llm_base").and_then(Value::as_str) {
                    base = v.to_string();
                }
                if let Some(v) = doc.get("llm_model").and_then(Value::as_str) {
                    model = v.to_string();
                }
            }
        }
    }
    (dark, zoom, page, provider, base, model)
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

impl eframe::App for SdrApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Boot latch: exactly one boot task set (no per-frame storms).
        if self.boot == LoadState::Unloaded {
            self.boot = LoadState::Loading;
            self.spawn_boot();
            self.boot = LoadState::Loaded;
        }
        handle_zoom(ui.ctx());
        self.drain(ui.ctx());

        egui::Panel::top("sdrcraft_menu").show(ui, |ui| {
            self.menu_bar(ui);
        });

        egui::Panel::top("sdr_topbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("SDRCraft");
                ui.small(if self.lang_de { "Empfang" } else { "Receive" });
                ui.separator();
                theme_toggle_button(ui, &mut self.dark_mode);
                if self.dark_mode != self.applied_dark {
                    apply_theme(ui.ctx(), self.dark_mode);
                    self.applied_dark = self.dark_mode;
                    self.save_fields();
                }
                // Zoom pill doubles as the reset button.
                let zoom_label = format!("{}%", (ui.ctx().zoom_factor() * 100.0).round());
                if ui
                    .button(zoom_label)
                    .on_hover_text("UI zoom (Ctrl+wheel); click resets to 100%")
                    .clicked()
                {
                    ui.ctx().set_zoom_factor(1.0);
                    self.zoom_saved = 1.0;
                    self.save_fields();
                }
                let zoom_now = ui.ctx().zoom_factor();
                if (zoom_now - self.zoom_saved).abs() > f32::EPSILON {
                    self.zoom_saved = zoom_now;
                    self.save_fields();
                }
                ui.separator();
                let mock = self
                    .devices_json
                    .get("mock_mode")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                status_pill(
                    ui,
                    if mock {
                        "Demo Mode (No Dongle)"
                    } else {
                        "Hardware (rtl_tcp)"
                    },
                    !mock,
                    mock,
                );
                if ui
                    .button(if mock {
                        "Try Hardware"
                    } else {
                        "Use Demo Mode"
                    })
                    .on_hover_text("Switch between synthetic Demo IQ and live rtl_tcp hardware")
                    .clicked()
                {
                    self.spawn_mock(Some(!mock));
                    self.spawn_devices();
                    self.spawn_spectrum();
                }
                let audio_label = if self.audio_running {
                    "Stop Audio"
                } else {
                    "Play Demo FM"
                };
                if ui
                    .button(audio_label)
                    .on_hover_text("Play or stop demodulated FM audio via cpal")
                    .clicked()
                {
                    self.spawn_audio_toggle();
                }
                ui.separator();
                if self.pending > 0 {
                    ui.small(format!("{} jobs", self.pending));
                } else {
                    ui.small(&self.status_line);
                }
            });
        });

        egui::Panel::left("sdr_nav")
            .resizable(false)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.small(if self.lang_de { "Seiten" } else { "Pages" });
                    ui.separator();
                    for page in Page::all() {
                        self.nav_button(ui, *page);
                    }
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| match self.page {
                    Page::Overview => self.page_overview(ui),
                    Page::Spectrum => self.page_spectrum(ui),
                    Page::Waterfall => self.page_waterfall(ui),
                    Page::Stations => self.page_stations(ui),
                    Page::Tools => self.page_tools(ui),
                    Page::Status => self.page_status(ui),
                    Page::AppHub => self.page_apphub(ui),
                    Page::Chat => self.page_chat(ui),
                    Page::Help => self.page_help(ui),
                    Page::Settings => self.page_settings(ui),
                    Page::Logs => self.page_logs(ui),
                    Page::Raw => self.page_raw(ui),
                });
        });

        if self.live_sweep
            && self.pending == 0
            && self.last_sweep.elapsed() >= std::time::Duration::from_millis(250)
        {
            self.last_sweep = std::time::Instant::now();
            self.spawn_spectrum();
        }
        if self.live_sweep {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
        if self.pending > 0 {
            ui.ctx().request_repaint();
        }
        self.draw_overlay(ui.ctx());
    }
}

// ---------------------------------------------------------------------------
// Domain pages (legacy nav order + Raw JSON)
// ---------------------------------------------------------------------------

impl SdrApp {
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Status").clicked() {
                    self.page = Page::Status;
                    self.spawn_status();
                    ui.close();
                }
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Settings").clicked() {
                    self.page = Page::Settings;
                    ui.close();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Dark").clicked() {
                    self.dark_mode = true;
                    ui.close();
                }
                if ui.button("Light").clicked() {
                    self.dark_mode = false;
                    ui.close();
                }
                if ui.button("English").clicked() {
                    self.lang_de = false;
                    ui.close();
                }
                if ui.button("Deutsch").clicked() {
                    self.lang_de = true;
                    ui.close();
                }
                if ui.button("Spectrum").clicked() {
                    self.page = Page::Spectrum;
                    ui.close();
                }
                if ui.button("Waterfall").clicked() {
                    self.page = Page::Waterfall;
                    ui.close();
                }
            });
            ui.menu_button("Insert", |ui| {
                if ui.button("Stations").clicked() {
                    self.page = Page::Stations;
                    ui.close();
                }
            });
            ui.menu_button("Modify", |ui| {
                if ui.button("Tools").clicked() {
                    self.page = Page::Tools;
                    ui.close();
                }
            });
            ui.menu_button("Commands", |ui| {
                if ui.button("Command Palette").clicked() {
                    self.overlay = Overlay::Palette;
                    ui.close();
                }
                if ui.button("Refresh status").clicked() {
                    self.spawn_status();
                    ui.close();
                }
            });
            ui.menu_button("Window", |ui| {
                if ui.button("Chat").clicked() {
                    self.page = Page::Chat;
                    ui.close();
                }
                if ui.button("Logs").clicked() {
                    self.page = Page::Logs;
                    ui.close();
                }
                if ui.button("Raw JSON").clicked() {
                    self.page = Page::Raw;
                    ui.close();
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("SDR Primer / Help").clicked() {
                    self.page = Page::Help;
                    ui.close();
                }
                ui.separator();
                if ui.button("Command Palette").clicked() {
                    self.overlay = Overlay::Palette;
                    ui.close();
                }
                if ui.button("Keyboard Shortcuts").clicked() {
                    self.overlay = Overlay::Shortcuts;
                    ui.close();
                }
                if ui.button("Licenses").clicked() {
                    self.overlay = Overlay::Licenses;
                    ui.close();
                }
                if ui.button("About").clicked() {
                    self.overlay = Overlay::About;
                    ui.close();
                }
            });
        });
    }

    fn draw_overlay(&mut self, ctx: &egui::Context) {
        if self.overlay == Overlay::None {
            return;
        }
        let which = self.overlay;
        let title = match which {
            Overlay::Palette => "Command Palette",
            Overlay::Shortcuts => "Keyboard Shortcuts",
            Overlay::Licenses => "Licenses",
            Overlay::About => "About",
            Overlay::None => return,
        };
        let mut open = true;
        egui::Window::new(title).open(&mut open).show(ctx, |ui| {
            match which {
                Overlay::Palette => {
                    for page in Page::all() {
                        if ui.button(page.label()).clicked() {
                            self.page = *page;
                            self.overlay = Overlay::None;
                        }
                    }
                }
                Overlay::Shortcuts => {
                    ui.label("Ctrl+wheel zooms the window. Click the percent to reset.");
                }
                Overlay::Licenses => {
                    ui.label("SDRCraft is MIT OR Apache-2.0. See LICENSE-MIT and LICENSE-APACHE.");
                }
                Overlay::About => {
                    ui.label("SDRCraft receives with an RTL-SDR stick or mock data. This build does not transmit. A licensed amateur uses a separate radio. Explorers carry a registered 406 MHz locator beacon. See docs/TRANSMIT_BOXES_AND_BEACONS.md.");
                }
                Overlay::None => {}
            }
        });
        if !open {
            self.overlay = Overlay::None;
        }
    }

    fn demo_banner(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            status_pill(ui, "Demo data — no dongle required", false, true);
            ui.small("Synthetic IQ drives spectrum, waterfall and scan.");
        });
    }

    fn open_listen(&mut self, mhz: f64) {
        self.tune_mhz = mhz;
        self.spawn_tune(mhz);
        self.spawn_spectrum();
        self.page = Page::Spectrum;
    }

    fn page_overview(&mut self, ui: &mut egui::Ui) {
        ui.heading("What you can hear");
        ui.label("A notebook, the stick, and the whip from the box will find a few strong FM stations and then look empty. That is the antenna and the band. Pick one. The stick tunes there and the spectrum opens. No dongle: mock data still draws.");
        ui.horizontal_wrapped(|ui| {
            if listen_card(
                ui,
                "FM broadcast",
                "88-108 MHz",
                "A few loud local stations on the whip. Extend it and move the stick to a window.",
                "Tune 98 MHz",
                EMERALD,
            ) {
                self.scan_start = 88.0;
                self.scan_end = 108.0;
                self.scan_step = 0.2;
                self.open_listen(98.0);
            }
            if listen_card(
                ui,
                "Airband",
                "118-137 MHz",
                "Aircraft and towers, still plain AM. Some countries treat this as non-public. A vertical whip helps.",
                "Tune 125 MHz",
                SKY,
            ) {
                self.open_listen(125.0);
            }
            if listen_card(
                ui,
                "Aircraft beacons",
                "1090 MHz",
                "ADS-B. The short whip is finally about the right length. You see the carriers here.",
                "Tune 1090 MHz",
                AMBER,
            ) {
                self.open_listen(1090.0);
            }
            if listen_card(
                ui,
                "Amateur 2 m",
                "144-146 MHz",
                "Hams, in the clear, on a vertical. This build only receives.",
                "Tune 145.5 MHz",
                VIOLET,
            ) {
                self.open_listen(145.5);
            }
            if listen_card(
                ui,
                "Text beacons",
                "144.800 MHz",
                "APRS in Europe: slow public text and positions, meant to be read. Tune shows the bursts. The button below copies a practice frame, not the air.",
                "Tune 144.8 MHz",
                EMERALD,
            ) {
                self.help_tab = 7;
                self.open_listen(144.8);
            }
            if listen_card(
                ui,
                "Morse and teletype",
                "CW and RTTY",
                "The decoders copy a practice tape. Live letters need the stick's audio, which is not in the window yet.",
                "Copy a tape",
                AMBER,
            ) {
                self.help_tab = 7;
                self.spawn_decode();
            }
            if listen_card(
                ui,
                "Medium wave",
                "0.5-1.7 MHz",
                "AM broadcast. An RTL-SDR Blog v4 hears this. A plain R820T2 stick does not. Night is louder.",
                "Tune 0.909 MHz",
                ACCENT,
            ) {
                self.open_listen(0.909);
            }
            if listen_card(
                ui,
                "Shortwave",
                "3-30 MHz",
                "Same v4 path as medium wave. The whip is still deaf here. A wire is the antenna.",
                "Tune 5.975 MHz",
                ACCENT,
            ) {
                self.open_listen(5.975);
            }
            if listen_card(
                ui,
                "Longwave",
                "0.15-0.28 MHz",
                "Below 0.5 MHz on the stick. A Ham It Up adds 125 MHz: BBC 198 kHz is tuned as 125.198, AM.",
                "Tune 125.198",
                SKY,
            ) {
                self.gnuradio_mode = "am".to_string();
                self.gnuradio_mhz = 125.198;
                self.open_listen(125.198);
            }
            if listen_card(
                ui,
                "Police chatter",
                "Mostly gone",
                "The movie version is analog FM. European police voice is encrypted TETRA. This app does not decode it.",
                "Read why",
                RED,
            ) {
                self.help_tab = 6;
                self.page = Page::Help;
            }
            if listen_card(
                ui,
                "Terrestrial TV",
                "About 470-694 MHz",
                "Digital only. You may see a wide hump. This stick cannot turn it into a picture.",
                "Look at 490 MHz",
                SLATE,
            ) {
                self.help_tab = 6;
                self.open_listen(490.0);
            }
        });
        ui.separator();
        ui.strong("Decoders (Practice Modem)");
        ui.small("Practice tapes and custom text round-trip. Nothing on the air is copied until live stick audio is wired.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Copy Morse, RTTY, and APRS").clicked() {
                self.spawn_decode();
            }
            egui::ComboBox::from_id_salt("decode_mode")
                .selected_text(self.decode_mode.to_uppercase())
                .show_ui(ui, |ui| {
                    for mode in ["morse", "rtty", "aprs"] {
                        ui.selectable_value(
                            &mut self.decode_mode,
                            mode.to_string(),
                            mode.to_uppercase(),
                        );
                    }
                });
            ui.add(
                egui::TextEdit::singleline(&mut self.decode_input)
                    .desired_width(180.0)
                    .hint_text("e.g. CQ DE OE1ABC"),
            );
            if ui.button("Encode & Decode").clicked() {
                let mode = self.decode_mode.clone();
                let text = self.decode_input.clone();
                self.spawn_decode_text(mode, text);
            }
        });
        if !self.decode_report.is_empty() {
            ui.label(&self.decode_report);
        }
        ui.separator();
        self.demo_banner(ui);
        ui.separator();
        ui.horizontal(|ui| {
            ui.strong("Live status");
            if ui.button("Refresh").clicked() {
                self.spawn_boot();
            }
        });
        ui.small("Refreshes on demand (legacy auto-refreshed every 5 seconds)");
        let hw = self
            .devices_json
            .get("devices")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let mock = self
            .devices_json
            .get("mock_mode")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        ui.horizontal_wrapped(|ui| {
            card_frame(ui, |ui| {
                hero_kpi(
                    ui,
                    "RTL-SDR",
                    if hw > 0 && !mock {
                        "Detected"
                    } else if mock {
                        "Mock"
                    } else {
                        "Not found"
                    },
                    SKY,
                );
                ui.small(format!("{hw} device(s)"));
            });
            card_frame(ui, |ui| {
                let online = self
                    .health_json
                    .get("gnuradio_sidecar")
                    .and_then(|v| v.get("reachable"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                hero_kpi(
                    ui,
                    "GNU Radio",
                    if online { "Online" } else { "Offline" },
                    VIOLET,
                );
                ui.small("Demod sidecar :11223");
            });
            card_frame(ui, |ui| {
                let mhz = self
                    .status_json
                    .get("device")
                    .and_then(|v| v.get("center_freq"))
                    .and_then(Value::as_f64)
                    .map(|hz| hz / 1e6)
                    .unwrap_or(227.0);
                hero_kpi(ui, "Center Freq", &format!("{mhz:.1} MHz"), EMERALD);
                ui.small("Retune on Spectrum page");
            });
            card_frame(ui, |ui| {
                hero_kpi(ui, "MCP Bridge", "Online", ACCENT);
                ui.small("44 tools, in-process");
            });
        });
    }

    fn page_spectrum(&mut self, ui: &mut egui::Ui) {
        ui.heading("Spectrum");
        ui.small("2048-pt power spectrum in dB, centered on the tuned frequency.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Refresh spectrum").clicked() {
                self.spawn_spectrum();
            }
            let sweep_label = if self.live_sweep {
                "Stop Live Sweep"
            } else {
                "Start Live Sweep"
            };
            if ui.button(sweep_label).clicked() {
                self.live_sweep = !self.live_sweep;
                if self.live_sweep {
                    self.last_sweep = std::time::Instant::now();
                    self.spawn_spectrum();
                }
            }
            ui.small(format!("{} bins", self.spectrum_power.len()));
            if let Some((peak_db, peak_hz)) =
                spectrum_peak(&self.spectrum_freqs, &self.spectrum_power)
            {
                ui.small(format!("peak {peak_db:.1} dB @ {peak_hz:.0} Hz offset"));
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.small("Quick tune:");
            for (label, mhz) in [
                ("98.0 FM", 98.0),
                ("125.0 Air", 125.0),
                ("144.8 APRS", 144.8),
                ("145.5 2m", 145.5),
                ("1090 ADS-B", 1090.0),
                ("7.074 FT8", 7.074),
                ("14.074 FT8", 14.074),
                ("5.975 SW", 5.975),
            ] {
                if ui.small_button(label).clicked() {
                    self.tune_mhz = mhz;
                    self.spawn_tune(mhz);
                    self.spawn_spectrum();
                }
            }
        });
        spectrum_canvas(ui, &self.spectrum_freqs, &self.spectrum_power);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Center freq (MHz)");
            let tune_speed = if self.tune_mhz < 30.0 { 0.001 } else { 0.5 };
            ui.add(
                egui::DragValue::new(&mut self.tune_mhz)
                    .speed(tune_speed)
                    .range(0.5..=1766.0),
            );
            if ui.button("Tune").clicked() {
                self.spawn_tune(self.tune_mhz);
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.gain_auto, "Gain auto");
            if !self.gain_auto {
                ui.add(
                    egui::DragValue::new(&mut self.gain_db)
                        .speed(0.5)
                        .range(0.0..=49.6),
                );
            }
            if ui.button("Set gain").clicked() {
                let gain = if self.gain_auto {
                    "auto".to_string()
                } else {
                    format!("{}", self.gain_db)
                };
                self.spawn_gain(gain);
            }
        });
        ui.separator();
        ui.strong("Frequency scan");
        ui.horizontal(|ui| {
            ui.label("Start");
            ui.add(egui::DragValue::new(&mut self.scan_start).speed(1.0));
            ui.label("End");
            ui.add(egui::DragValue::new(&mut self.scan_end).speed(1.0));
            ui.label("Step");
            ui.add(egui::DragValue::new(&mut self.scan_step).speed(0.1));
            if ui.button("Scan").clicked() {
                self.spawn_scan(self.scan_start, self.scan_end, self.scan_step);
            }
        });
        ui.small("Max 100 steps; 0.1 s dwell per step; Demo IQ when unconfigured.");
        if let Some(results) = self
            .scan_json
            .get("scan_results")
            .and_then(Value::as_array)
            .cloned()
        {
            ui.small(format!("{} step(s)", results.len()));
            for row in results.iter().take(12) {
                ui.monospace(format!(
                    "{:.2} MHz  {:.1} dB",
                    row.get("frequency_mhz")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                    row.get("peak_db").and_then(Value::as_f64).unwrap_or(0.0)
                ));
            }
        }
    }

    fn page_waterfall(&mut self, ui: &mut egui::Ui) {
        ui.heading("Waterfall");
        ui.small("Scrolling signal history, newest at the bottom (60 rows kept).");
        ui.horizontal(|ui| {
            if ui.button("Refresh (adds a row)").clicked() {
                self.spawn_spectrum();
            }
            let sweep_label = if self.live_sweep {
                "Stop Live Sweep"
            } else {
                "Start Live Sweep"
            };
            if ui.button(sweep_label).clicked() {
                self.live_sweep = !self.live_sweep;
                if self.live_sweep {
                    self.last_sweep = std::time::Instant::now();
                    self.spawn_spectrum();
                }
            }
            if ui.button("Clear").clicked() {
                self.waterfall_rows.clear();
            }
            ui.small(format!("{} rows", self.waterfall_rows.len()));
        });
        waterfall_view(ui, &self.waterfall_rows);
    }

    fn page_stations(&mut self, ui: &mut egui::Ui) {
        ui.heading("Stations");
        ui.small("Seeded database (11 stations) plus online lookup.");
        ui.horizontal(|ui| {
            field(
                ui,
                "Search",
                &mut self.stations_query,
                "name, callsign or country",
            );
            if ui.button("Search").clicked() {
                let band = if self.stations_band == "All" {
                    None
                } else {
                    Some(self.stations_band.clone())
                };
                self.spawn_stations_search(self.stations_query.clone(), band);
            }
        });
        ui.horizontal(|ui| {
            ui.label("Band");
            egui::ComboBox::from_id_salt("stations_band")
                .selected_text(&self.stations_band)
                .show_ui(ui, |ui| {
                    for band in ["All", "LW", "MW", "SW", "VHF", "UHF"] {
                        ui.selectable_value(&mut self.stations_band, band.to_string(), band);
                    }
                });
        });
        if let Some(items) = self
            .stations_json
            .get("stations")
            .and_then(Value::as_array)
            .cloned()
        {
            ui.small(format!("{} station(s)", items.len()));
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for item in &items {
                        let name = str_field(item, "name");
                        let callsign = str_field(item, "callsign");
                        let mhz = item
                            .get("frequency_mhz")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0);
                        ui.horizontal(|ui| {
                            ui.strong(&name);
                            ui.small(format!("{callsign} | {mhz:.3} MHz"));
                            if ui.button("Tune").clicked() {
                                self.stations_detail = format!("{name} ({callsign})");
                                self.spawn_preset(callsign.clone());
                            }
                            if ui.button("Schedule").clicked() {
                                self.stations_detail = format!("{name} ({callsign})");
                                self.sched_callsign = callsign.clone();
                                self.spawn_schedule(callsign, None);
                            }
                        });
                    }
                });
        }
        ui.separator();
        if !self.stations_detail.is_empty() {
            ui.small(format!("Selected: {}", self.stations_detail));
        }
        ui.strong("Quick Presets");
        ui.horizontal_wrapped(|ui| {
            for preset in ["BBC LW", "ORF LW", "France Inter LW", "RTL LW"] {
                if ui.button(preset).clicked() {
                    self.spawn_preset(preset.to_string());
                }
            }
        });
        ui.separator();
        ui.strong("Program schedule");
        ui.horizontal(|ui| {
            field(ui, "Callsign", &mut self.sched_callsign, "e.g. BBC LW");
            field(ui, "Day", &mut self.sched_day, "optional, e.g. monday");
            if ui.button("Get schedule").clicked() {
                let day = if self.sched_day.trim().is_empty() {
                    None
                } else {
                    Some(self.sched_day.clone())
                };
                self.spawn_schedule(self.sched_callsign.clone(), day);
            }
        });
        if let Some(current) = self
            .sched_json
            .get("current_program")
            .and_then(Value::as_str)
        {
            ui.small(format!("Now playing: {current}"));
        }
        ui.separator();
        ui.strong("Manual tune");
        ui.horizontal(|ui| {
            field(
                ui,
                "Preset or frequency text",
                &mut self.preset_name,
                "callsign, name or LW key",
            );
            if ui.button("Tune").clicked() {
                self.spawn_preset(self.preset_name.clone());
            }
        });
        ui.separator();
        ui.strong("Online search (radio-browser.info)");
        ui.horizontal(|ui| {
            field(ui, "Query", &mut self.online_query, "e.g. jazz");
            if ui.button("Search online").clicked() {
                self.spawn_online_search(self.online_query.clone());
            }
        });
        if let Some(items) = self.online_json.get("stations").and_then(Value::as_array) {
            ui.small(format!("{} online result(s)", items.len()));
            for item in items.iter().take(8) {
                ui.small(format!(
                    "{} ({})",
                    str_field(item, "name"),
                    str_field(item, "country")
                ));
            }
        }
        ui.separator();
        ui.strong("Signal identification");
        ui.horizontal(|ui| {
            field(ui, "Signal", &mut self.signal_query, "e.g. AM, FM, FT8");
            if ui.button("Identify").clicked() {
                self.spawn_signal_id(self.signal_query.clone());
            }
        });
        if let Some(title) = self.signal_json.get("title").and_then(Value::as_str) {
            ui.small(format!(
                "{}: {}",
                title,
                str_field(&self.signal_json, "description")
            ));
        }
    }

    fn page_tools(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tools");
        ui.small("One click per operation group. Full surface: 43 MCP tools (38 ArtCraft + 5 legacy aliases).");
        ui.add_space(6.0);
        egui::Grid::new("tools_grid")
            .num_columns(2)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                card_frame(ui, |ui| {
                    ui.set_min_width(280.0);
                    ui.strong("Device");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("List devices").clicked() {
                            self.spawn_devices();
                        }
                        if ui.button("Status").clicked() {
                            self.spawn_status();
                        }
                        if ui.button("Health").clicked() {
                            self.spawn_health();
                        }
                        if ui.button("Mock on").clicked() {
                            self.spawn_mock(Some(true));
                        }
                        if ui.button("Mock off").clicked() {
                            self.spawn_mock(Some(false));
                        }
                    });
                });
                card_frame(ui, |ui| {
                    ui.set_min_width(280.0);
                    ui.strong("Spectrum & Stream");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Get spectrum").clicked() {
                            self.spawn_spectrum();
                        }
                        if ui.button("WS start").clicked() {
                            let tx = self.tx.clone();
                            self.rt.spawn(async move {
                                let _ = tx.send(Job::Status(
                                    dispatch(Command::SpectrumWsStart {
                                        host: "127.0.0.1".to_string(),
                                        port: 8765,
                                    })
                                    .await,
                                ));
                            });
                            self.pending += 1;
                        }
                        if ui.button("WS stop").clicked() {
                            let tx = self.tx.clone();
                            self.rt.spawn(async move {
                                let _ =
                                    tx.send(Job::Status(dispatch(Command::SpectrumWsStop).await));
                            });
                            self.pending += 1;
                        }
                        if ui.button("WS status").clicked() {
                            let tx = self.tx.clone();
                            self.rt.spawn(async move {
                                let _ =
                                    tx.send(Job::Status(dispatch(Command::SpectrumWsStatus).await));
                            });
                            self.pending += 1;
                        }
                        if ui.button("Audio status").clicked() {
                            let tx = self.tx.clone();
                            self.rt.spawn(async move {
                                let _ = tx
                                    .send(Job::Audio(dispatch(Command::SpectrumAudioStatus).await));
                            });
                            self.pending += 1;
                        }
                    });
                });
                ui.end_row();
                card_frame(ui, |ui| {
                    ui.set_min_width(280.0);
                    ui.strong("Stations & Decoders");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Search BBC").clicked() {
                            self.spawn_stations_search("BBC".to_string(), None);
                        }
                        if ui.button("LW band").clicked() {
                            self.spawn_stations_search(String::new(), Some("LW".to_string()));
                        }
                        if ui.button("Database stats").clicked() {
                            let tx = self.tx.clone();
                            self.rt.spawn(async move {
                                let _ =
                                    tx.send(Job::Stations(dispatch(Command::StationsStats).await));
                            });
                            self.pending += 1;
                        }
                        if ui.button("Copy practice tapes").clicked() {
                            self.spawn_decode();
                        }
                    });
                });
                card_frame(ui, |ui| {
                    ui.set_min_width(280.0);
                    ui.strong("GNU Radio Sidecar");
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Mode");
                        egui::ComboBox::from_id_salt("gnuradio_mode")
                            .selected_text(&self.gnuradio_mode)
                            .show_ui(ui, |ui| {
                                for mode in ["fm", "am", "usb", "lsb"] {
                                    ui.selectable_value(
                                        &mut self.gnuradio_mode,
                                        mode.to_string(),
                                        mode.to_uppercase(),
                                    );
                                }
                            });
                        let demod_speed = if self.gnuradio_mhz < 30.0 { 0.001 } else { 0.5 };
                        ui.add(
                            egui::DragValue::new(&mut self.gnuradio_mhz)
                                .speed(demod_speed)
                                .range(0.5..=1766.0),
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Sidecar health").clicked() {
                            self.spawn_gnuradio_health();
                        }
                        if ui.button("Start demod").clicked() {
                            self.spawn_gnuradio_start(
                                self.gnuradio_mhz,
                                self.gnuradio_mode.clone(),
                            );
                        }
                        if ui.button("Demod status").clicked() {
                            let tx = self.tx.clone();
                            self.rt.spawn(async move {
                                let _ =
                                    tx.send(Job::Gnuradio(dispatch(Command::GnuradioStatus).await));
                            });
                            self.pending += 1;
                        }
                        if ui.button("Stop demod").clicked() {
                            self.spawn_gnuradio_stop();
                        }
                    });
                });
                ui.end_row();
            });
        ui.separator();
        card_frame(ui, |ui| {
            ui.strong("Active Capabilities");
            ui.small("43 MCP tools over stdio: sdr.* device/spectrum/stations/online/gnuradio/decode, bridge.*, llm.*, plus legacy sdr_device/sdr_spectrum/sdr_stations/sdr_online/sdr_gnuradio aliases.");
            ui.small("RX-only binary: there is no transmit path by construction.");
        });
    }

    fn page_status(&mut self, ui: &mut egui::Ui) {
        ui.heading("Status");
        ui.small("Live telemetry from sdrcraft-core (in-process, zero ports).");
        if ui.button("Refresh telemetry").clicked() {
            self.spawn_boot();
        }
        let mock = self
            .devices_json
            .get("mock_mode")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let rtl = self
            .health_json
            .get("rtl_tcp")
            .and_then(|v| v.get("reachable"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let sidecar = self
            .health_json
            .get("gnuradio_sidecar")
            .and_then(|v| v.get("reachable"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        ui.horizontal_wrapped(|ui| {
            card_frame(ui, |ui| {
                ui.strong("RTL-SDR");
                status_pill(ui, if mock { "Mock IQ" } else { "Hardware" }, !mock, mock);
                ui.small(format!("rtl_tcp reachable: {rtl}"));
            });
            card_frame(ui, |ui| {
                ui.strong("GNU Radio sidecar");
                status_pill(
                    ui,
                    if sidecar { "Online" } else { "Offline" },
                    sidecar,
                    false,
                );
                ui.small("Demod service :11223");
            });
            card_frame(ui, |ui| {
                ui.strong("Mock IQ");
                status_pill(ui, if mock { "Active" } else { "Off" }, !mock, mock);
                ui.small("Synthetic demo stream");
            });
            card_frame(ui, |ui| {
                ui.strong("Activity");
                status_pill(
                    ui,
                    &format!("{} jobs pending", self.pending),
                    self.pending == 0,
                    self.pending > 0,
                );
                ui.small(&self.status_line);
            });
        });
        ui.separator();
        card_frame(ui, |ui| {
            ui.strong("Health Check Details");
            ui.monospace(&self.raw);
        });
    }

    fn page_apphub(&mut self, ui: &mut egui::Ui) {
        ui.heading("App Hub");
        ui.small(
            "Adopted targets plus fleet bridge quick-send (blueprint Crossconnect lives here).",
        );
        card_frame(ui, |ui| {
            ui.strong("Fleet Registry");
            if self.registry.devices.is_empty() {
                ui.small("No adopted targets yet.");
            }
            for dev in &self.registry.devices {
                ui.small(format!(
                    "{} · {}:{} ({})",
                    dev.label,
                    dev.ip,
                    dev.port.unwrap_or(0),
                    dev.kind
                ));
            }
            ui.separator();
            ui.horizontal(|ui| {
                field(ui, "Host", &mut self.adopt_host, "rtl_tcp host");
                field(ui, "Port", &mut self.adopt_port, "e.g. 1234");
            });
            ui.horizontal(|ui| {
                ui.label("Kind");
                egui::ComboBox::from_id_salt("adopt_kind")
                    .selected_text(&self.adopt_kind)
                    .show_ui(ui, |ui| {
                        for kind in ["rtl_tcp", "sidecar", "generic"] {
                            ui.selectable_value(&mut self.adopt_kind, kind.to_string(), kind);
                        }
                    });
                field(ui, "Label", &mut self.adopt_label, "optional");
                if ui.button("Adopt").clicked() {
                    if let Ok(port) = self.adopt_port.trim().parse::<u16>() {
                        let dev = registry::adopt(
                            &self.adopt_host.clone(),
                            port,
                            &self.adopt_kind.clone(),
                            &self.adopt_label.clone(),
                        );
                        self.registry.devices.push(dev);
                        registry::save(&self.registry);
                        self.note("target adopted");
                    } else {
                        self.note("adopt: port must be 1-65535");
                    }
                }
            });
        });
        ui.separator();
        card_frame(ui, |ui| {
            ui.strong("Bridge quick-send");
            field(ui, "Title", &mut self.bridge_title, "event title");
            field(ui, "Summary", &mut self.bridge_summary, "optional");
            ui.horizontal(|ui| {
                if ui.button("Send event to aiwatcher").clicked() {
                    self.spawn_bridge_event(self.bridge_title.clone(), self.bridge_summary.clone());
                }
            });
            field(ui, "Speak", &mut self.speak_text, "max 1000 chars");
            if ui.button("Speak via speech-mcp").clicked() {
                self.spawn_speak(self.speak_text.clone());
            }
            field(ui, "Webhook URL", &mut self.bridge_url, "http(s) only");
            if ui.button("POST webhook").clicked() {
                let url = self.bridge_url.clone();
                let title = if self.bridge_title.trim().is_empty() {
                    "sdrcraft-mcp".to_string()
                } else {
                    self.bridge_title.clone()
                };
                let summary = self.bridge_summary.clone();
                self.spawn(move || async move {
                    Job::Bridge(
                        dispatch(Command::BridgeWebhook {
                            url,
                            title,
                            summary: if summary.trim().is_empty() {
                                None
                            } else {
                                Some(summary)
                            },
                        })
                        .await,
                    )
                });
            }
        });
    }

    fn page_chat(&mut self, ui: &mut egui::Ui) {
        ui.heading("AI Command");
        if self.llm_model.trim().is_empty() {
            ui.colored_label(RED, "no model selected");
        }
        ui.small(format!(
            "Chat via {} / {} (key in OS vault, never on disk).",
            self.llm_provider, self.llm_model
        ));
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                if self.chat_log.is_empty() {
                    ui.small("Try: list devices, tune 101.5 mhz, spectrum, gnuradio health...");
                }
                for (role, text) in &self.chat_log {
                    ui.horizontal(|ui| {
                        ui.strong(format!("{role}:"));
                        ui.label(text);
                    });
                }
            });
        ui.horizontal(|ui| {
            let input = ui.text_edit_singleline(&mut self.chat_input);
            if ui.button("Send").clicked()
                && !self.chat_input.trim().is_empty()
                && !self.llm_model.trim().is_empty()
            {
                let prompt = self.chat_input.clone();
                self.chat_log.push(("you".to_string(), prompt.clone()));
                self.chat_input.clear();
                self.spawn_chat(prompt);
            }
            if input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let prompt = self.chat_input.clone();
                if !prompt.trim().is_empty() && !self.llm_model.trim().is_empty() {
                    self.chat_log.push(("you".to_string(), prompt.clone()));
                    self.chat_input.clear();
                    self.spawn_chat(prompt);
                }
            }
        });
        if ui.button("Clear chat").clicked() {
            self.chat_log.clear();
        }
    }

    fn page_help(&mut self, ui: &mut egui::Ui) {
        ui.heading("SDR Primer");
        ui.small("Horizontal tabs, like the legacy Help page.");
        let tabs = [
            "Tune and listen",
            "Spectrum",
            "Stations",
            "Mock and Demo",
            "Buying a dongle",
            "Antennas",
            "Police and TV",
            "Morse and text",
            "HackRF and TX safety",
        ];
        ui.horizontal_wrapped(|ui| {
            for (idx, tab) in tabs.iter().enumerate() {
                if ui.selectable_label(self.help_tab == idx, *tab).clicked() {
                    self.help_tab = idx;
                }
            }
        });
        ui.separator();
        match self.help_tab {
            0 => {
                card_frame(ui, |ui| {
                    ui.heading("Tuning and bands (0.5 to 1766 MHz)");
                    ui.label("Enter a frequency in MHz on the Spectrum page (0.5-1766) and press Tune, or pick a band card on Listen.");
                    ui.label("Under 24 MHz (HF / MW / LW): 0.5-24 MHz needs an RTL-SDR Blog v4 (direct sampling) or a native HF radio like the SDRplay RSP1B (1 kHz-2 GHz, 14-bit). A plain R820T2 stick (including Nooelec NESDR Nano 2 Plus) starts near 24-25 MHz.");
                    ui.label("Interesting signals under 24 MHz: DCF77 (77.5 kHz) and WWV/CHU time stations, European LW/MW at night, shortwave broadcast bands (49m/41m/31m/25m/19m), 40m/20m amateur HF (7.074 / 14.074 MHz FT8, 14.0956 MHz WSPR), Shanwick/Gander HF aero & VOLMET, DDH47 weatherfax, and 27 MHz CB.");
                    ui.label("With a Ham It Up upconverter, add 125 MHz and use AM: BBC 198 kHz is 125.198 MHz.");
                    ui.small("Full details: docs/TUNING_RANGE.md");
                });
            }
            1 => {
                card_frame(ui, |ui| {
                    ui.heading("Spectrum and Waterfall");
                    ui.label("Refresh spectrum runs a 2048-pt Hamming-windowed rustfft FFT in dB. The peak readout shows the strongest frequency bin.");
                    ui.label("Waterfall accumulates the last 60 rows (newest at the bottom). WebSocket streaming broadcasts live JSON frames on demand.");
                });
            }
            2 => {
                card_frame(ui, |ui| {
                    ui.heading("Stations and Online Search");
                    ui.label("11 seeded stations across LW, MW, SW, VHF, and UHF work completely offline with program schedules.");
                    ui.label("Online search queries radio-browser.info (25,000+ internet streams). Signal ID Wiki explains modes like FT8, RTTY, and APRS.");
                });
            }
            3 => {
                card_frame(ui, |ui| {
                    ui.heading("Demo Mode (No Dongle Required)");
                    ui.label("When no RTL-SDR stick or rtl_tcp server is connected, SDRCraft automatically runs in Demo Mode using deterministic synthetic IQ (four demo tones + drift).");
                    ui.label("Spectrum, Waterfall, Scan, Presets, and Play Demo FM (cpal audio) all work out of the box without hardware. Use the top-bar Demo Mode / Try Hardware button to switch.");
                });
            }
            4 => {
                card_frame(ui, |ui| {
                    ui.heading("Hardware and Buying Guide");
                    ui.label("Cheap RTL2832U + R820T2 TV-tuner sticks (8-bit, ~2.4 MHz BW, e.g. Nooelec NESDR Nano 2 Plus) cover ~25-1750 MHz. They work for FM, airband, and 1090 MHz ADS-B, but overload near strong local stations.");
                    ui.label("SDRplay RSP1B (recommended): 1 kHz-2 GHz, 14-bit, automatic preselection, MW/FM and DAB notches, 4.7 V Bias-T. Hear it in SDRconnect. This window does not open it. Procedure: docs/RSP1B.md.");
                    ui.label("Airspy HF+ Discovery: benchmark HF sensitivity (9 kHz-31 MHz, 60-260 MHz, open drivers), but no UHF.");
                    ui.label("RTL-SDR Blog v4: best budget stick (0.5-1766 MHz HF direct sampling), still 8-bit dynamic range.");
                    ui.label("HackRF One: 1 MHz-6 GHz 8-bit transceiver with high noise floor; skip for RX-only listening.");
                    ui.label("Windows RTL-SDR setup: install WinUSB via Zadig, then serve the stick with rtl_tcp on port 1234.");
                });
            }
            5 => {
                ui.label("The whip in the box hears a few strong local FM stations and sometimes nearby aircraft. It is a poor size for shortwave, it sits in the laptop's noise, and a vertical rod mismatches many horizontal FM broadcasts. Extend it, and move the stick to a window on a USB extension, before you decide the dongle is deaf.");
                ui.label("A dipole kit (two telescopic rods) is the indoor upgrade. Each side is about a quarter wave: 75 cm near 100 MHz, about 7 cm near 1090 MHz. An outdoor vertical or a discone is the VHF/UHF step (airband, repeaters, 1090 MHz).");
                ui.label("Wire between trees is the shortwave antenna. A dipole is two equal wires fed in the middle, about 150 / f(MHz) metres end to end. An inverted-V is that dipole with the centre high and the ends low. One long wire to a tree works too if it has a counterpoise.");
                ui.label("A Yagi is a boom of parallel rods that hears best from the front. A log-periodic looks similar and covers a wider band. A rhombic is four long wires in a diamond (a rhombus), tens of metres on a side, aimed at one horizon. A small loop is the indoor HF answer when you cannot hang wire. One antenna will not be good at 7 MHz and at 1090 MHz.");
                ui.small("Full note: docs/ANTENNAS.md. This build only receives.");
            }
            6 => {
                ui.heading("Police chatter");
                ui.label("It is mostly not a thing anymore, and where the voice is still in the clear the law is local. In the Hollywood era, police, fire, and ambulances used ordinary analog FM. A scanner could follow a pursuit. Many US fire and EMS channels are still like that, and some smaller police departments still are. Large US cities have moved to digital trunking (P25), and a growing share of police channels are encrypted, so a whip hears hiss or nothing. Europe went further: police voice in Austria, Germany, and most of the EU is TETRA, and it is encrypted. You will not hear units being dispatched.");
                ui.label("Listening to encrypted police radio, or trying to remove that encryption, is unlawful. This program has no decoder for TETRA or for encrypted P25, and it will not grow one. Broadcast FM, shortwave broadcasts, and amateur radio are the listens that stay both audible and ordinary. Airband is technically open AM; some countries still treat it as a private communication. Check before you leave it running.");
                ui.heading("Terrestrial TV");
                ui.label("Analog television is off the air in Europe and in the United States. What is left is digital: DVB-T or DVB-T2 in Europe, ATSC in North America. A channel is about 6 to 8 MHz wide. This stick stably samples about 2.4 MHz, so it can show you a raised hump in the UHF TV band and cannot assemble a picture. The chip began life as a TV tuner. The WinUSB driver that makes it a software radio replaces the TV driver. Free-to-air television wants that TV mode, or a separate tuner. Pay TV is encrypted on top of the broadcast. This app does not remove that either.");
                ui.small("Notes: docs/POLICE_AND_TV.md.");
            }
            7 => {
                ui.heading("Morse and teletype");
                ui.label("Morse and RTTY are decoded from a practice tape on the Listen page. The same functions will read the air when the stick's audio is in the window. That path is not here yet, so the report says the copy is not live. PSK31, FT8, and JS8 are still for fldigi and WSJT-X. Most of that traffic is below 24 MHz. A Blog v4 can tune there. A plain R820T2 stick cannot.");
                ui.heading("The phone in Tokyo");
                ui.label("A garakei TV antenna is for 1seg, the mobile slice of Japanese digital terrestrial TV. Tokyo's ISDB-T stations are still on the air from Skytree. Analog TV ended in 2011. In 2026 the stations dropped the 1seg news-and-weather data pages. The program picture continues, including TOKYO MX. One segment is narrow enough that a software radio could capture it. This app has no ISDB-T demodulator, so it will not show the picture. The phone itself still can, in the Tokyo coverage area, if its tuner still works.");
                ui.heading("Walkie-talkies");
                ui.label("License-free handhelds in Austria are PMR446, around 446 MHz. The analog ones are plain FM and only carry a short distance, so you hear a worksite or a family if they are nearby. That is a private conversation, not a broadcast. Digital handhelds are not intelligible. This app does not scan them.");
                ui.heading("Slow text in Austria");
                ui.label("Three different webs get called this. APRS on 144.800 MHz is the open amateur beacon net: positions and short text, published on purpose. MeshCom is the OeVSV LoRa mesh on 70 cm, also unencrypted and identified by callsign, bridged to APRS. Meshtastic on 868 MHz is the license-free cousin. Its default public channel is readable by every node on that preset. Private channels, and MeshCore, are actually encrypted. This app does not break those. It tunes 144.800 so you can see the bursts. The APRS line in the decoder report is a practice frame, not a station on the air.");
                ui.small("Note: docs/MORSE_TEXT_AND_HANDHELDS.md.");
            }
            _ => {
                ui.small("HackRF notes in the legacy repo are a buying guide only. This binary is RX-only by construction: no transmit command exists in the Command bus.");
            }
        }
    }

    fn page_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configuration");
        ui.small("LLM provider, SDR targets, and vault state.");
        card_frame(ui, |ui| {
            ui.strong("LLM Provider");
            ui.small("Chat backend for AI Command. Keys live in the OS Credential Manager, never on disk.");
            ui.horizontal(|ui| {
                ui.label("Provider");
                egui::ComboBox::from_id_salt("llm_provider")
                    .selected_text(&self.llm_provider)
                    .show_ui(ui, |ui| {
                        for p in ["ollama", "lmstudio", "openai", "anthropic", "custom"] {
                            ui.selectable_value(&mut self.llm_provider, p.to_string(), p);
                        }
                    });
            });
            field(
                ui,
                "Base URL",
                &mut self.llm_base,
                "blank = provider default",
            );
            ui.horizontal(|ui| {
                field(ui, "Model", &mut self.llm_model, "e.g. gemma4:12b");
                if ui.button("Elicit models").clicked() {
                    self.spawn_chat_models();
                }
            });
            if !self.llm_models.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Pick");
                    egui::ComboBox::from_id_salt("llm_model_pick")
                        .selected_text(&self.llm_model)
                        .show_ui(ui, |ui| {
                            for m in self.llm_models.clone() {
                                ui.selectable_value(&mut self.llm_model, m.clone(), m);
                            }
                        });
                });
            }
            ui.horizontal(|ui| {
                ui.label("API key");
                ui.add(
                    egui::TextEdit::singleline(&mut self.llm_key_input)
                        .password(true)
                        .hint_text("stored in OS vault"),
                );
                if ui.button("Save key").clicked() {
                    let secret = self.llm_key_input.clone();
                    self.llm_key_input.clear();
                    self.spawn_vault_save(secret);
                }
            });
            status_pill(
                ui,
                &format!("LLM: {}", self.llm_status),
                self.llm_status == "connected",
                self.llm_status == "not tested",
            );
            if ui.button("Test connection").clicked() {
                self.spawn_chat_models();
            }
            if ui.button("Save settings").clicked() {
                self.save_fields();
                self.note("settings saved to ui.json");
            }
        });
        ui.separator();
        card_frame(ui, |ui| {
            ui.strong("SDR targets (read-only import)");
            let cfg = sdrcraft_core::import::LegacyConfig::resolve();
            ui.small(format!("rtl_tcp: {}:{}", cfg.rtl_host, cfg.rtl_port));
            ui.small(format!("sidecar: {}", cfg.gnuradio_url));
            ui.small(format!("mock: {}", cfg.mock.as_str()));
            ui.small("Change via SDR_MCP_MOCK / RTL_TCP_* / GNURADIO_DEMOD_URL env, then relaunch. Adopt hosts on the App Hub page.");
        });
    }

    fn page_logs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Logs");
            if ui.button("Clear all logs").clicked() {
                self.log.clear();
            }
        });
        field(ui, "Search", &mut self.logs_filter, "filter lines");
        let filter = self.logs_filter.to_ascii_lowercase();
        let lines: Vec<String> = self
            .log
            .iter()
            .filter(|l| filter.is_empty() || l.to_ascii_lowercase().contains(&filter))
            .cloned()
            .collect();
        if lines.is_empty() {
            ui.small("No log entries");
        }
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for line in &lines {
                    let low = line.to_ascii_lowercase();
                    if low.contains("error") || low.contains("fail") {
                        ui.colored_label(RED, egui::RichText::new(line).monospace());
                    } else {
                        ui.monospace(line);
                    }
                }
            });
    }

    fn page_raw(&mut self, ui: &mut egui::Ui) {
        ui.heading("Raw JSON");
        ui.small("Latest payload returned by sdrcraft-core (fleet chrome page).");
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.monospace(&self.raw);
        });
    }
}

// ---------------------------------------------------------------------------
// Painters (no new crates: pure egui vectors + rects)
// ---------------------------------------------------------------------------

/// Strongest bin of the current spectrum, if any.
fn spectrum_peak(freqs: &[f64], power: &[f64]) -> Option<(f64, f64)> {
    if freqs.len() != power.len() || power.is_empty() {
        return None;
    }
    let mut best = 0usize;
    for (i, p) in power.iter().enumerate() {
        if *p > power[best] {
            best = i;
        }
    }
    Some((power[best], freqs[best]))
}

/// Spectrum polyline + dB grid + area fill + peak marker + hover crosshair.
fn spectrum_canvas(ui: &mut egui::Ui, freqs: &[f64], power: &[f64]) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(200.0), 220.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(4, 10, 24));
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 58, 95)),
        egui::StrokeKind::Inside,
    );
    let min_db = -120.0;
    let max_db = 0.0;
    let grid_stroke = egui::Stroke::new(
        1.0,
        egui::Color32::from_rgba_unmultiplied(148, 163, 184, 30),
    );
    for db_tick in [-20.0f32, -40.0, -60.0, -80.0, -100.0] {
        let v = 1.0 - (db_tick - min_db as f32) / (max_db as f32 - min_db as f32);
        let y = rect.top() + v * rect.height();
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            grid_stroke,
        );
        painter.text(
            egui::pos2(rect.left() + 6.0, y - 2.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{db_tick:.0} dB"),
            egui::FontId::monospace(10.0),
            SLATE,
        );
    }
    let center_x = rect.center().x;
    painter.line_segment(
        [
            egui::pos2(center_x, rect.top()),
            egui::pos2(center_x, rect.bottom()),
        ],
        grid_stroke,
    );

    if freqs.len() != power.len() || power.len() < 2 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No spectrum yet - Refresh spectrum",
            egui::FontId::monospace(13.0),
            SLATE,
        );
        return;
    }
    let n = power.len();
    let to_pos = |i: usize| {
        let t = i as f32 / (n - 1) as f32;
        let db = power[i].clamp(min_db, max_db) as f32;
        let v = 1.0 - (db - min_db as f32) / (max_db as f32 - min_db as f32);
        egui::pos2(
            rect.left() + t * rect.width(),
            rect.top() + v * rect.height(),
        )
    };
    let fill_color = egui::Color32::from_rgba_unmultiplied(56, 189, 248, 28);
    let mut prev = to_pos(0);
    for i in 1..n {
        let next = to_pos(i);
        if i % 4 == 0 {
            let strip = egui::Rect::from_min_max(
                egui::pos2(prev.x, prev.y.min(next.y)),
                egui::pos2(next.x + 1.0, rect.bottom()),
            );
            painter.rect_filled(strip, 0.0, fill_color);
        }
        painter.line_segment([prev, next], egui::Stroke::new(1.4, SKY));
        prev = next;
    }
    if let Some((peak_db, peak_hz)) = spectrum_peak(freqs, power) {
        let t = if freqs.len() > 1 {
            let f0 = freqs[0];
            let f1 = freqs[freqs.len() - 1];
            ((peak_hz - f0) / (f1 - f0).max(1.0)).clamp(0.0, 1.0) as f32
        } else {
            0.5
        };
        let v = 1.0 - ((peak_db.clamp(min_db, max_db) as f32 - min_db as f32) / 120.0);
        let pos = egui::pos2(
            rect.left() + t * rect.width(),
            rect.top() + v * rect.height(),
        );
        painter.circle_filled(pos, 4.0, AMBER);
        painter.text(
            pos + egui::vec2(8.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{peak_db:.1} dB"),
            egui::FontId::monospace(12.0),
            AMBER,
        );
    }
    if let Some(hover) = response.hover_pos() {
        if rect.contains(hover) {
            let t = ((hover.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let idx = ((t * (n - 1) as f32).round() as usize).min(n - 1);
            let hz = freqs[idx];
            let db = power[idx];
            let cross = egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(245, 158, 11, 120),
            );
            painter.line_segment(
                [
                    egui::pos2(hover.x, rect.top()),
                    egui::pos2(hover.x, rect.bottom()),
                ],
                cross,
            );
            painter.text(
                egui::pos2(rect.right() - 8.0, rect.top() + 8.0),
                egui::Align2::RIGHT_TOP,
                format!("{:+.1} kHz  {db:.1} dB", hz / 1e3),
                egui::FontId::monospace(11.0),
                AMBER,
            );
        }
    }
}

/// Heat color ramp for waterfall cells (deep blue to cyan to amber).
fn heat_color(t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        let k = t * 2.0;
        egui::Color32::from_rgb(0, (80.0 + 130.0 * k) as u8, (120.0 + 120.0 * k) as u8)
    } else {
        let k = (t - 0.5) * 2.0;
        egui::Color32::from_rgb(
            (40.0 + 215.0 * k) as u8,
            (210.0 + 20.0 * k) as u8,
            (240.0 - 160.0 * k) as u8,
        )
    }
}

/// Waterfall as painted rects (no texture crate needed): newest row at the
/// bottom, 64 columns, up to 60 rows.
fn waterfall_view(ui: &mut egui::Ui, rows: &VecDeque<Vec<f32>>) {
    const COLS: usize = 64;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(200.0), 260.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, egui::Color32::BLACK);
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 70, 96)),
        egui::StrokeKind::Inside,
    );
    if rows.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No rows yet — Refresh",
            egui::FontId::monospace(13.0),
            SLATE,
        );
        return;
    }
    let kept: Vec<&Vec<f32>> = rows.iter().rev().take(60).collect();
    let row_h = rect.height() / 60.0;
    let col_w = rect.width() / COLS as f32;
    for (r, row) in kept.iter().enumerate() {
        if row.is_empty() {
            continue;
        }
        for c in 0..COLS {
            let idx = c * row.len() / COLS;
            let t = row.get(idx).copied().unwrap_or(0.0);
            let cell = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + c as f32 * col_w,
                    rect.bottom() - (r + 1) as f32 * row_h,
                ),
                egui::vec2(col_w + 0.5, row_h + 0.5),
            );
            painter.rect_filled(cell, 0.0, heat_color(t));
        }
    }
}

// ---------------------------------------------------------------------------
// Small JSON field readers (char-boundary safe, no indexing)
// ---------------------------------------------------------------------------

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

// ---------------------------------------------------------------------------
// Parity tests: goldens + tofu guard
// ---------------------------------------------------------------------------

#[cfg(test)]
mod parity_tests {
    use super::*;
    use egui_kittest::Harness;

    /// Component snapshots guard the converged look (see docs/UI_PARITY.md).
    /// A failing snapshot means the look changed: inspect the PNG, then fix
    /// the code or bless the new gold. Goldens here are implementation
    /// snapshots; the legacy-pixel oracle diff is tracked in UI_PARITY.md
    /// until `docs/parity/legacy/` captures exist.
    /// Glyph allowlist: this file is pure ASCII by construction (vector
    /// icons, painted dots, no symbol glyphs). The scan below enforces it.
    #[test]
    fn glyph_source_scan() {
        const ALLOWED: &[char] = &['°', '×', '·', '…', '—'];
        let source = include_str!("app.rs");
        let mut offenders = Vec::new();
        for (idx, ch) in source.char_indices() {
            if !ch.is_ascii() && !ALLOWED.contains(&ch) {
                offenders.push((idx, ch));
                if offenders.len() >= 8 {
                    break;
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "non-allowlisted glyphs in app.rs: {offenders:?}"
        );
    }

    #[test]
    fn hero_kpi_card() {
        let mut harness = Harness::new_ui(|ui| {
            hero_kpi(ui, "RTL-SDR", "Mock", SKY);
        });
        harness.snapshot("hero_kpi");
    }

    #[test]
    fn status_pills() {
        let mut harness = Harness::new_ui(|ui| {
            status_pill(ui, "Online", true, false);
            status_pill(ui, "Demo data", false, true);
            status_pill(ui, "Offline", false, false);
        });
        harness.snapshot("pills");
    }

    #[test]
    fn spectrum_canvas_demo() {
        // End-to-end render lock: deterministic L1 Demo IQ to FFT to canvas.
        let mut gen = sdrcraft_core::sdr::MockIq::new(2_048_000.0, 227_000_000.0, 42);
        let samples = gen.generate(4096);
        let (freqs, power) = sdrcraft_core::sdr::compute_spectrum(&samples, 2_048_000.0);
        let mut harness = Harness::new_ui(|ui| {
            spectrum_canvas(ui, &freqs, &power);
        });
        harness.snapshot("spectrum_demo");
    }
}
