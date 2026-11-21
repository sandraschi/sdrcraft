#![forbid(unsafe_code)]
//! L3 launcher: native `eframe` window + `--smoke` headless CI flag.

mod app;
mod registry;
mod vault;

use app::SdrApp;
use eframe::egui;

fn main() {
    sdrcraft_core::error::install_panic_hook();
    let smoke = std::env::args().any(|a| a == "--smoke");
    if smoke {
        let app = SdrApp::boot_headless();
        let payload = serde_json::json!({
            "success": true,
            "smoke": "ok",
            "stage": "phase-4 L3 app (window + goldens in kittest)",
            "status_chars": app.status_len(),
        });
        match serde_json::to_string(&payload) {
            Ok(line) => println!("{line}"),
            Err(_) => println!("{{\"success\":false}}"),
        }
        return;
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 750.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        "SDRCraft",
        options,
        Box::new(|cc| Ok(Box::new(SdrApp::new(&cc.egui_ctx)))),
    );
    if let Err(e) = result {
        eprintln!("sdrcraft-app failed to launch: {e}");
    }
}
