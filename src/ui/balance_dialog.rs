use crate::app::DatasetCleanerApp;
use crate::core::analysis::{
    get_recommendations, ImageCategory, RebalanceConfig, SelectionStrategy, TargetRatios,
};
use crate::core::dataset::DatasetSplit;
use eframe::egui;

/// State for the balance dialog tabs
#[derive(Default, Clone, Copy, PartialEq)]
pub enum BalanceDialogTab {
    #[default]
    Balance,
    Integrity,
}

/// Render the balance analysis dialog
pub fn render_balance_dialog(app: &mut DatasetCleanerApp, ctx: &egui::Context) {
    if !app.balance.show_dialog {
        return;
    }

    let mut show_dialog = app.balance.show_dialog;
    let needs_repaint = app.balance.analyzing || app.integrity.analyzing;
    
    // Get screen center for initial position
    let screen_rect = ctx.screen_rect();
    let center = screen_rect.center();
    
    egui::Window::new("📊 Dataset Analysis")
        .open(&mut show_dialog)
        .collapsible(false)
        .resizable(true)
        .default_width(700.0)
        .default_height(550.0)
        .min_width(600.0)
        .min_height(500.0)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(center)
        .show(ctx, |ui| {
            // Use tracked min height (grows but never shrinks)
            ui.set_min_height(app.balance.tracked_min_height);
            
            // Tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut app.balance.current_tab,
                    0,
                    egui::RichText::new("📊 Balance Analysis").size(14.0),
                );
                ui.selectable_value(
                    &mut app.balance.current_tab,
                    1,
                    egui::RichText::new("🔍 Data Integrity").size(14.0),
                );
            });
            
            ui.separator();
            ui.add_space(10.0);

            match app.balance.current_tab {
                0 => render_balance_tab(app, ui),
                1 => render_integrity_tab(app, ui),
                _ => {}
            }
            
            // Update tracked height to current content height (only grows)
            let current_height = ui.min_rect().height();
            if current_height > app.balance.tracked_min_height {
                app.balance.tracked_min_height = current_height;
            }
        });

    // Update dialog visibility if X button was clicked
    if !show_dialog {
        app.balance.show_dialog = false;
        app.balance.results = None;
        app.rebalance.error_message = None;
    }

    // Request repaint only during active analysis
    if needs_repaint {
        ctx.request_repaint();
    }
}

/// Render the Balance Analysis tab
fn render_balance_tab(app: &mut DatasetCleanerApp, ui: &mut egui::Ui) {
    if app.balance.analyzing {
        render_analyzing_state(app, ui);
    } else if app.balance.results.is_some() {
        render_balance_results(app, ui);
    } else {
        // No analysis yet - show button to start
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label("Click the button below to analyze dataset balance.");
            ui.add_space(10.0);
            if ui.button("🔄 Start Balance Analysis").clicked() {
                app.analyze_balance();
            }
            ui.add_space(20.0);
        });
    }
}

/// Render analyzing state with progress bar
fn render_analyzing_state(app: &DatasetCleanerApp, ui: &mut egui::Ui) {
    ui.heading("Analyzing dataset...");
    ui.add_space(10.0);
    
    if app.balance.total_images > 0 {
        let progress = app.balance.current_progress as f32 / app.balance.total_images as f32;
        ui.add(egui::ProgressBar::new(progress).text(format!(
            "Analyzed {} / {} images",
            app.balance.current_progress,
            app.balance.total_images
        )));
    } else {
        ui.spinner();
    }
    
    ui.add_space(10.0);
    ui.label("Scanning images and categorizing by player type...");
    
    if let Some(stats) = &app.balance.results {
        ui.add_space(5.0);
        ui.label(egui::RichText::new("Current count:").size(12.0).italics());
        ui.label(format!("  • Player images: {}", stats.total_player_images()));
        ui.label(format!("  • Background: {}", stats.background));
    }
}

/// Render balance results with all sections
fn render_balance_results(app: &mut DatasetCleanerApp, ui: &mut egui::Ui) {
    // Extract data we need before entering closures
    let stats = match &app.balance.results {
        Some(s) => s.clone(),
        None => return,
    };
    
    let target_ratios = TargetRatios {
        player_ratio: app.config.target_player_ratio,
        background_ratio: app.config.target_background_ratio,
        hardcase_ratio: app.config.target_hardcase_ratio,
    };

    egui::ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
        // Current Distribution Section
        egui::CollapsingHeader::new(
            egui::RichText::new("📊 Current Distribution").strong().size(15.0)
        )
        .default_open(true)
        .show(ui, |ui| {
            render_distribution_section(ui, &stats);
        });

        ui.add_space(10.0);

        // Target Distribution Section
        egui::CollapsingHeader::new(
            egui::RichText::new("🎯 Target Distribution").strong().size(15.0)
        )
        .default_open(false)
        .show(ui, |ui| {
            render_target_section(ui, app);
        });

        ui.add_space(10.0);

        // Recommendations Section
        egui::CollapsingHeader::new(
            egui::RichText::new("💡 Recommendations").strong().size(15.0).color(egui::Color32::from_rgb(100, 150, 255))
        )
        .default_open(true)
        .show(ui, |ui| {
            let recommendations = get_recommendations(&stats, &target_ratios);
            for recommendation in recommendations {
                ui.label(recommendation);
            }
        });

        ui.add_space(10.0);

        // Auto-Rebalance Section
        let current_split = app.dataset.current_split();
        let error_message = app.rebalance.error_message.clone();
        
        let bg_excess = crate::core::analysis::calculate_move_count(
            &stats,
            ImageCategory::Background,
            &target_ratios,
        );
        let player_excess = crate::core::analysis::calculate_move_count(
            &stats,
            ImageCategory::CTOnly,
            &target_ratios,
        );

        let mut pending_config: Option<RebalanceConfig> = None;

        egui::CollapsingHeader::new(
            egui::RichText::new("🔄 Auto-Rebalance").strong().size(15.0).color(egui::Color32::from_rgb(100, 180, 255))
        )
        .default_open(true)
        .show(ui, |ui| {
            render_rebalance_section(
                ui,
                app,
                &stats,
                &target_ratios,
                current_split,
                bg_excess,
                player_excess,
                &mut pending_config,
            );

            if let Some(error) = &error_message {
                ui.add_space(5.0);
                ui.colored_label(egui::Color32::from_rgb(255, 200, 100), error);
            }
        });

        if let Some(config) = pending_config {
            app.calculate_rebalance_plan(config);
        }

        ui.add_space(10.0);

        // Global Balance Section
        egui::CollapsingHeader::new(
            egui::RichText::new("🌐 Global Auto-Balance").strong().size(15.0).color(egui::Color32::from_rgb(200, 150, 255))
        )
        .default_open(false)
        .show(ui, |ui| {
            render_global_balance_section(app, ui);
        });
    });
}

/// Render the distribution section
fn render_distribution_section(ui: &mut egui::Ui, stats: &crate::core::analysis::BalanceStats) {
    ui.label(format!("📂 Total Images: {}", stats.total_images));
    ui.add_space(5.0);

    let player_count = stats.total_player_images();
    let player_pct = stats.player_percentage();
    ui.label(
        egui::RichText::new(format!("👥 Player Images: {} ({:.1}%)", player_count, player_pct))
            .color(egui::Color32::from_rgb(100, 200, 100)),
    );

    ui.indent("player_breakdown", |ui| {
        let ct_count = stats.get_count(ImageCategory::CTOnly);
        let ct_pct = stats.get_percentage(ImageCategory::CTOnly);
        ui.label(format!("• CT Only: {} ({:.1}%)", ct_count, ct_pct));

        let t_count = stats.get_count(ImageCategory::TOnly);
        let t_pct = stats.get_percentage(ImageCategory::TOnly);
        ui.label(format!("• T Only: {} ({:.1}%)", t_count, t_pct));

        let multi_count = stats.get_count(ImageCategory::MultiplePlayer);
        let multi_pct = stats.get_percentage(ImageCategory::MultiplePlayer);
        ui.label(format!("• Multiple Players: {} ({:.1}%)", multi_count, multi_pct));
    });

    ui.add_space(5.0);

    let bg_count = stats.get_count(ImageCategory::Background);
    let bg_pct = stats.get_percentage(ImageCategory::Background);
    ui.label(
        egui::RichText::new(format!("🌄 Background Images: {} ({:.1}%)", bg_count, bg_pct))
            .color(egui::Color32::from_rgb(200, 150, 100)),
    );

    let hc_count = stats.get_count(ImageCategory::HardCase);
    if hc_count > 0 {
        let hc_pct = stats.get_percentage(ImageCategory::HardCase);
        ui.add_space(5.0);
        ui.label(
            egui::RichText::new(format!("⚠ Hard Cases: {} ({:.1}%)", hc_count, hc_pct))
                .color(egui::Color32::from_rgb(255, 200, 0)),
        );
    }
}

/// Render target distribution section
fn render_target_section(ui: &mut egui::Ui, app: &DatasetCleanerApp) {
    let target_player_pct = app.config.target_player_ratio * 100.0;
    let target_bg_pct = app.config.target_background_ratio * 100.0;
    let target_hc_pct = app.config.target_hardcase_ratio * 100.0;

    ui.label(format!("👥 Player Images: {:.0}%", target_player_pct));
    ui.label(format!("🌄 Background Images: {:.0}%", target_bg_pct));
    ui.label(format!("⚠ Hard Cases: {:.0}%", target_hc_pct));
}

/// Render auto-rebalance section
fn render_rebalance_section(
    ui: &mut egui::Ui,
    app: &DatasetCleanerApp,
    stats: &crate::core::analysis::BalanceStats,
    target_ratios: &TargetRatios,
    current_split: DatasetSplit,
    bg_excess: i32,
    player_excess: i32,
    pending_config: &mut Option<RebalanceConfig>,
) {
    ui.horizontal(|ui| {
        ui.label("Current split:");
        ui.label(
            egui::RichText::new(current_split.as_str().to_uppercase())
                .strong()
                .color(egui::Color32::from_rgb(100, 200, 100)),
        );
        ui.label(format!("({} images)", stats.total_images));
    });
    
    ui.add_space(5.0);
    
    // Comparison table
    let target_bg_pct = target_ratios.background_ratio * 100.0;
    let current_bg_pct = stats.get_percentage(ImageCategory::Background);
    let ideal_bg_count = (stats.total_images as f32 * target_ratios.background_ratio) as usize;
    
    let target_player_pct = target_ratios.player_ratio * 100.0;
    let current_player_pct = stats.player_percentage();
    let ideal_player_count = (stats.total_images as f32 * target_ratios.player_ratio) as usize;

    egui::Grid::new("rebalance_comparison")
        .num_columns(4)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            ui.label("");
            ui.label(egui::RichText::new("Current").size(11.0));
            ui.label(egui::RichText::new("Target").size(11.0));
            ui.label(egui::RichText::new("Excess").size(11.0));
            ui.end_row();
            
            ui.label("🌄 Background");
            ui.label(format!("{} ({:.1}%)", stats.background, current_bg_pct));
            ui.label(format!("{} ({:.0}%)", ideal_bg_count, target_bg_pct));
            render_excess_label(ui, bg_excess);
            ui.end_row();
            
            ui.label("👥 Players");
            ui.label(format!("{} ({:.1}%)", stats.total_player_images(), current_player_pct));
            ui.label(format!("{} ({:.0}%)", ideal_player_count, target_player_pct));
            render_excess_label(ui, player_excess);
            ui.end_row();
        });

    ui.add_space(5.0);

    // Action buttons - use cached best destinations or default to first available split
    
    if bg_excess > 0 {
        // Get best destination: cached value or first available split
        let (dest_split, to_move) = if let Some((best_dest, dest_needs)) = app.balance.cached_best_bg_dest {
            (best_dest, bg_excess.min(dest_needs) as usize)
        } else {
            // Default to first available split (not current)
            let default_dest = match current_split {
                DatasetSplit::Train => DatasetSplit::Val,
                DatasetSplit::Val => DatasetSplit::Train,
                DatasetSplit::Test => DatasetSplit::Train,
            };
            (default_dest, bg_excess as usize)
        };
        
        if ui.button(format!(
            "Move {} background → {}", 
            to_move, 
            dest_split.as_str().to_uppercase()
        )).clicked() {
            *pending_config = Some(RebalanceConfig {
                target_ratios: target_ratios.clone(),
                selection_strategy: SelectionStrategy::Random,
                preserve_ct_t_balance: true,
                source_split: current_split,
                destination_split: dest_split,
                category: ImageCategory::Background,
            });
        }
    }

    if player_excess > 0 {
        // Get best destination: cached value or first available split
        let (dest_split, to_move) = if let Some((best_dest, dest_needs)) = app.balance.cached_best_player_dest {
            (best_dest, player_excess.min(dest_needs) as usize)
        } else {
            // Default to first available split (not current)
            let default_dest = match current_split {
                DatasetSplit::Train => DatasetSplit::Val,
                DatasetSplit::Val => DatasetSplit::Train,
                DatasetSplit::Test => DatasetSplit::Train,
            };
            (default_dest, player_excess as usize)
        };
        
        if ui.button(format!(
            "Move {} players → {}", 
            to_move, 
            dest_split.as_str().to_uppercase()
        )).clicked() {
            *pending_config = Some(RebalanceConfig {
                target_ratios: target_ratios.clone(),
                selection_strategy: SelectionStrategy::Random,
                preserve_ct_t_balance: true,
                source_split: current_split,
                destination_split: dest_split,
                category: ImageCategory::CTOnly,
            });
        }
    }

    if bg_excess <= 0 && player_excess <= 0 {
        ui.colored_label(
            egui::Color32::from_rgb(100, 200, 100),
            "✓ No excess images - this split is balanced!",
        );
    }
}

/// Render excess label with appropriate color
fn render_excess_label(ui: &mut egui::Ui, excess: i32) {
    if excess > 0 {
        ui.label(
            egui::RichText::new(format!("+{}", excess))
                .color(egui::Color32::from_rgb(255, 150, 100)),
        );
    } else if excess < 0 {
        ui.label(
            egui::RichText::new(format!("{}", excess))
                .color(egui::Color32::from_rgb(100, 150, 255)),
        );
    } else {
        ui.label(
            egui::RichText::new("✓")
                .color(egui::Color32::from_rgb(100, 200, 100)),
        );
    }
}

/// Render global balance section
fn render_global_balance_section(app: &mut DatasetCleanerApp, ui: &mut egui::Ui) {
    ui.label("Automatically balance your dataset across all splits:");
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Target:").size(10.0).color(egui::Color32::GRAY));
        ui.label(egui::RichText::new("Train 70%").size(10.0).color(egui::Color32::from_rgb(100, 200, 255)));
        ui.label(egui::RichText::new("/ Val 20%").size(10.0).color(egui::Color32::from_rgb(100, 255, 100)));
        ui.label(egui::RichText::new("/ Test 10%").size(10.0).color(egui::Color32::from_rgb(255, 200, 100)));
    });
    
    ui.add_space(5.0);

    if ui.button("🔄 Balance All Splits").clicked() {
        app.calculate_global_rebalance();
    }
}

// =============================================================================
// DATA INTEGRITY TAB
// =============================================================================

/// Render the Data Integrity tab
fn render_integrity_tab(app: &mut DatasetCleanerApp, ui: &mut egui::Ui) {
    if app.integrity.analyzing {
        render_integrity_analyzing(app, ui);
    } else if app.integrity.results.is_some() {
        render_integrity_results(app, ui);
    } else {
        render_integrity_start(app, ui);
    }
}

/// Render start screen for integrity analysis
fn render_integrity_start(app: &mut DatasetCleanerApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        
        ui.label(
            egui::RichText::new("🔍 Data Integrity Check")
                .size(18.0)
                .strong()
        );
        
        ui.add_space(10.0);
        ui.label("Scan for orphaned files in your dataset:");
        ui.add_space(5.0);
        
        ui.horizontal(|ui| {
            ui.add_space(50.0);
            ui.vertical(|ui| {
                ui.label("• Images without corresponding label files");
                ui.label("• Label files without corresponding images");
            });
        });
        
        ui.add_space(15.0);
        
        if ui.button(egui::RichText::new("🔄 Analyze Integrity").size(14.0)).clicked() {
            app.analyze_integrity();
        }
        
        ui.add_space(20.0);
    });
}

/// Render integrity analyzing state
fn render_integrity_analyzing(app: &DatasetCleanerApp, ui: &mut egui::Ui) {
    ui.heading("Scanning files...");
    ui.add_space(10.0);
    
    if app.integrity.total_files > 0 {
        let progress = app.integrity.current_progress as f32 / app.integrity.total_files as f32;
        ui.add(egui::ProgressBar::new(progress).text(format!(
            "Scanned {} / {} files",
            app.integrity.current_progress,
            app.integrity.total_files
        )));
    } else {
        ui.spinner();
    }
    
    ui.add_space(10.0);
    
    if let Some(stats) = &app.integrity.results {
        ui.label(format!(
            "Found: {} images without labels, {} labels without images",
            stats.images_without_labels.len(),
            stats.labels_without_images.len()
        ));
    }
}

/// Render integrity results
fn render_integrity_results(app: &mut DatasetCleanerApp, ui: &mut egui::Ui) {
    // Extract counts upfront to avoid borrowing issues
    let (img_count, lbl_count, total_issues) = match &app.integrity.results {
        Some(stats) => (
            stats.images_without_labels.len(),
            stats.labels_without_images.len(),
            stats.total_issues(),
        ),
        None => return,
    };
    
    // Summary cards
    ui.horizontal(|ui| {
        // Images without labels card
        let img_color = if img_count == 0 {
            egui::Color32::from_rgb(100, 200, 100)
        } else {
            egui::Color32::from_rgb(255, 150, 100)
        };
        
        ui.group(|ui| {
            ui.set_min_width(200.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("🖼️ Images Without Labels").strong());
                ui.label(
                    egui::RichText::new(format!("{}", img_count))
                        .size(28.0)
                        .color(img_color)
                );
            });
        });

        ui.add_space(10.0);

        // Labels without images card
        let lbl_color = if lbl_count == 0 {
            egui::Color32::from_rgb(100, 200, 100)
        } else {
            egui::Color32::from_rgb(255, 150, 100)
        };
        
        ui.group(|ui| {
            ui.set_min_width(200.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("📝 Labels Without Images").strong());
                ui.label(
                    egui::RichText::new(format!("{}", lbl_count))
                        .size(28.0)
                        .color(lbl_color)
                );
            });
        });
    });

    ui.add_space(10.0);

    if total_issues == 0 {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("✓ No integrity issues found!")
                    .size(16.0)
                    .color(egui::Color32::from_rgb(100, 200, 100))
            );
            ui.add_space(10.0);
            if ui.button("🔄 Re-analyze").clicked() {
                app.analyze_integrity();
            }
            ui.add_space(20.0);
        });
        return;
    }

    // Sub-tabs for issue types
    ui.horizontal(|ui| {
        if ui.selectable_label(
            app.integrity.current_tab == 0,
            format!("🖼️ Images ({}) ", img_count)
        ).clicked() {
            app.integrity.current_tab = 0;
        }
        if ui.selectable_label(
            app.integrity.current_tab == 1,
            format!("📝 Labels ({}) ", lbl_count)
        ).clicked() {
            app.integrity.current_tab = 1;
        }
    });

    ui.separator();

    // Issue list - we need to access the actual vectors via app.integrity.results
    if let Some(ref results) = app.integrity.results {
        let images_issues = &results.images_without_labels;
        let labels_issues = &results.labels_without_images;
        
        egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
            match app.integrity.current_tab {
                0 => {
                    if images_issues.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(
                                egui::RichText::new("✓ No images without labels")
                                    .color(egui::Color32::from_rgb(100, 200, 100))
                            );
                            ui.add_space(20.0);
                        });
                    } else {
                        for (idx, issue) in images_issues.iter().enumerate() {
                            let mut is_selected = app.integrity.selected_images_without_labels.contains(&idx);
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut is_selected, "").clicked() {
                                    if is_selected {
                                        app.integrity.selected_images_without_labels.insert(idx);
                                    } else {
                                        app.integrity.selected_images_without_labels.remove(&idx);
                                    }
                                }
                                if let Some(filename) = issue.path.file_name() {
                                    ui.label(filename.to_string_lossy().as_ref());
                                } else {
                                    ui.label(issue.path.display().to_string());
                                }
                            });
                        }
                    }
                }
                1 => {
                    if labels_issues.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(
                                egui::RichText::new("✓ No labels without images")
                                    .color(egui::Color32::from_rgb(100, 200, 100))
                            );
                            ui.add_space(20.0);
                        });
                    } else {
                        for (idx, issue) in labels_issues.iter().enumerate() {
                            let mut is_selected = app.integrity.selected_labels_without_images.contains(&idx);
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut is_selected, "").clicked() {
                                    if is_selected {
                                        app.integrity.selected_labels_without_images.insert(idx);
                                    } else {
                                        app.integrity.selected_labels_without_images.remove(&idx);
                                    }
                                }
                                if let Some(filename) = issue.path.file_name() {
                                    ui.label(filename.to_string_lossy().as_ref());
                                } else {
                                    ui.label(issue.path.display().to_string());
                                }
                            });
                        }
                    }
                }
                _ => {}
            }
        });
    }

    ui.add_space(10.0);

    // Action buttons
    let selection_count = app.integrity.selection_count();
    let current_issues = match app.integrity.current_tab {
        0 => img_count,
        1 => lbl_count,
        _ => 0,
    };

    ui.horizontal(|ui| {
        // Select All / Deselect All
        if current_issues > 0 {
            if selection_count < current_issues {
                if ui.button("☑ Select All").clicked() {
                    match app.integrity.current_tab {
                        0 => {
                            for i in 0..img_count {
                                app.integrity.selected_images_without_labels.insert(i);
                            }
                        }
                        1 => {
                            for i in 0..lbl_count {
                                app.integrity.selected_labels_without_images.insert(i);
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                if ui.button("☐ Deselect All").clicked() {
                    match app.integrity.current_tab {
                        0 => app.integrity.selected_images_without_labels.clear(),
                        1 => app.integrity.selected_labels_without_images.clear(),
                        _ => {}
                    }
                }
            }
        }

        ui.add_space(20.0);

        // Delete Selected button
        let delete_enabled = selection_count > 0;
        if ui.add_enabled(
            delete_enabled,
            egui::Button::new(format!("🗑️ Delete Selected ({})", selection_count))
        ).clicked() {
            app.delete_selected_integrity_issues();
        }

        // Delete All button
        if total_issues > 0 {
            ui.add_space(10.0);
            if ui.button(
                egui::RichText::new(format!("⚠️ Delete All ({})", total_issues))
                    .color(egui::Color32::from_rgb(255, 100, 100))
            ).clicked() {
                app.delete_all_integrity_issues();
            }
        }
    });

    // Error message
    if let Some(error) = &app.integrity.error_message {
        ui.add_space(5.0);
        ui.colored_label(egui::Color32::from_rgb(255, 150, 100), error);
    }

    ui.add_space(5.0);
    
    // Re-analyze button
    if ui.button("🔄 Re-analyze").clicked() {
        app.analyze_integrity();
    }
}

/// Render a list of integrity issues with checkboxes
fn render_issue_list(
    ui: &mut egui::Ui,
    issues: &[crate::core::analysis::IntegrityIssue],
    selected: &mut std::collections::HashSet<usize>,
    empty_message: &str,
) {
    if issues.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new(format!("✓ {}", empty_message))
                    .color(egui::Color32::from_rgb(100, 200, 100))
            );
            ui.add_space(20.0);
        });
        return;
    }

    for (idx, issue) in issues.iter().enumerate() {
        let mut is_selected = selected.contains(&idx);
        
        ui.horizontal(|ui| {
            if ui.checkbox(&mut is_selected, "").clicked() {
                if is_selected {
                    selected.insert(idx);
                } else {
                    selected.remove(&idx);
                }
            }
            
            if let Some(filename) = issue.path.file_name() {
                ui.label(filename.to_string_lossy().as_ref());
            } else {
                ui.label(issue.path.display().to_string());
            }
        });
    }
}
