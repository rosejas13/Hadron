// Hadron — Goldleaf's USB client (native Rust port of Quark, ARM macOS).
// Ported from the Java Quark by XorTroll (GPL-3.0-or-later).

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

mod app;
mod buffer;
mod command_block;
mod command_framework;
mod config;
mod filesystem;
mod logging;
mod usb;
mod version;

use app::{HadronApp, UsbToUi};
use config::Config;

fn load_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/Icon.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    })
}

fn parse_cfgfile_arg(args: &[String]) -> Option<PathBuf> {
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if (a == "--cfgfile" || a == "-cfgfile") && i + 1 < args.len() {
            return Some(PathBuf::from(&args[i + 1]));
        }
        if let Some(rest) = a.strip_prefix("--cfgfile=") {
            return Some(PathBuf::from(rest));
        }
        i += 1;
    }
    None
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cfg_path = parse_cfgfile_arg(&args).unwrap_or_else(config::default_config_path);

    let cfg = match Config::load(&cfg_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Failed to load config from {}: {e}",
                cfg_path.display()
            );
            Config::empty(cfg_path.clone())
        }
    };
    let cfg = Arc::new(Mutex::new(cfg));

    let (usb_tx, usb_rx) = mpsc::channel::<UsbToUi>();

    // USB worker thread (keeps all nusb handles on one thread).
    {
        let cfg = Arc::clone(&cfg);
        let usb_tx = usb_tx.clone();
        std::thread::Builder::new()
            .name("usb-worker".into())
            .spawn(move || usb::run_usb_loop(cfg, usb_tx))
            .expect("failed to spawn USB worker thread");
    }

    let viewport = egui::ViewportBuilder::default()
        .with_title("Hadron — Goldleaf USB client (Rust port of Quark)")
        .with_inner_size([900.0, 400.0])
        .with_min_inner_size([640.0, 360.0]);
    let viewport = match load_icon() {
        Some(icon) => viewport.with_icon(icon),
        None => viewport,
    };

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Hadron",
        options,
        Box::new(move |_cc| Ok(Box::new(HadronApp::new(cfg, usb_rx)))),
    )
}