use crate::{dataset::DatasetSplit, DatasetCleanerApp};
use eframe::egui;

/// Handle keyboard shortcuts for navigation and deletion
pub fn handle_keyboard_shortcuts(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    use tracing::info;

    // Check if any text input is focused (to avoid triggering shortcuts while typing)
    let text_edit_focused = ctx.memory(|mem| mem.focused().is_some());

    // Escape key - Close dialogs/cancel operations (always handle, even with text focus)
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        info!("[KEYBOARD] Escape key pressed");

        // Priority order: filter dialog, batch confirmation, batch processing
        if app.show_filter_dialog {
            app.show_filter_dialog = false;
            info!("[KEYBOARD] Closed filter dialog");
        } else if app.show_batch_delete_confirm {
            app.show_batch_delete_confirm = false;
            info!("[KEYBOARD] Closed batch delete confirmation dialog");
        } else if app.batch_processing {
            app.cancel_batch_processing();
            info!("[KEYBOARD] Cancelled batch processing");
        } else if app.fullscreen_mode {
            app.fullscreen_mode = false;
            info!("[KEYBOARD] Exited fullscreen mode");
        }
        return; // Don't process other shortcuts when Escape is pressed
    }

    // Don't process other shortcuts if a text input is focused
    if text_edit_focused {
        return;
    }

    // Zoom shortcuts
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Num0)) {
        info!("[KEYBOARD] Ctrl+0 pressed - Reset zoom to 100%");
        app.zoom_level = 1.0;
        return;
    }

    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Equals)) {
        info!("[KEYBOARD] Ctrl+= pressed - Zoom in");
        app.zoom_level = (app.zoom_level + 0.1).clamp(0.5, 3.0);
        return;
    }

    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Minus)) {
        info!("[KEYBOARD] Ctrl+- pressed - Zoom out");
        app.zoom_level = (app.zoom_level - 0.1).clamp(0.5, 3.0);
        return;
    }

    // Navigation shortcuts with modifier keys
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
        info!("[KEYBOARD] Ctrl+O pressed - Open dataset dialog");
        // Trigger file picker dialog (will be handled in panels.rs)
        // For now, we'll use a simple flag or direct call
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            info!("[KEYBOARD] Selected dataset path: {:?}", path);
            app.load_dataset(path);
        }
        return;
    }

    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::F)) {
        info!("[KEYBOARD] Ctrl+F pressed - Open filter dialog");
        app.show_filter_dialog = true;
        return;
    }

    // Basic navigation shortcuts
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
        info!("[KEYBOARD] Right arrow pressed");
        app.next_image();
    }

    if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
        info!("[KEYBOARD] Left arrow pressed");
        app.prev_image();
    }

    // Jump to first/last image
    if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
        info!("[KEYBOARD] Home key pressed");
        app.jump_to_first();
    }

    if ctx.input(|i| i.key_pressed(egui::Key::End)) {
        info!("[KEYBOARD] End key pressed");
        app.jump_to_last();
    }

    // Page Up/Down - Jump by 10 images
    if ctx.input(|i| i.key_pressed(egui::Key::PageUp)) {
        info!("[KEYBOARD] Page Up pressed");
        app.jump_by_offset(-10);
    }

    if ctx.input(|i| i.key_pressed(egui::Key::PageDown)) {
        info!("[KEYBOARD] Page Down pressed");
        app.jump_by_offset(10);
    }

    // Space - Toggle fullscreen mode
    if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
        info!("[KEYBOARD] Space pressed");
        app.toggle_fullscreen();
    }

    // Delete current image
    if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
        info!("[KEYBOARD] Delete key pressed!");
        if !app.dataset.get_image_files().is_empty() {
            info!("[KEYBOARD] Dataset is not empty, calling delete_current_image()");
            app.delete_current_image();
        } else {
            info!("[KEYBOARD] Dataset is empty, not deleting");
        }
    }

    // Ctrl+Z - Undo delete
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
        info!("[KEYBOARD] Ctrl+Z pressed - Undo delete");
        if app.undo_manager.can_undo() {
            app.undo_delete();
        }
    }

    // Ctrl+Y - Redo delete (Windows standard)
    if ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Y)) {
        info!("[KEYBOARD] Ctrl+Y pressed - Redo delete");
        if app.undo_manager.can_redo() {
            app.redo_delete();
        }
    }

    // Ctrl+Shift+Z - Redo delete (cross-platform alternative)
    if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)) {
        info!("[KEYBOARD] Ctrl+Shift+Z pressed - Redo delete");
        if app.undo_manager.can_redo() {
            app.redo_delete();
        }
    }

    // Number keys 1, 2, 3 - Switch dataset splits
    if ctx.input(|i| i.key_pressed(egui::Key::Num1)) {
        info!("[KEYBOARD] Key 1 pressed - Switch to Train");
        app.change_split(DatasetSplit::Train);
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Num2)) {
        info!("[KEYBOARD] Key 2 pressed - Switch to Val");
        app.change_split(DatasetSplit::Val);
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Num3)) {
        info!("[KEYBOARD] Key 3 pressed - Switch to Test");
        app.change_split(DatasetSplit::Test);
    }
}
