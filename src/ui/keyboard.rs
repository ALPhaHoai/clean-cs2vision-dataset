use crate::DatasetCleanerApp;
use eframe::egui;

/// Handle keyboard shortcuts for navigation and deletion
pub fn handle_keyboard_shortcuts(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
        app.next_image();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
        app.prev_image();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
        if !app.dataset.get_image_files().is_empty() {
            app.delete_current_image();
        }
    }
}
