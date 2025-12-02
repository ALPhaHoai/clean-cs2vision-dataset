use crate::DatasetCleanerApp;
use eframe::egui;
use std::time::Duration;

const UNDO_DURATION_SECS: u64 = 10;

/// Render the toast notification for undo delete
pub fn render_toast_notification(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    if let Some(undo_state) = &app.undo_state {
        let elapsed = undo_state.deleted_at.elapsed();
        let remaining = Duration::from_secs(UNDO_DURATION_SECS)
            .saturating_sub(elapsed);
        
        let remaining_secs = remaining.as_secs();
        
        // Auto-finalize delete if time expired
        if remaining_secs == 0 && remaining.as_millis() == 0 {
            app.finalize_delete();
            return;
        }
        
        // Show toast in bottom-left corner
        egui::Window::new("deletion_toast")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_pos(egui::pos2(20.0, ctx.screen_rect().height() - 120.0))
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                
                // Style with background color
                let frame = egui::Frame::none()
                    .fill(egui::Color32::from_rgb(45, 45, 48))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)))
                    .rounding(6.0)
                    .inner_margin(12.0);
                
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🗑").size(20.0));
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Deleted: {}", undo_state.image_filename))
                                    .strong()
                                    .color(egui::Color32::WHITE)
                            );
                            ui.label(
                                egui::RichText::new(format!("Undo available for {} seconds", remaining_secs))
                                    .small()
                                    .color(egui::Color32::from_rgb(180, 180, 180))
                            );
                        });
                    });
                    
                    ui.add_space(8.0);
                    
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("↶ Undo")
                                .color(egui::Color32::WHITE)
                        )
                        .fill(egui::Color32::from_rgb(70, 130, 220))
                    )
                    .clicked() {
                        app.undo_delete();
                    }
                });
            });
        
        // Request repaint to update timer
        ctx.request_repaint();
    }
}

