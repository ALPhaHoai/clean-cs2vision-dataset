// Additional UI functions for batch processing

use crate::app::DatasetCleanerApp;
use eframe::egui;

/// Render the batch delete confirmation dialog
pub fn render_batch_delete_confirmation(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    if app.ui.show_batch_delete_confirm {
        egui::Window::new("✨ Remove Black Images")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("This will scan all images in the current split and delete images with");
                ui.label("black or near-black dominant colors (RGB < 10).");
                ui.add_space(10.0);

                ui.label(format!("Current split: {:?}", app.dataset.current_split()));
                ui.label(format!(
                    "Total images: {}",
                    app.dataset.get_image_files().len()
                ));

                ui.add_space(10.0);

                ui.colored_label(
                    egui::Color32::from_rgb(255, 150, 0),
                    "⚠ Warning: This action cannot be undone!",
                );

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("✓ Yes, Scan & Delete").clicked() {
                        app.ui.show_batch_delete_confirm = false;
                        app.process_black_images();
                    }

                    if ui.button("✗ Cancel").clicked() {
                        app.ui.show_batch_delete_confirm = false;
                    }
                });
            });
    }
}

/// Render the batch processing progress/results dialog
pub fn render_batch_progress(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    if app.batch.processing || (app.batch.stats.is_some() && !app.ui.show_batch_delete_confirm) {
        egui::Window::new(if app.batch.processing {
            "⏳ Processing..."
        } else {
            "✓ Processing Complete"
        })
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some(stats) = &app.batch.stats {
                if app.batch.processing {
                    ui.label(format!(
                        "Scanning images: {}/{}",
                        stats.current_progress,
                        stats.total_scanned.max(stats.current_progress)
                    ));
                    ui.add_space(5.0);
                    ui.label(format!("Images deleted so far: {}", stats.total_deleted));
                    ui.add_space(10.0);
                    ui.spinner();
                } else {
                    ui.heading("Scan Complete!");
                    ui.add_space(10.0);

                    ui.label(format!("📊 Total images scanned: {}", stats.total_scanned));
                    ui.label(format!("🗑 Images deleted: {}", stats.total_deleted));

                    let retention_rate = if stats.total_scanned > 0 {
                        ((stats.total_scanned - stats.total_deleted) as f32
                            / stats.total_scanned as f32)
                            * 100.0
                    } else {
                        0.0
                    };

                    ui.label(format!("✓ Retention rate: {:.1}%", retention_rate));

                    ui.add_space(10.0);

                    if ui.button("Close").clicked() {
                        app.batch.stats = None;
                    }
                }
            }
        });

        // Request repaint to update progress
        if app.batch.processing {
            ctx.request_repaint();
        }
    }
}
