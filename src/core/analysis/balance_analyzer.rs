use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};
use tracing::{info, warn};

use crate::core::dataset::{parse_label_file, DatasetSplit};

/// Progress message types for background analysis
#[derive(Clone)]
pub enum BalanceProgressMessage {
    Progress {
        current: usize,
        total: usize,
        stats: BalanceStats,
    },
    Complete(BalanceStats),
    Cancelled(BalanceStats),
}

/// Categories for classifying images based on their detections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageCategory {
    /// Image contains only CT players (class_id 1)
    CTOnly,
    /// Image contains only T players (class_id 0)
    TOnly,
    /// Image contains both CT and T players
    MultiplePlayer,
    /// Image has no detections (background)
    Background,
    /// Hard case - requires manual review
    HardCase,
}

impl ImageCategory {
    pub fn as_str(&self) -> &str {
        match self {
            ImageCategory::CTOnly => "CT Only",
            ImageCategory::TOnly => "T Only",
            ImageCategory::MultiplePlayer => "Multiple Players",
            ImageCategory::Background => "Background",
            ImageCategory::HardCase => "Hard Case",
        }
    }
}

/// Statistics about dataset balance
#[derive(Debug, Clone)]
pub struct BalanceStats {
    pub total_images: usize,
    pub ct_only: usize,
    pub t_only: usize,
    pub multiple_player: usize,
    pub background: usize,
    pub hard_case: usize,
}

impl BalanceStats {
    pub fn new() -> Self {
        Self {
            total_images: 0,
            ct_only: 0,
            t_only: 0,
            multiple_player: 0,
            background: 0,
            hard_case: 0,
        }
    }

    /// Get count for a specific category
    pub fn get_count(&self, category: ImageCategory) -> usize {
        match category {
            ImageCategory::CTOnly => self.ct_only,
            ImageCategory::TOnly => self.t_only,
            ImageCategory::MultiplePlayer => self.multiple_player,
            ImageCategory::Background => self.background,
            ImageCategory::HardCase => self.hard_case,
        }
    }

    /// Get percentage for a specific category
    pub fn get_percentage(&self, category: ImageCategory) -> f32 {
        if self.total_images == 0 {
            return 0.0;
        }
        (self.get_count(category) as f32 / self.total_images as f32) * 100.0
    }

    /// Get total player images (CT + T + Multiple)
    pub fn total_player_images(&self) -> usize {
        self.ct_only + self.t_only + self.multiple_player
    }

    /// Get percentage of player images
    pub fn player_percentage(&self) -> f32 {
        if self.total_images == 0 {
            return 0.0;
        }
        (self.total_player_images() as f32 / self.total_images as f32) * 100.0
    }
}

impl Default for BalanceStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Target ratios for dataset balancing
#[derive(Debug, Clone)]
pub struct TargetRatios {
    pub player_ratio: f32,     // 0.85 for 85%
    pub background_ratio: f32, // 0.10 for 10%
    pub hardcase_ratio: f32,   // 0.05 for 5%
}

impl Default for TargetRatios {
    fn default() -> Self {
        Self {
            player_ratio: 0.85,
            background_ratio: 0.10,
            hardcase_ratio: 0.05,
        }
    }
}

// =============================================================================
// DATA INTEGRITY ANALYSIS
// =============================================================================

/// Types of integrity issues found in the dataset
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityIssueType {
    /// Image file exists but no corresponding label file
    ImageWithoutLabel,
    /// Label file exists but no corresponding image
    LabelWithoutImage,
}

/// A single integrity issue
#[derive(Debug, Clone)]
pub struct IntegrityIssue {
    pub issue_type: IntegrityIssueType,
    /// The existing file path
    pub path: PathBuf,
    /// The missing counterpart path (for display purposes)
    pub expected_counterpart: PathBuf,
}

/// Statistics about dataset integrity issues
#[derive(Debug, Clone, Default)]
pub struct IntegrityStats {
    pub images_without_labels: Vec<IntegrityIssue>,
    pub labels_without_images: Vec<IntegrityIssue>,
}

impl IntegrityStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total count of all integrity issues
    pub fn total_issues(&self) -> usize {
        self.images_without_labels.len() + self.labels_without_images.len()
    }

    /// Check if the dataset has any integrity issues
    pub fn has_issues(&self) -> bool {
        self.total_issues() > 0
    }
}

/// Progress message types for integrity analysis
#[derive(Clone)]
pub enum IntegrityProgressMessage {
    Progress {
        current: usize,
        total: usize,
        stats: IntegrityStats,
    },
    Complete(IntegrityStats),
    Cancelled(IntegrityStats),
}

/// Categorize an image based on its label file
pub fn categorize_image(label_path: &PathBuf) -> ImageCategory {
    // Try to parse the label file
    match parse_label_file(label_path) {
        Some(label_info) => {
            if label_info.detections.is_empty() {
                // No detections = background
                return ImageCategory::Background;
            }

            let mut has_ct = false;
            let mut has_t = false;

            for detection in &label_info.detections {
                match detection.class_id {
                    0 => has_t = true,
                    1 => has_ct = true,
                    _ => {} // Unknown class
                }
            }

            // Categorize based on what players are present
            match (has_ct, has_t) {
                (true, true) => ImageCategory::MultiplePlayer,
                (true, false) => ImageCategory::CTOnly,
                (false, true) => ImageCategory::TOnly,
                (false, false) => ImageCategory::Background, // Detections but none are CT or T
            }
        }
        None => {
            // No label file = background
            ImageCategory::Background
        }
    }
}

/// Analyze dataset balance for a given split with optional progress reporting
pub fn analyze_dataset_with_progress(
    dataset_path: &PathBuf,
    split: DatasetSplit,
    progress_tx: Option<Sender<BalanceProgressMessage>>,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> BalanceStats {
    let mut stats = BalanceStats::new();

    // Navigate to split/images folder
    let images_path = dataset_path.join(split.as_str()).join("images");
    let labels_path = dataset_path.join(split.as_str()).join("labels");

    info!("Analyzing balance for split: {:?}", split.as_str());
    info!("Images path: {:?}", images_path);

    // Collect all image paths first to know total count
    let mut image_paths = Vec::new();
    if let Ok(entries) = fs::read_dir(&images_path) {
        for entry in entries.flatten() {
            let image_path = entry.path();
            if let Some(ext) = image_path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if ext == "png" || ext == "jpg" || ext == "jpeg" {
                    image_paths.push(image_path);
                }
            }
        }
    } else {
        warn!("Failed to read directory: {:?}", images_path);
        if let Some(tx) = progress_tx {
            let _ = tx.send(BalanceProgressMessage::Complete(stats.clone()));
        }
        return stats;
    }

    let total_images = image_paths.len();
    stats.total_images = total_images;

    // Process each image
    for (idx, image_path) in image_paths.iter().enumerate() {
        // Check for cancellation
        if let Some(ref cancel) = cancel_flag {
            if cancel.load(Ordering::Relaxed) {
                warn!(
                    "Balance analysis cancelled by user at image {}/{}",
                    idx + 1,
                    total_images
                );
                if let Some(ref tx) = progress_tx {
                    let _ = tx.send(BalanceProgressMessage::Cancelled(stats.clone()));
                }
                return stats;
            }
        }

        // Get corresponding label file
        if let Some(stem) = image_path.file_stem() {
            let label_path = labels_path.join(format!("{}.txt", stem.to_string_lossy()));

            let category = categorize_image(&label_path);

            match category {
                ImageCategory::CTOnly => stats.ct_only += 1,
                ImageCategory::TOnly => stats.t_only += 1,
                ImageCategory::MultiplePlayer => stats.multiple_player += 1,
                ImageCategory::Background => stats.background += 1,
                ImageCategory::HardCase => stats.hard_case += 1,
            }
        }

        // Send progress update every 10 images or on last image
        if let Some(ref tx) = progress_tx {
            if (idx + 1) % 10 == 0 || idx == total_images - 1 {
                let _ = tx.send(BalanceProgressMessage::Progress {
                    current: idx + 1,
                    total: total_images,
                    stats: stats.clone(),
                });
            }
        }
    }

    info!(
        "Analysis complete: {} total images ({} player, {} background, {} hard cases)",
        stats.total_images,
        stats.total_player_images(),
        stats.background,
        stats.hard_case
    );

    // Send completion message
    if let Some(tx) = progress_tx {
        let _ = tx.send(BalanceProgressMessage::Complete(stats.clone()));
    }

    stats
}

/// Analyze dataset balance for a given split (synchronous version)
pub fn analyze_dataset(dataset_path: &PathBuf, split: DatasetSplit) -> BalanceStats {
    analyze_dataset_with_progress(dataset_path, split, None, None)
}

/// Generate recommendations for manual balancing
pub fn get_recommendations(stats: &BalanceStats, target_ratios: &TargetRatios) -> Vec<String> {
    let mut recommendations = Vec::new();

    if stats.total_images == 0 {
        recommendations.push("No images found in dataset.".to_string());
        return recommendations;
    }

    let total = stats.total_images as f32;
    let current_player_pct = stats.player_percentage();
    let current_bg_pct = stats.get_percentage(ImageCategory::Background);
    let current_hc_pct = stats.get_percentage(ImageCategory::HardCase);

    let target_player_pct = target_ratios.player_ratio * 100.0;
    let target_bg_pct = target_ratios.background_ratio * 100.0;
    let target_hc_pct = target_ratios.hardcase_ratio * 100.0;

    // Calculate ideal counts
    let ideal_player_count = (total * target_ratios.player_ratio) as i32;
    let ideal_bg_count = (total * target_ratios.background_ratio) as i32;
    let ideal_hc_count = (total * target_ratios.hardcase_ratio) as i32;

    let current_player_count = stats.total_player_images() as i32;
    let current_bg_count = stats.background as i32;
    let current_hc_count = stats.hard_case as i32;

    // Player images recommendations
    let player_diff = current_player_count - ideal_player_count;
    if player_diff > 0 {
        recommendations.push(format!(
            "📉 Remove approximately {} player images (currently {:.1}%, target {:.1}%)",
            player_diff, current_player_pct, target_player_pct
        ));

        // Suggest which type to remove based on distribution
        let ct_count = stats.ct_only as i32;
        let t_count = stats.t_only as i32;
        let multi_count = stats.multiple_player as i32;

        if ct_count > t_count + 100 {
            recommendations.push(format!(
                "   → Consider removing more CT-only images ({} available)",
                ct_count
            ));
        } else if t_count > ct_count + 100 {
            recommendations.push(format!(
                "   → Consider removing more T-only images ({} available)",
                t_count
            ));
        } else {
            recommendations.push(format!(
                "   → Balance removals across CT ({}), T ({}), and Multiple ({})",
                ct_count, t_count, multi_count
            ));
        }
    } else if player_diff < 0 {
        recommendations.push(format!(
            "📈 Add approximately {} more player images (currently {:.1}%, target {:.1}%)",
            -player_diff, current_player_pct, target_player_pct
        ));
    } else {
        recommendations.push(format!(
            "✓ Player images are balanced ({:.1}%)",
            current_player_pct
        ));
    }

    // Background recommendations
    let bg_diff = current_bg_count - ideal_bg_count;
    if bg_diff > 0 {
        recommendations.push(format!(
            "📉 Remove approximately {} background images (currently {:.1}%, target {:.1}%)",
            bg_diff, current_bg_pct, target_bg_pct
        ));
    } else if bg_diff < 0 {
        recommendations.push(format!(
            "📈 Add approximately {} more background images (currently {:.1}%, target {:.1}%)",
            -bg_diff, current_bg_pct, target_bg_pct
        ));
    } else {
        recommendations.push(format!(
            "✓ Background images are balanced ({:.1}%)",
            current_bg_pct
        ));
    }

    // Hard case recommendations
    let hc_diff = current_hc_count - ideal_hc_count;
    if current_hc_count > 0 {
        if hc_diff > 0 {
            recommendations.push(format!(
                "📉 Review and reduce hard cases by {} (currently {:.1}%, target {:.1}%)",
                hc_diff, current_hc_pct, target_hc_pct
            ));
        } else if hc_diff < 0 {
            recommendations.push(format!(
                "📈 Mark {} more images as hard cases for review (currently {:.1}%, target {:.1}%)",
                -hc_diff, current_hc_pct, target_hc_pct
            ));
        } else {
            recommendations.push(format!(
                "✓ Hard cases are balanced ({:.1}%)",
                current_hc_pct
            ));
        }
    }

    recommendations
}

/// Analyze dataset integrity to find orphaned files
/// 
/// Detects:
/// - Images without corresponding label files
/// - Label files without corresponding images
pub fn analyze_dataset_integrity_with_progress(
    dataset_path: &PathBuf,
    split: DatasetSplit,
    progress_tx: Option<Sender<IntegrityProgressMessage>>,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> IntegrityStats {
    let mut stats = IntegrityStats::new();

    let images_path = dataset_path.join(split.as_str()).join("images");
    let labels_path = dataset_path.join(split.as_str()).join("labels");

    info!("Analyzing integrity for split: {:?}", split.as_str());
    info!("Images path: {:?}", images_path);
    info!("Labels path: {:?}", labels_path);

    // Collect all image files
    let mut image_stems: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut image_paths: Vec<PathBuf> = Vec::new();
    
    if let Ok(entries) = fs::read_dir(&images_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if ext == "png" || ext == "jpg" || ext == "jpeg" {
                    if let Some(stem) = path.file_stem() {
                        image_stems.insert(stem.to_string_lossy().to_string());
                        image_paths.push(path);
                    }
                }
            }
        }
    }

    // Collect all label files
    let mut label_stems: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut label_paths: Vec<PathBuf> = Vec::new();
    
    if let Ok(entries) = fs::read_dir(&labels_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "txt" {
                    if let Some(stem) = path.file_stem() {
                        label_stems.insert(stem.to_string_lossy().to_string());
                        label_paths.push(path);
                    }
                }
            }
        }
    }

    let total_files = image_paths.len() + label_paths.len();
    let mut processed = 0;

    // Find images without labels
    for image_path in &image_paths {
        // Check for cancellation
        if let Some(ref cancel) = cancel_flag {
            if cancel.load(Ordering::Relaxed) {
                warn!("Integrity analysis cancelled by user");
                if let Some(ref tx) = progress_tx {
                    let _ = tx.send(IntegrityProgressMessage::Cancelled(stats.clone()));
                }
                return stats;
            }
        }

        if let Some(stem) = image_path.file_stem() {
            let stem_str = stem.to_string_lossy().to_string();
            if !label_stems.contains(&stem_str) {
                let expected_label = labels_path.join(format!("{}.txt", stem_str));
                stats.images_without_labels.push(IntegrityIssue {
                    issue_type: IntegrityIssueType::ImageWithoutLabel,
                    path: image_path.clone(),
                    expected_counterpart: expected_label,
                });
            }
        }

        processed += 1;
        if let Some(ref tx) = progress_tx {
            if processed % 50 == 0 {
                let _ = tx.send(IntegrityProgressMessage::Progress {
                    current: processed,
                    total: total_files,
                    stats: stats.clone(),
                });
            }
        }
    }

    // Find labels without images
    for label_path in &label_paths {
        // Check for cancellation
        if let Some(ref cancel) = cancel_flag {
            if cancel.load(Ordering::Relaxed) {
                warn!("Integrity analysis cancelled by user");
                if let Some(ref tx) = progress_tx {
                    let _ = tx.send(IntegrityProgressMessage::Cancelled(stats.clone()));
                }
                return stats;
            }
        }

        if let Some(stem) = label_path.file_stem() {
            let stem_str = stem.to_string_lossy().to_string();
            if !image_stems.contains(&stem_str) {
                // Try to guess the expected image extension
                let expected_image = images_path.join(format!("{}.png", stem_str));
                stats.labels_without_images.push(IntegrityIssue {
                    issue_type: IntegrityIssueType::LabelWithoutImage,
                    path: label_path.clone(),
                    expected_counterpart: expected_image,
                });
            }
        }

        processed += 1;
        if let Some(ref tx) = progress_tx {
            if processed % 50 == 0 || processed == total_files {
                let _ = tx.send(IntegrityProgressMessage::Progress {
                    current: processed,
                    total: total_files,
                    stats: stats.clone(),
                });
            }
        }
    }

    info!(
        "Integrity analysis complete: {} images without labels, {} labels without images",
        stats.images_without_labels.len(),
        stats.labels_without_images.len()
    );

    // Send completion message
    if let Some(tx) = progress_tx {
        let _ = tx.send(IntegrityProgressMessage::Complete(stats.clone()));
    }

    stats
}

/// Analyze dataset integrity (synchronous version)
pub fn analyze_dataset_integrity(dataset_path: &PathBuf, split: DatasetSplit) -> IntegrityStats {
    analyze_dataset_integrity_with_progress(dataset_path, split, None, None)
}
