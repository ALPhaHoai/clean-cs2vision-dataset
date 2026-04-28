use crate::app::DatasetCleanerApp;
use eframe::egui;
use egui_phosphor::regular as Icon;

/// Render the bottom panel with navigation controls
pub fn render_bottom_panel(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            
            // Navigation buttons removed (moved to image overlay)
            /*
            if ui.add_enabled(app.current_index > 0, egui::Button::new("◄ Previous"))
                .clicked()
            {
                app.prev_image();
            }
            
            if ui.add_enabled(
                !app.dataset.get_image_files().is_empty() && app.current_index < app.dataset.get_image_files().len() - 1,
                egui::Button::new("Next ►"),
            )
            .clicked()
            {
                app.next_image();
            }
            */
            
            ui.add_space(20.0);
            
            // Delete button
            let delete_enabled = !app.dataset.get_image_files().is_empty();
            let delete_btn = ui.add_enabled(
                delete_enabled,
                egui::Button::new(format!("{} Delete Image & Label", Icon::TRASH)).fill(egui::Color32::from_rgb(200, 50, 50)),
            );
            
            if delete_btn.clicked() {
                tracing::info!("[BUTTON] Delete button clicked!");
                app.delete_current_image();
            } else if delete_btn.hovered()
            && ui.input(|i| i.pointer.any_click()) {
                 tracing::warn!("[BUTTON] Delete button HOVERED and CLICKED (raw), but .clicked() is FALSE. Enabled: {}", delete_enabled);
            }
            
            ui.add_space(20.0);
            
            // Batch delete black images button
            let button_text = if app.batch.processing {
                if let Some(stats) = &app.batch.stats {
                    let total = stats.total_scanned.max(stats.current_progress);
                    let percentage = if total > 0 {
                        (stats.current_progress as f32 / total as f32 * 100.0) as u32
                    } else {
                        0
                    };
                    format!("{} Processing... {}%", Icon::MAGIC_WAND, percentage)
                } else {
                    format!("{} Processing...", Icon::MAGIC_WAND)
                }
            } else {
                format!("{} Remove Black Images", Icon::MAGIC_WAND)
            };
            
            let button = egui::Button::new(&button_text).fill(egui::Color32::from_rgb(100, 100, 180));
            if ui.add_enabled(
                !app.dataset.get_image_files().is_empty() && !app.batch.processing,
                button,
            )
            .clicked()
            {
                app.ui.show_batch_delete_confirm = true;
            }
            
            // Cancel button (only visible during batch processing)
            if app.batch.processing
                && ui.button("❌ Cancel").clicked() {
                    app.cancel_batch_processing();
                }
            
            ui.add_space(20.0);

            // Balance analyzer button
            let balance_btn_text = if app.balance.analyzing {
                format!("{} Analyzing...", Icon::CHART_BAR)
            } else {
                format!("{} Analyze Balance", Icon::CHART_BAR)
            };
            
            let balance_button = egui::Button::new(&balance_btn_text).fill(egui::Color32::from_rgb(100, 150, 100));
            if ui.add_enabled(
                !app.dataset.get_image_files().is_empty() && !app.balance.analyzing,
                balance_button,
            )
            .clicked()
            {
                app.analyze_balance();
            }
            
            ui.add_space(20.0);

            
            // Current file name
            if !app.dataset.get_image_files().is_empty() {
                if let Some(filename) = app.dataset.get_image_files()[app.current_index].file_name() {
                    ui.label(format!("{} {}", Icon::FILE, filename.to_string_lossy()));
                }
            }
        });
        ui.add_space(10.0);
    });
}
