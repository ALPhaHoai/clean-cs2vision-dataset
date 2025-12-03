use crate::app::DatasetCleanerApp;
use crate::core::analysis::{get_recommendations, ImageCategory, TargetRatios};
use eframe::egui;

/// Render the balance analysis dialog
pub fn render_balance_dialog(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    if !app.balance.show_dialog {
        return;
    }

    egui::Window::new("📊 Dataset Balance Analysis")
        .collapsible(false)
        .resizable(true)
        .default_width(600.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if app.balance.analyzing {
                ui.heading("Analyzing dataset...");
                ui.add_space(10.0);
                ui.spinner();
                ui.add_space(10.0);
                ui.label("Scanning images and categorizing by player type...");
            } else if let Some(stats) = &app.balance.results {
                ui.heading("Balance Analysis Results");
                ui.add_space(10.0);

                // Current Distribution Section
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("CURRENT DISTRIBUTION")
                            .strong()
                            .size(16.0),
                    );
                    ui.add_space(5.0);

                    ui.label(format!("📂 Total Images: {}", stats.total_images));
                    ui.add_space(10.0);

                    // Player images breakdown
                    let player_count = stats.total_player_images();
                    let player_pct = stats.player_percentage();
                    ui.label(
                        egui::RichText::new(format!(
                            "👥 Player Images: {} ({:.1}%)",
                            player_count, player_pct
                        ))
                        .color(egui::Color32::from_rgb(100, 200, 100)),
                    );

                    ui.indent("player_breakdown", |ui| {
                        let ct_count = stats.get_count(ImageCategory::CTOnly);
                        let ct_pct = stats.get_percentage(ImageCategory::CTOnly);
                        ui.label(format!("  • CT Only: {} ({:.1}%)", ct_count, ct_pct));

                        let t_count = stats.get_count(ImageCategory::TOnly);
                        let t_pct = stats.get_percentage(ImageCategory::TOnly);
                        ui.label(format!("  • T Only: {} ({:.1}%)", t_count, t_pct));

                        let multi_count = stats.get_count(ImageCategory::MultiplePlayer);
                        let multi_pct = stats.get_percentage(ImageCategory::MultiplePlayer);
                        ui.label(format!(
                            "  • Multiple Players: {} ({:.1}%)",
                            multi_count, multi_pct
                        ));
                    });

                    ui.add_space(5.0);

                    // Background images
                    let bg_count = stats.get_count(ImageCategory::Background);
                    let bg_pct = stats.get_percentage(ImageCategory::Background);
                    ui.label(
                        egui::RichText::new(format!(
                            "🌄 Background Images: {} ({:.1}%)",
                            bg_count, bg_pct
                        ))
                        .color(egui::Color32::from_rgb(200, 150, 100)),
                    );

                    // Hard cases (if any)
                    let hc_count = stats.get_count(ImageCategory::HardCase);
                    if hc_count > 0 {
                        let hc_pct = stats.get_percentage(ImageCategory::HardCase);
                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "⚠ Hard Cases: {} ({:.1}%)",
                                hc_count, hc_pct
                            ))
                            .color(egui::Color32::from_rgb(255, 200, 0)),
                        );
                    }
                });

                ui.add_space(15.0);

                // Target Distribution Section
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("TARGET DISTRIBUTION")
                            .strong()
                            .size(16.0),
                    );
                    ui.add_space(5.0);

                    let target_player_pct = app.config.target_player_ratio * 100.0;
                    let target_bg_pct = app.config.target_background_ratio * 100.0;
                    let target_hc_pct = app.config.target_hardcase_ratio * 100.0;

                    ui.label(format!("👥 Player Images: {:.0}%", target_player_pct));
                    ui.label(format!("🌄 Background Images: {:.0}%", target_bg_pct));
                    ui.label(format!("⚠ Hard Cases: {:.0}%", target_hc_pct));

                    ui.add_space(5.0);

                    // Example breakdown for 10,000 images
                    ui.label(
                        egui::RichText::new("Example for 10,000 images:")
                            .italics()
                            .size(12.0),
                    );
                    ui.indent("example", |ui| {
                        ui.label("  • 8,500 images (85%): With players");
                        ui.label("    - ~3,800 CT only");
                        ui.label("    - ~3,800 T only");
                        ui.label("    - ~900 Multiple players");
                        ui.label("  • 1,000 images (10%): Background");
                        ui.label("  • 500 images (5%): Hard cases");
                    });
                });

                ui.add_space(15.0);

                // Recommendations Section
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("RECOMMENDATIONS")
                            .strong()
                            .size(16.0)
                            .color(egui::Color32::from_rgb(100, 150, 255)),
                    );
                    ui.add_space(5.0);

                    let target_ratios = TargetRatios {
                        player_ratio: app.config.target_player_ratio,
                        background_ratio: app.config.target_background_ratio,
                        hardcase_ratio: app.config.target_hardcase_ratio,
                    };

                    let recommendations = get_recommendations(stats, &target_ratios);

                    for recommendation in recommendations {
                        ui.label(recommendation);
                    }

                    ui.add_space(10.0);

                    ui.colored_label(
                        egui::Color32::from_rgb(200, 200, 200),
                        "💡 These are suggestions for manual balancing. Review and adjust your dataset accordingly.",
                    );
                });

                ui.add_space(10.0);

                // Close button
                ui.horizontal(|ui| {
                    if ui.button("✓ Close").clicked() {
                        app.balance.show_dialog = false;
                        app.balance.results = None;
                    }
                });
            } else {
                ui.label("No analysis results available.");
                ui.add_space(10.0);

                if ui.button("Close").clicked() {
                    app.balance.show_dialog = false;
                }
            }
        });

    // Request repaint if analyzing
    if app.balance.analyzing {
        ctx.request_repaint();
    }
}
