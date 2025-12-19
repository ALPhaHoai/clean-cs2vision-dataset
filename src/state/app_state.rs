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

/// Balance analysis state for dataset balance statistics
#[derive(Default)]
pub struct BalanceAnalysisState {
    /// Whether balance analysis is currently running
    pub analyzing: bool,
    /// Results from the last balance analysis
    pub results: Option<crate::core::analysis::BalanceStats>,
    /// Whether to show the balance dialog
    pub show_dialog: bool,
    /// Current tab in the dialog (0 = Balance, 1 = Integrity)
    pub current_tab: usize,
    /// Current progress (images analyzed so far)
    pub current_progress: usize,
    /// Total images to analyze
    pub total_images: usize,
    /// Tracked minimum height for popup (grows but never shrinks)
    pub tracked_min_height: f32,
    /// Cached best destination for background images (split, needed count)
    pub cached_best_bg_dest: Option<(crate::core::dataset::DatasetSplit, i32)>,
    /// Cached best destination for player images (split, needed count)
    pub cached_best_player_dest: Option<(crate::core::dataset::DatasetSplit, i32)>,
    /// Selected split to analyze (0=Train, 1=Val, 2=Test, 3=All)
    pub selected_split_index: usize,
    /// Channel receiver for progress updates from background thread
    pub(crate) progress_receiver:
        Option<std::sync::mpsc::Receiver<crate::core::analysis::BalanceProgressMessage>>,
    /// Flag to signal cancellation to background thread
    pub(crate) cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl BalanceAnalysisState {
    /// Create a new BalanceAnalysisState with default values
    pub fn new() -> Self {
        Self {
            analyzing: false,
            results: None,
            show_dialog: false,
            current_tab: 0,
            current_progress: 0,
            total_images: 0,
            tracked_min_height: 400.0,
            cached_best_bg_dest: None,
            cached_best_player_dest: None,
            selected_split_index: 0, // Default to Train
            progress_receiver: None,
            cancel_flag: None,
        }
    }
}

/// Filter state for image filtering by team, player count, etc.
#[derive(Default)]
pub struct FilterState {
    /// Current filter criteria
    pub criteria: crate::core::filter::FilterCriteria,
    /// Cached list of filtered indices (indices into the original image list)
    pub filtered_indices: Vec<usize>,
    /// Total number of images before filtering
    pub total_count: usize,
}

impl FilterState {
    /// Create a new FilterState with default values
    pub fn new() -> Self {
        Self {
            criteria: Default::default(),
            filtered_indices: Vec::new(),
            total_count: 0,
        }
    }

    /// Check if any filters are currently active
    pub fn is_active(&self) -> bool {
        self.criteria.is_active()
    }

    /// Clear all filters and reset to unfiltered state
    pub fn clear(&mut self) {
        self.criteria.clear();
        self.filtered_indices.clear();
        self.total_count = 0;
    }

    /// Get the actual (unfiltered) index from a filtered index
    /// Returns None if the filtered index is out of bounds
    pub fn get_actual_index(&self, filtered_index: usize) -> Option<usize> {
        if self.is_active() {
            self.filtered_indices.get(filtered_index).copied()
        } else {
            Some(filtered_index)
        }
    }

    /// Get the filtered (virtual) index from an actual index
    /// Returns None if the actual index is not in the filtered list
    pub fn get_filtered_index(&self, actual_index: usize) -> Option<usize> {
        if self.is_active() {
            self.filtered_indices
                .iter()
                .position(|&idx| idx == actual_index)
        } else {
            Some(actual_index)
        }
    }

    /// Get the count of filtered images (or total if no filter active)
    pub fn filtered_count(&self) -> usize {
        if self.is_active() {
            self.filtered_indices.len()
        } else {
            self.total_count
        }
    }
}

/// State for dataset rebalancing operations
#[derive(Default)]
pub struct RebalanceState {
    /// Whether a rebalance is currently being calculated or executed
    pub is_active: bool,
    /// Whether this is a global (all splits) rebalance
    pub is_global: bool,
    /// Current rebalance plan (if calculated) - single split
    pub plan: Option<crate::core::analysis::RebalancePlan>,
    /// Current global rebalance plan (if calculated) - all splits
    pub global_plan: Option<crate::core::analysis::GlobalRebalancePlan>,
    /// Current rebalance configuration
    pub config: Option<crate::core::analysis::RebalanceConfig>,
    /// Execution progress (current, total)
    pub progress: Option<(usize, usize)>,
    /// Last moved filename (for progress display)
    pub last_moved: Option<String>,
    /// Results from last execution (for undo)
    pub last_results: Option<Vec<crate::core::analysis::MoveResult>>,
    /// Channel receiver for progress updates
    pub(crate) progress_receiver:
        Option<std::sync::mpsc::Receiver<crate::core::analysis::RebalanceProgressMessage>>,
    /// Flag to signal cancellation
    pub(crate) cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Show the rebalance preview dialog
    pub show_preview: bool,
    /// Show the execution result dialog
    pub show_result: bool,
    /// Error message if something went wrong
    pub error_message: Option<String>,
}

impl RebalanceState {
    /// Create a new RebalanceState with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all state (called after closing dialogs)
    pub fn reset(&mut self) {
        self.is_active = false;
        self.plan = None;
        self.progress = None;
        self.last_moved = None;
        self.progress_receiver = None;
        self.cancel_flag = None;
        self.show_preview = false;
        self.show_result = false;
        self.error_message = None;
        // Note: keep last_results and config for undo capability
    }

    /// Check if there are results that can be undone
    pub fn can_undo(&self) -> bool {
        self.last_results
            .as_ref()
            .map(|r| r.iter().any(|res| res.success))
            .unwrap_or(false)
    }
}

/// State for dataset integrity checking
#[derive(Default)]
pub struct IntegrityState {
    /// Whether integrity check is currently running
    pub analyzing: bool,
    /// Results of the integrity check  
    pub results: Option<crate::core::analysis::IntegrityStats>,
    /// Selected issue indices (for images without labels tab)
    pub selected_images_without_labels: std::collections::HashSet<usize>,
    /// Selected issue indices (for labels without images tab)
    pub selected_labels_without_images: std::collections::HashSet<usize>,
    /// Current tab (0 = images without labels, 1 = labels without images)
    pub current_tab: usize,
    /// Current progress during analysis
    pub current_progress: usize,
    /// Total files to analyze
    pub total_files: usize,
    /// Channel receiver for progress updates
    pub(crate) progress_receiver:
        Option<std::sync::mpsc::Receiver<crate::core::analysis::IntegrityProgressMessage>>,
    /// Flag to signal cancellation
    pub(crate) cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Whether deletion is in progress
    pub deleting: bool,
    /// Error message if something went wrong
    pub error_message: Option<String>,
}

impl IntegrityState {
    /// Create a new IntegrityState with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset the state
    pub fn reset(&mut self) {
        self.analyzing = false;
        self.results = None;
        self.selected_images_without_labels.clear();
        self.selected_labels_without_images.clear();
        self.current_tab = 0;
        self.current_progress = 0;
        self.total_files = 0;
        self.progress_receiver = None;
        self.cancel_flag = None;
        self.deleting = false;
        self.error_message = None;
    }

    /// Check if there are any selected items in the current tab
    pub fn has_selection(&self) -> bool {
        match self.current_tab {
            0 => !self.selected_images_without_labels.is_empty(),
            1 => !self.selected_labels_without_images.is_empty(),
            _ => false,
        }
    }

    /// Get count of selected items in current tab
    pub fn selection_count(&self) -> usize {
        match self.current_tab {
            0 => self.selected_images_without_labels.len(),
            1 => self.selected_labels_without_images.len(),
            _ => 0,
        }
    }
}
