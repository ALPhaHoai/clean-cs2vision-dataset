use crate::DatasetCleanerApp;
use eframe::egui;

/// Handle keyboard shortcuts for navigation and deletion
pub fn handle_keyboard_shortcuts(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    use tracing::info;
    
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
        info!("[KEYBOARD] Right arrow pressed");
        app.next_image();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
        info!("[KEYBOARD] Left arrow pressed");
        app.prev_image();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
        info!("[KEYBOARD] Delete key pressed!");
        if !app.dataset.get_image_files().is_empty() {
            info!("[KEYBOARD] Dataset is not empty, calling delete_current_image()");
            app.delete_current_image();
        } else {
            info!("[KEYBOARD] Dataset is empty, not deleting");
        }
    }
}
