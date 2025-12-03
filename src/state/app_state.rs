use egui::TextureHandle;
use std::sync::mpsc::Receiver;
use std::sync::{atomic::AtomicBool, Arc};

use crate::app::BatchStats;
use crate::core::dataset::LabelInfo;

/// Batch progress message types for communication between threads
pub enum BatchProgressMessage {
    Progress(BatchStats),
    Complete(BatchStats),
    Cancelled(BatchStats),
}

/// Image-related state including texture, label, analysis, and display settings
#[derive(Default)]
pub struct ImageState {
    /// Currently loaded texture for display
    pub texture: Option<TextureHandle>,
    /// Parsed label information for the current image
    pub label: Option<LabelInfo>,
    /// Calculated dominant color of the image
    pub dominant_color: Option<egui::Color32>,
    /// Error message if image failed to load
    pub load_error: Option<String>,
    /// Current zoom level for image display
    pub zoom_level: f32,
}

impl ImageState {
    /// Create a new ImageState with default values
    pub fn new() -> Self {
        Self {
            texture: None,
            label: None,
            dominant_color: None,
            load_error: None,
            zoom_level: 1.0,
        }
    }

    /// Reset all image state (optionally preserving zoom level)
    pub fn reset(&mut self, reset_zoom: bool) {
        self.texture = None;
        self.label = None;
        self.dominant_color = None;
        self.load_error = None;
        if reset_zoom {
            self.zoom_level = 1.0;
        }
    }
}

/// UI-related state for dialogs, modes, and user input
#[derive(Default)]
pub struct UIState {
    /// Whether fullscreen mode is active
    pub fullscreen_mode: bool,
    /// Whether the filter dialog is shown
    pub show_filter_dialog: bool,
    /// Whether the batch delete confirmation dialog is shown
    pub show_batch_delete_confirm: bool,
    /// Manual index input field content
    pub manual_index_input: String,
}

impl UIState {
    /// Create a new UIState with default values
    pub fn new() -> Self {
        Self {
            fullscreen_mode: false,
            show_filter_dialog: false,
            show_batch_delete_confirm: false,
            manual_index_input: String::from("1"),
        }
    }
}

/// Batch processing state including progress tracking and cancellation
#[derive(Default)]
pub struct BatchState {
    /// Whether batch processing is currently active
    pub processing: bool,
    /// Statistics about the current/last batch operation
    pub stats: Option<BatchStats>,
    /// Channel receiver for progress updates from background thread
    pub(crate) progress_receiver: Option<Receiver<BatchProgressMessage>>,
    /// Flag to signal cancellation to background thread
    pub(crate) cancel_flag: Option<Arc<AtomicBool>>,
}

impl BatchState {
    /// Create a new BatchState with default values
    pub fn new() -> Self {
        Self {
            processing: false,
            stats: None,
            progress_receiver: None,
            cancel_flag: None,
        }
    }
}
