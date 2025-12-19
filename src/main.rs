#![windows_subsystem = "windows"]

use eframe::egui;
use tracing::info;

mod app;
mod config;
mod core;
mod infrastructure;
mod navigation;
mod state;
mod ui;

use app::DatasetCleanerApp;
use state::Settings;

fn main() -> Result<(), eframe::Error> {
    // Setup logging
    infrastructure::logging::setup_logging();

    // Load settings to get window dimensions
    let settings = Settings::load();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([settings.window_width, settings.window_height])
            .with_title("YOLO Dataset Cleaner"),
        centered: true,
        ..Default::default()
    };

    info!("Launching application window");
    eframe::run_native(
        "YOLO Dataset Cleaner",
        options,
        Box::new(|cc| {
            // Initialize egui-phosphor
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(DatasetCleanerApp::default()))
        }),
    )
}
