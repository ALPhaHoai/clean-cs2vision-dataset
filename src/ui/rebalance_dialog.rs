//! Rebalance dialog for previewing and executing dataset rebalancing.

use crate::app::DatasetCleanerApp;
use crate::core::analysis::{
    ImageCategory, RebalanceConfig, SelectionStrategy, TargetRatios,
};
use crate::core::dataset::DatasetSplit;
use eframe::egui;

/// Render the rebalance dialog (preview, progress, or results)
pub fn render_rebalance_dialog(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    // Show preview dialog
    if app.rebalance.show_preview {
        render_preview_dialog(app, ctx);
    }

    // Show progress during execution
    if app.rebalance.is_active {
        render_progress_dialog(app, ctx);
    }

    // Show results after completion
    if app.rebalance.show_result {
        render_result_dialog(app, ctx);
    }
}

/// Render the preview dialog showing what will be moved
fn render_preview_dialog(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    let mut should_execute = false;
    let mut should_close = false;
    let is_global = app.rebalance.is_global;

    let title = if is_global { "🌐 Global Rebalance Preview" } else { "📦 Rebalance Preview" };

    egui::Window::new(title)
        .collapsible(false)
        .resizable(true)
        .default_width(if is_global { 600.0 } else { 500.0 })
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if is_global {
                // Global plan preview
                if let Some(plan) = &app.rebalance.global_plan {
                    ui.heading("Global Multi-Split Optimization");
                    ui.add_space(10.0);

                    // Move summary
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("MOVE SUMMARY").strong().size(14.0));
                        ui.add_space(5.0);
                        ui.label(format!("Total files to move: {}", plan.total_moves));
                        ui.label(format!("Move groups: {} (iterations: {})", plan.moves.len(), plan.iterations_used));
                        ui.add_space(5.0);
                        for move_group in &plan.moves {
                            ui.label(format!(
                                "  {} → {}: {} {} images",
                                move_group.from_split.as_str().to_uppercase(),
                                move_group.to_split.as_str().to_uppercase(),
                                move_group.count,
                                move_group.category.as_str()
                            ));
                        }
                    });

                    // Projected stats
                    if let (Some(current), Some(projected)) = (&plan.current_stats, &plan.projected_stats) {
                        ui.add_space(10.0);
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("BEFORE → AFTER").strong().size(14.0));
                            ui.add_space(5.0);
                            for split in [DatasetSplit::Train, DatasetSplit::Val, DatasetSplit::Test] {
                                let cur = current.get(split);
                                let proj = projected.get(split);
                                ui.label(format!(
                                    "{}: BG {:.1}%→{:.1}%, Player {:.1}%→{:.1}%",
                                    split.as_str().to_uppercase(),
                                    cur.get_percentage(ImageCategory::Background),
                                    proj.get_percentage(ImageCategory::Background),
                                    cur.player_percentage(),
                                    proj.player_percentage()
                                ));
                            }
                        });
                    }

                    ui.add_space(15.0);
                    ui.colored_label(egui::Color32::from_rgb(255, 200, 100), "⚠️ Files will be physically moved. This can be undone.");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new("✓ Execute").color(egui::Color32::GREEN)).clicked() {
                            should_execute = true;
                        }
                        if ui.button("❌ Cancel").clicked() {
                            should_close = true;
                        }
                    });
                } else {
                    ui.label("No global plan available.");
                    if ui.button("Close").clicked() {
                        should_close = true;
                    }
                }
            } else {
                // Single-split plan preview
                if let Some(plan) = &app.rebalance.plan {
                    ui.heading("Proposed Changes");
                    ui.add_space(10.0);

                    ui.group(|ui| {
                        ui.label(egui::RichText::new("MOVE SUMMARY").strong().size(14.0));
                        ui.add_space(5.0);
                        let from = plan.from_split.map(|s| s.as_str().to_uppercase()).unwrap_or_else(|| "?".to_string());
                        let to = plan.to_split.map(|s| s.as_str().to_uppercase()).unwrap_or_else(|| "?".to_string());
                        let cat = plan.category.map(|c| c.as_str().to_string()).unwrap_or_else(|| "?".to_string());
                        ui.label(format!("Move {} {} images", plan.len(), cat));
                        ui.label(format!("From: {} → To: {}", from, to));
                    });

                    if let (Some(current), Some(projected)) = (&plan.current_stats, &plan.projected_stats) {
                        ui.add_space(10.0);
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("BEFORE → AFTER").strong().size(14.0));
                            ui.add_space(5.0);
                            ui.label(format!(
                                "Players: {} ({:.1}%) → {} ({:.1}%)",
                                current.total_player_images(), current.player_percentage(),
                                projected.total_player_images(), projected.player_percentage()
                            ));
                            ui.label(format!(
                                "Background: {} ({:.1}%) → {} ({:.1}%)",
                                current.background, current.get_percentage(ImageCategory::Background),
                                projected.background, projected.get_percentage(ImageCategory::Background)
                            ));
                        });
                    }

                    ui.add_space(15.0);
                    ui.colored_label(egui::Color32::from_rgb(255, 200, 100), "⚠️ Files will be physically moved. This can be undone.");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new("✓ Execute").color(egui::Color32::GREEN)).clicked() {
                            should_execute = true;
                        }
                        if ui.button("❌ Cancel").clicked() {
                            should_close = true;
                        }
                    });
                } else {
                    ui.label("No rebalance plan available.");
                    if ui.button("Close").clicked() {
                        should_close = true;
                    }
                }
            }
        });

    if should_execute {
        if is_global {
            app.execute_global_rebalance();
        } else {
            app.execute_rebalance();
        }
    }

    if should_close {
        app.close_rebalance();
    }
}

/// Render progress dialog during execution
fn render_progress_dialog(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    egui::Window::new("🔄 Rebalancing...")
        .collapsible(false)
        .resizable(false)
        .default_width(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading("Moving images...");
            ui.add_space(10.0);

            // Progress bar
            if let Some((current, total)) = app.rebalance.progress {
                let progress = if total > 0 {
                    current as f32 / total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(progress).text(format!(
                    "{} / {} images moved",
                    current, total
                )));
            } else {
                ui.spinner();
            }

            // Last moved file
            if let Some(last) = &app.rebalance.last_moved {
                ui.add_space(5.0);
                ui.label(format!("Last: {}", last));
            }

            ui.add_space(10.0);

            // Cancel button
            if ui.button("❌ Cancel").clicked() {
                app.cancel_rebalance();
            }
        });

    // Request repaint for animation
    ctx.request_repaint();
}

/// Render result dialog after completion
fn render_result_dialog(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    let mut should_close = false;
    let mut should_undo = false;

    egui::Window::new("✅ Rebalance Complete")
        .collapsible(false)
        .resizable(false)
        .default_width(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some(results) = &app.rebalance.last_results {
                let success_count = results.iter().filter(|r| r.success).count();
                let failed_count = results.iter().filter(|r| !r.success).count();

                ui.heading("Rebalance Completed");
                ui.add_space(10.0);

                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("RESULTS")
                            .strong()
                            .size(14.0),
                    );
                    ui.add_space(5.0);

                    ui.colored_label(
                        egui::Color32::from_rgb(100, 200, 100),
                        format!("✓ Successfully moved: {} images", success_count),
                    );

                    if failed_count > 0 {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 100, 100),
                            format!("✗ Failed: {} images", failed_count),
                        );
                    }
                });

                ui.add_space(10.0);

                ui.label("💡 The dataset has been reloaded with the new structure.");

                ui.add_space(15.0);

                // Action buttons
                ui.horizontal(|ui| {
                    if app.rebalance.can_undo() {
                        if ui.button("↩ Undo All").clicked() {
                            should_undo = true;
                        }
                    }

                    if ui.button("✓ Done").clicked() {
                        should_close = true;
                    }
                });
            } else {
                ui.label("No results available.");
                if ui.button("Close").clicked() {
                    should_close = true;
                }
            }

            // Show error if any
            if let Some(error) = &app.rebalance.error_message {
                ui.add_space(10.0);
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    format!("Error: {}", error),
                );
            }
        });

    if should_undo {
        app.undo_rebalance();
        app.rebalance.show_result = false;
    }

    if should_close {
        // Clear results when closing (can't undo anymore)
        app.rebalance.last_results = None;
        app.rebalance.show_result = false;
    }
}

/// Configuration UI component that can be embedded in the balance dialog
pub fn render_rebalance_config(
    ui: &mut egui::Ui,
    current_split: DatasetSplit,
) -> Option<RebalanceConfig> {
    let mut config: Option<RebalanceConfig> = None;

    ui.group(|ui| {
        ui.label(
            egui::RichText::new("🔄 AUTO-REBALANCE")
                .strong()
                .size(14.0)
                .color(egui::Color32::from_rgb(100, 180, 255)),
        );
        ui.add_space(5.0);

        ui.label("Move excess images to another split:");
        ui.add_space(5.0);

        // Destination split selection
        ui.horizontal(|ui| {
            ui.label("Move to:");
            
            // Show options excluding current split
            let destinations: Vec<DatasetSplit> = 
                [DatasetSplit::Train, DatasetSplit::Val, DatasetSplit::Test]
                    .into_iter()
                    .filter(|s| *s != current_split)
                    .collect();

            for dest in destinations {
                if ui.button(dest.as_str()).clicked() {
                    // Create config for moving Background images (most common imbalance)
                    config = Some(RebalanceConfig {
                        target_ratios: TargetRatios::default(),
                        selection_strategy: SelectionStrategy::Random,
                        preserve_ct_t_balance: true,
                        source_split: current_split,
                        destination_split: dest,
                        category: ImageCategory::Background,
                    });
                }
            }
        });

        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("Moves excess background images to balance the dataset.")
                .size(11.0)
                .italics()
                .color(egui::Color32::GRAY),
        );
    });

    config
}
