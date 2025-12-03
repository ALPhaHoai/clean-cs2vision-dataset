use eframe::egui::Color32;
use std::path::PathBuf;

/// Application configuration containing all hardcoded values
///
/// This struct centralizes configuration values to make them easier to manage
/// and provides a foundation for future configuration file support.
#[derive(Clone)]
pub struct AppConfig {
    pub default_dataset_path: PathBuf,
    pub window_width: f32,
    pub window_height: f32,
    pub class_names: Vec<&'static str>,
    pub class_colors: Vec<(Color32, Color32)>, // (border_color, fill_color)
    pub side_panel_width: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_dataset_path: PathBuf::from(
                r"D:\projects\RustProjects\clean-cs2vision-dataset\sample-dataset",
            ),
            window_width: 1200.0,
            window_height: 800.0,
            class_names: vec!["T", "CT"],
            class_colors: vec![
                // T - Orange
                (
                    Color32::from_rgb(255, 140, 0),
                    Color32::from_rgba_unmultiplied(255, 140, 0, 30),
                ),
                // CT - Blue
                (
                    Color32::from_rgb(100, 149, 237),
                    Color32::from_rgba_unmultiplied(100, 149, 237, 30),
                ),
            ],
            side_panel_width: 300.0,
        }
    }
}

impl AppConfig {
    /// Get class name for a given class ID
    pub fn get_class_name(&self, class_id: u32) -> &str {
        self.class_names
            .get(class_id as usize)
            .copied()
            .unwrap_or("Unknown")
    }

    /// Get colors for a given class ID
    /// Returns (border_color, fill_color)
    pub fn get_class_colors(&self, class_id: u32) -> (Color32, Color32) {
        self.class_colors
            .get(class_id as usize)
            .copied()
            .unwrap_or((
                Color32::GRAY,
                Color32::from_rgba_unmultiplied(128, 128, 128, 30),
            ))
    }
}
