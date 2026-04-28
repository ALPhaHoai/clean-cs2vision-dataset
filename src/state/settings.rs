use crate::core::filter::FilterCriteria;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

/// Persistent user settings that are saved between sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Last dataset path that was opened
    pub last_dataset_path: Option<PathBuf>,

    /// Last window width
    pub window_width: f32,

    /// Last window height
    pub window_height: f32,

    /// Last active split (train, val, or test)
    pub last_split: String,

    /// Last image index in the dataset
    pub last_image_index: usize,

    /// Last active filter configuration
    #[serde(default)]
    pub filter_criteria: FilterCriteria,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_dataset_path: None,
            window_width: 1200.0,
            window_height: 800.0,
            last_split: "train".to_string(),
            last_image_index: 0,
            filter_criteria: FilterCriteria::default(),
        }
    }
}

impl Settings {
    /// Get the path to the settings file (in the same directory as the executable)
    pub fn get_config_path() -> Option<PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(|dir| dir.to_path_buf()))
            .map(|dir| dir.join("settings.json"))
    }

    /// Load settings from disk, or return defaults if file doesn't exist or is corrupted
    pub fn load() -> Self {
        if let Some(config_path) = Self::get_config_path() {
            info!("Loading settings from: {:?}", config_path);

            match fs::read_to_string(&config_path) {
                Ok(contents) => match serde_json::from_str::<Settings>(&contents) {
                    Ok(settings) => {
                        info!("Successfully loaded settings");
                        return settings;
                    }
                    Err(e) => {
                        warn!("Failed to parse settings file: {}. Using defaults.", e);
                    }
                },
                Err(e) => {
                    // It's normal for the file not to exist on first run
                    if e.kind() != std::io::ErrorKind::NotFound {
                        warn!("Failed to read settings file: {}. Using defaults.", e);
                    } else {
                        info!("No settings file found. Using defaults.");
                    }
                }
            }
        } else {
            warn!("Could not determine config directory. Using defaults.");
        }

        Self::default()
    }

    /// Save settings to disk
    pub fn save(&self) {
        if let Some(config_path) = Self::get_config_path() {
            // Create config directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    warn!("Failed to create config directory: {}", e);
                    return;
                }
            }

            match serde_json::to_string_pretty(self) {
                Ok(json) => {
                    if let Err(e) = fs::write(&config_path, json) {
                        warn!("Failed to write settings file: {}", e);
                    } else {
                        info!("Settings saved to: {:?}", config_path);
                    }
                }
                Err(e) => {
                    warn!("Failed to serialize settings: {}", e);
                }
            }
        } else {
            warn!("Could not determine config directory. Settings not saved.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.window_width, 1200.0);
        assert_eq!(settings.window_height, 800.0);
        assert_eq!(settings.last_split, "train");
        assert_eq!(settings.last_image_index, 0);
        assert!(settings.last_dataset_path.is_none());
        assert!(!settings.filter_criteria.is_active());
    }

    #[test]
    fn test_settings_serialization_roundtrip() {
        let settings = Settings {
            last_dataset_path: Some(PathBuf::from("test/path/dataset")),
            window_width: 1280.0,
            window_height: 720.0,
            last_split: "val".to_string(),
            last_image_index: 42,
            filter_criteria: FilterCriteria::default(),
        };

        let json = serde_json::to_string(&settings).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();

        assert_eq!(
            loaded.last_dataset_path,
            Some(PathBuf::from("test/path/dataset"))
        );
        assert_eq!(loaded.window_width, 1280.0);
        assert_eq!(loaded.window_height, 720.0);
        assert_eq!(loaded.last_split, "val");
        assert_eq!(loaded.last_image_index, 42);
        assert!(!loaded.filter_criteria.is_active());
    }
}
