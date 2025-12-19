//! Dataset rebalancing module for moving images between splits.
//!
//! This module provides functionality to analyze and rebalance dataset splits
//! by moving images from over-represented categories to other splits.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};
use tracing::{error, info, warn};

use crate::core::dataset::{parse_label_file, DatasetSplit};
use crate::core::operations::{get_label_path_for_image, move_file};

use super::{categorize_image, BalanceStats, ImageCategory, TargetRatios};

/// Strategy for selecting which images to move
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SelectionStrategy {
    /// Select randomly
    #[default]
    Random,
    /// Select images with fewest detections first
    FewestDetections,
    /// Select oldest files first (by filename, assuming timestamp-based names)
    OldestFirst,
    /// Select newest files first
    NewestFirst,
}

impl SelectionStrategy {
    pub fn as_str(&self) -> &str {
        match self {
            SelectionStrategy::Random => "Random",
            SelectionStrategy::FewestDetections => "Fewest Detections",
            SelectionStrategy::OldestFirst => "Oldest First",
            SelectionStrategy::NewestFirst => "Newest First",
        }
    }

    pub fn all() -> Vec<SelectionStrategy> {
        vec![
            SelectionStrategy::Random,
            SelectionStrategy::FewestDetections,
            SelectionStrategy::OldestFirst,
            SelectionStrategy::NewestFirst,
        ]
    }
}

/// A single move action in a rebalance plan
#[derive(Debug, Clone)]
pub struct MoveAction {
    /// Source image path
    pub image_path: PathBuf,
    /// Source label path (if exists)
    pub label_path: Option<PathBuf>,
    /// Category of this image
    pub category: ImageCategory,
    /// Source split
    pub from_split: DatasetSplit,
    /// Destination split
    pub to_split: DatasetSplit,
}

/// Result of a single move operation
#[derive(Debug, Clone)]
pub struct MoveResult {
    pub action: MoveAction,
    pub success: bool,
    pub error: Option<String>,
    /// New image path after move
    pub new_image_path: Option<PathBuf>,
    /// New label path after move
    pub new_label_path: Option<PathBuf>,
}

/// A complete rebalance plan
#[derive(Debug, Clone, Default)]
pub struct RebalancePlan {
    /// List of move actions to execute
    pub actions: Vec<MoveAction>,
    /// Category being rebalanced
    pub category: Option<ImageCategory>,
    /// Source split
    pub from_split: Option<DatasetSplit>,
    /// Destination split
    pub to_split: Option<DatasetSplit>,
    /// Number to move
    pub count_to_move: usize,
    /// Current stats before rebalance
    pub current_stats: Option<BalanceStats>,
    /// Projected stats after rebalance
    pub projected_stats: Option<BalanceStats>,
}

impl RebalancePlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }
}

/// Configuration for rebalancing
#[derive(Debug, Clone)]
pub struct RebalanceConfig {
    /// Target ratios to achieve
    pub target_ratios: TargetRatios,
    /// Strategy for selecting images
    pub selection_strategy: SelectionStrategy,
    /// Whether to preserve CT/T balance when moving player images
    pub preserve_ct_t_balance: bool,
    /// Source split to take images from
    pub source_split: DatasetSplit,
    /// Destination split to move images to
    pub destination_split: DatasetSplit,
    /// Category to rebalance
    pub category: ImageCategory,
}

impl Default for RebalanceConfig {
    fn default() -> Self {
        Self {
            target_ratios: TargetRatios::default(),
            selection_strategy: SelectionStrategy::Random,
            preserve_ct_t_balance: true,
            source_split: DatasetSplit::Train,
            destination_split: DatasetSplit::Val,
            category: ImageCategory::Background,
        }
    }
}

/// Progress message for rebalance execution
#[derive(Debug, Clone)]
pub enum RebalanceProgressMessage {
    Progress {
        current: usize,
        total: usize,
        last_moved: String,
    },
    Complete {
        success_count: usize,
        failed_count: usize,
        results: Vec<MoveResult>,
    },
    Cancelled {
        completed_count: usize,
        results: Vec<MoveResult>,
    },
    Error(String),
}

// ============================================================================
// GLOBAL MULTI-SPLIT OPTIMIZATION
// ============================================================================

/// Statistics for all splits combined
#[derive(Debug, Clone, Default)]
pub struct GlobalBalanceStats {
    pub train: BalanceStats,
    pub val: BalanceStats,
    pub test: BalanceStats,
}

impl GlobalBalanceStats {
    /// Get total images across all splits
    pub fn total_images(&self) -> usize {
        self.train.total_images + self.val.total_images + self.test.total_images
    }

    /// Get stats for a specific split
    pub fn get(&self, split: DatasetSplit) -> &BalanceStats {
        match split {
            DatasetSplit::Train => &self.train,
            DatasetSplit::Val => &self.val,
            DatasetSplit::Test => &self.test,
        }
    }

    /// Get mutable stats for a specific split
    pub fn get_mut(&mut self, split: DatasetSplit) -> &mut BalanceStats {
        match split {
            DatasetSplit::Train => &mut self.train,
            DatasetSplit::Val => &mut self.val,
            DatasetSplit::Test => &mut self.test,
        }
    }

    /// Check if all splits are within tolerance of target ratios
    pub fn is_balanced(&self, target: &TargetRatios, tolerance: f32) -> bool {
        for split in [DatasetSplit::Train, DatasetSplit::Val, DatasetSplit::Test] {
            let stats = self.get(split);
            if stats.total_images == 0 {
                continue;
            }

            let bg_diff = (stats.get_percentage(ImageCategory::Background) / 100.0 
                - target.background_ratio).abs();
            let player_diff = (stats.player_percentage() / 100.0 
                - target.player_ratio).abs();

            if bg_diff > tolerance || player_diff > tolerance {
                return false;
            }
        }
        true
    }
}

/// A move in a global rebalance plan
#[derive(Debug, Clone)]
pub struct GlobalMoveAction {
    pub from_split: DatasetSplit,
    pub to_split: DatasetSplit,
    pub category: ImageCategory,
    pub count: usize,
    pub actions: Vec<MoveAction>,
}

/// A complete global rebalance plan with moves across all splits
#[derive(Debug, Clone, Default)]
pub struct GlobalRebalancePlan {
    /// List of move groups (from -> to)
    pub moves: Vec<GlobalMoveAction>,
    /// Current stats before rebalance
    pub current_stats: Option<GlobalBalanceStats>,
    /// Projected stats after rebalance
    pub projected_stats: Option<GlobalBalanceStats>,
    /// Total number of files to move
    pub total_moves: usize,
    /// Number of iterations used to calculate
    pub iterations_used: usize,
}

impl GlobalRebalancePlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Get all individual move actions
    pub fn all_actions(&self) -> Vec<&MoveAction> {
        self.moves.iter().flat_map(|m| m.actions.iter()).collect()
    }
}

/// Target ratios for train/val/test split distribution
#[derive(Debug, Clone)]
pub struct SplitRatios {
    pub train: f32,  // e.g., 0.70 for 70%
    pub val: f32,    // e.g., 0.15 for 15%
    pub test: f32,   // e.g., 0.15 for 15%
}

impl Default for SplitRatios {
    fn default() -> Self {
        Self {
            train: 0.70,
            val: 0.20,
            test: 0.10,
        }
    }
}

impl SplitRatios {
    /// Get the target ratio for a specific split
    pub fn get(&self, split: DatasetSplit) -> f32 {
        match split {
            DatasetSplit::Train => self.train,
            DatasetSplit::Val => self.val,
            DatasetSplit::Test => self.test,
        }
    }
}

/// Configuration for global rebalancing
#[derive(Debug, Clone)]
pub struct GlobalRebalanceConfig {
    pub target_ratios: TargetRatios,
    pub split_ratios: SplitRatios,
    pub selection_strategy: SelectionStrategy,
    /// Target ratio for CT players among all player images (0.50 = 50% CT, 50% T)
    pub ct_t_ratio: f32,
    /// Tolerance for considering a split "balanced" (e.g., 0.02 = 2%)
    pub tolerance: f32,
    /// Maximum iterations for iterative balancing
    pub max_iterations: usize,
    /// Whether to balance locations when moving images
    pub balance_locations: bool,
}

impl Default for GlobalRebalanceConfig {
    fn default() -> Self {
        Self {
            target_ratios: TargetRatios::default(),
            split_ratios: SplitRatios::default(),
            selection_strategy: SelectionStrategy::Random,
            ct_t_ratio: 0.50, // 50% CT, 50% T
            tolerance: 0.02, // 2% tolerance
            max_iterations: 10,
            balance_locations: true,
        }
    }
}

/// Metadata about an image for selection purposes
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    pub path: PathBuf,
    pub category: ImageCategory,
    pub detection_count: usize,
    /// Location from label metadata (e.g., "TSpawn", "LongDoors")
    pub location: Option<String>,
}

/// Collect metadata for all images in a split
pub fn collect_image_metadata(
    dataset_path: &PathBuf,
    split: DatasetSplit,
) -> Vec<ImageMetadata> {
    let images_path = dataset_path.join(split.as_str()).join("images");
    let labels_path = dataset_path.join(split.as_str()).join("labels");

    let mut metadata = Vec::new();

    if let Ok(entries) = fs::read_dir(&images_path) {
        for entry in entries.flatten() {
            let image_path = entry.path();
            if let Some(ext) = image_path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if ext == "png" || ext == "jpg" || ext == "jpeg" {
                    // Get label path and categorize
                    if let Some(stem) = image_path.file_stem() {
                        let label_path = labels_path.join(format!("{}.txt", stem.to_string_lossy()));
                        let category = categorize_image(&label_path);
                        
                        // Parse label to get detection count and location
                        let (detection_count, location) = if let Some(label_info) = parse_label_file(&label_path) {
                            (label_info.detections.len(), label_info.location)
                        } else {
                            (0, None)
                        };

                        metadata.push(ImageMetadata {
                            path: image_path,
                            category,
                            detection_count,
                            location,
                        });
                    }
                }
            }
        }
    }

    metadata
}

/// Calculate how many images to move based on current stats and targets
pub fn calculate_move_count(
    stats: &BalanceStats,
    category: ImageCategory,
    target_ratios: &TargetRatios,
) -> i32 {
    let total = stats.total_images as f32;
    if total == 0.0 {
        return 0;
    }

    match category {
        ImageCategory::Background => {
            let current = stats.background as f32;
            let target = total * target_ratios.background_ratio;
            (current - target).round() as i32
        }
        ImageCategory::CTOnly | ImageCategory::TOnly | ImageCategory::MultiplePlayer => {
            // For player categories, calculate based on total player ratio
            let current_player = stats.total_player_images() as f32;
            let target_player = total * target_ratios.player_ratio;
            (current_player - target_player).round() as i32
        }
        ImageCategory::HardCase => {
            let current = stats.hard_case as f32;
            let target = total * target_ratios.hardcase_ratio;
            (current - target).round() as i32
        }
    }
}

/// Find the best destination split for moving excess images of a category.
/// Returns the split that needs the most images of that category, along with how many it needs.
pub fn find_best_destination_split(
    dataset_path: &PathBuf,
    source_split: DatasetSplit,
    category: ImageCategory,
    target_ratios: &TargetRatios,
) -> Option<(DatasetSplit, i32)> {
    use super::analyze_dataset;
    
    let other_splits: Vec<DatasetSplit> = [DatasetSplit::Train, DatasetSplit::Val, DatasetSplit::Test]
        .into_iter()
        .filter(|s| *s != source_split)
        .collect();

    let mut best_split: Option<(DatasetSplit, i32)> = None;

    for split in other_splits {
        let stats = analyze_dataset(dataset_path, split);
        let excess = calculate_move_count(&stats, category, target_ratios);
        
        // Negative excess means this split needs MORE images
        // We want to find the split with the most negative excess (needs the most)
        if excess < 0 {
            let needed = -excess; // Convert to positive "needed" count
            if let Some((_, current_best)) = best_split {
                if needed > current_best {
                    best_split = Some((split, needed));
                }
            } else {
                best_split = Some((split, needed));
            }
        }
    }

    best_split
}

/// Analyze all splits and return combined statistics
pub fn analyze_all_splits(dataset_path: &PathBuf) -> GlobalBalanceStats {
    use super::analyze_dataset;
    
    GlobalBalanceStats {
        train: analyze_dataset(dataset_path, DatasetSplit::Train),
        val: analyze_dataset(dataset_path, DatasetSplit::Val),
        test: analyze_dataset(dataset_path, DatasetSplit::Test),
    }
}

/// Calculate a global rebalance plan that redistributes images between splits
/// to match target split ratios (e.g., 70%/15%/15% for train/val/test)
pub fn calculate_global_rebalance_plan(
    dataset_path: &PathBuf,
    config: &GlobalRebalanceConfig,
) -> GlobalRebalancePlan {
    let mut plan = GlobalRebalancePlan::new();
    
    // Analyze all splits
    let initial_stats = analyze_all_splits(dataset_path);
    plan.current_stats = Some(initial_stats.clone());
    
    // Calculate total images across all splits
    let total_images = initial_stats.train.total_images 
                     + initial_stats.val.total_images 
                     + initial_stats.test.total_images;
    
    if total_images == 0 {
        info!("No images found in dataset");
        plan.projected_stats = Some(initial_stats);
        return plan;
    }
    
    // Calculate target counts for each split
    let target_train = (total_images as f32 * config.split_ratios.train).round() as usize;
    let target_val = (total_images as f32 * config.split_ratios.val).round() as usize;
    let target_test = total_images - target_train - target_val; // Remainder goes to test
    
    info!(
        "Split balancing: Total={}, Target Train={} ({}%), Val={} ({}%), Test={} ({}%)",
        total_images,
        target_train, (config.split_ratios.train * 100.0) as i32,
        target_val, (config.split_ratios.val * 100.0) as i32,
        target_test, (config.split_ratios.test * 100.0) as i32
    );
    
    info!(
        "Current: Train={} ({:.1}%), Val={} ({:.1}%), Test={} ({:.1}%)",
        initial_stats.train.total_images,
        initial_stats.train.total_images as f32 / total_images as f32 * 100.0,
        initial_stats.val.total_images,
        initial_stats.val.total_images as f32 / total_images as f32 * 100.0,
        initial_stats.test.total_images,
        initial_stats.test.total_images as f32 / total_images as f32 * 100.0
    );
    
    // Calculate excess/deficit for each split
    let mut excess: HashMap<DatasetSplit, i32> = HashMap::new();
    excess.insert(DatasetSplit::Train, initial_stats.train.total_images as i32 - target_train as i32);
    excess.insert(DatasetSplit::Val, initial_stats.val.total_images as i32 - target_val as i32);
    excess.insert(DatasetSplit::Test, initial_stats.test.total_images as i32 - target_test as i32);
    
    // Check if already balanced within tolerance
    let tolerance_count = (total_images as f32 * config.tolerance) as i32;
    let splits_balanced = excess.values().all(|&e| e.abs() <= tolerance_count);
    
    // Check if locations are imbalanced (if location balancing is enabled)
    let locations_imbalanced = if config.balance_locations {
        // Collect all unique locations across all splits
        let mut all_locations: HashMap<String, Vec<(DatasetSplit, usize)>> = HashMap::new();
        for (split, stats) in [
            (DatasetSplit::Train, &initial_stats.train),
            (DatasetSplit::Val, &initial_stats.val),
            (DatasetSplit::Test, &initial_stats.test),
        ] {
            for (loc, &count) in &stats.location_counts {
                all_locations.entry(loc.clone()).or_default().push((split, count));
            }
        }
        
        info!("Location balance check: found {} unique locations", all_locations.len());
        
        // Check if any location is significantly imbalanced across splits
        // (e.g., one split has 80% of a location while another has 5%)
        let mut has_imbalance = false;
        for (loc, split_counts) in &all_locations {
            // Only check locations that exist in at least 1 split with meaningful count
            let total: usize = split_counts.iter().map(|(_, c)| c).sum();
            if total >= 5 {  // Only check if there are enough samples
                for &(split, count) in split_counts {
                    let expected_ratio = match split {
                        DatasetSplit::Train => config.split_ratios.train,
                        DatasetSplit::Val => config.split_ratios.val,
                        DatasetSplit::Test => config.split_ratios.test,
                    };
                    let actual_ratio = count as f32 / total as f32;
                    // Consider imbalanced if ratio differs by more than 10% from expected
                    if (actual_ratio - expected_ratio).abs() > 0.10 {
                        info!("Location '{}' is imbalanced in {:?}: actual {:.1}% vs expected {:.1}%", 
                            loc, split, actual_ratio * 100.0, expected_ratio * 100.0);
                        has_imbalance = true;
                        break;
                    }
                }
            }
            if has_imbalance { break; }
        }
        has_imbalance
    } else {
        false
    };
    
    if splits_balanced && !locations_imbalanced {
        info!("Splits already balanced within {}% tolerance and locations are balanced", (config.tolerance * 100.0) as i32);
        plan.projected_stats = Some(initial_stats);
        return plan;
    }
    
    // Collect metadata for all splits
    let mut metadata: HashMap<DatasetSplit, Vec<ImageMetadata>> = HashMap::new();
    for split in [DatasetSplit::Train, DatasetSplit::Val, DatasetSplit::Test] {
        metadata.insert(split, collect_image_metadata(dataset_path, split));
    }
    
    // If splits are balanced but locations aren't, use SMART SWAP MODE
    let swap_mode = splits_balanced && locations_imbalanced;
    if swap_mode {
        info!("=== SMART SWAP MODE: Improving location balance via intelligent swaps ===");
        
        // Calculate location ratios for each split to find swap opportunities
        let splits = [DatasetSplit::Train, DatasetSplit::Val, DatasetSplit::Test];
        
        // Find best swap pair by comparing location imbalances
        let mut best_swap: Option<(DatasetSplit, DatasetSplit, usize)> = None;
        
        for i in 0..splits.len() {
            for j in (i+1)..splits.len() {
                let split_a = splits[i];
                let split_b = splits[j];
                
                let stats_a = initial_stats.get(split_a);
                let stats_b = initial_stats.get(split_b);
                
                // Find locations that are overrepresented in A but underrepresented in B (and vice versa)
                let mut swap_potential = 0usize;
                
                for (loc, &count_a) in &stats_a.location_counts {
                    if let Some(&count_b) = stats_b.location_counts.get(loc) {
                        let total = count_a + count_b;
                        if total > 5 {
                            let ratio_a = count_a as f32 / stats_a.total_images as f32;
                            let ratio_b = count_b as f32 / stats_b.total_images as f32;
                            
                            // If significantly different ratios, this is a good swap candidate
                            if (ratio_a - ratio_b).abs() > 0.05 {
                                swap_potential += std::cmp::min(count_a, count_b);
                            }
                        }
                    }
                }
                
                info!("Swap potential {:?} <-> {:?}: {} images", split_a, split_b, swap_potential);
                
                if let Some((_, _, current_best)) = best_swap {
                    if swap_potential > current_best {
                        best_swap = Some((split_a, split_b, swap_potential));
                    }
                } else if swap_potential > 0 {
                    best_swap = Some((split_a, split_b, swap_potential));
                }
            }
        }
        
        if let Some((split_a, split_b, potential)) = best_swap {
            info!("Selected swap pair: {:?} <-> {:?} with potential {}", split_a, split_b, potential);
            
            // Get stats for both splits
            let stats_a = initial_stats.get(split_a);
            let stats_b = initial_stats.get(split_b);
            
            // Find locations overrepresented in A (should move A→B)
            // and locations overrepresented in B (should move B→A)
            let mut a_overrep_locations: Vec<String> = Vec::new();
            let mut b_overrep_locations: Vec<String> = Vec::new();
            
            for (loc, &count_a) in &stats_a.location_counts {
                if let Some(&count_b) = stats_b.location_counts.get(loc) {
                    let ratio_a = count_a as f32 / stats_a.total_images as f32;
                    let ratio_b = count_b as f32 / stats_b.total_images as f32;
                    
                    if ratio_a > ratio_b + 0.03 {
                        a_overrep_locations.push(loc.clone());
                    } else if ratio_b > ratio_a + 0.03 {
                        b_overrep_locations.push(loc.clone());
                    }
                }
            }
            
            info!("Locations overrepresented in {:?}: {:?}", split_a, a_overrep_locations);
            info!("Locations overrepresented in {:?}: {:?}", split_b, b_overrep_locations);
            
            // Select images from A with overrepresented locations
            let images_a = metadata.get(&split_a).unwrap();
            let images_b = metadata.get(&split_b).unwrap();
            
            let mut swap_from_a: Vec<&ImageMetadata> = images_a.iter()
                .filter(|img| img.location.as_ref().map(|l| a_overrep_locations.contains(l)).unwrap_or(false))
                .collect();
            
            let mut swap_from_b: Vec<&ImageMetadata> = images_b.iter()
                .filter(|img| img.location.as_ref().map(|l| b_overrep_locations.contains(l)).unwrap_or(false))
                .collect();
            
            // Limit swap count to maintain balance
            let swap_count = std::cmp::min(swap_from_a.len(), swap_from_b.len());
            let swap_count = std::cmp::min(swap_count, 100); // Cap at 100 swaps
            
            if swap_count > 0 {
                info!("Planning {} bidirectional swaps between {:?} and {:?}", swap_count, split_a, split_b);
                
                // Create A → B moves
                let labels_path_a = dataset_path.join(split_a.as_str()).join("labels");
                let mut actions_a_to_b = Vec::new();
                for meta in swap_from_a.iter().take(swap_count) {
                    let label_path = if let Some(stem) = meta.path.file_stem() {
                        let lp = labels_path_a.join(format!("{}.txt", stem.to_string_lossy()));
                        if lp.exists() { Some(lp) } else { None }
                    } else {
                        None
                    };
                    actions_a_to_b.push(MoveAction {
                        image_path: meta.path.clone(),
                        label_path,
                        category: meta.category,
                        from_split: split_a,
                        to_split: split_b,
                    });
                }
                
                // Create B → A moves
                let labels_path_b = dataset_path.join(split_b.as_str()).join("labels");
                let mut actions_b_to_a = Vec::new();
                for meta in swap_from_b.iter().take(swap_count) {
                    let label_path = if let Some(stem) = meta.path.file_stem() {
                        let lp = labels_path_b.join(format!("{}.txt", stem.to_string_lossy()));
                        if lp.exists() { Some(lp) } else { None }
                    } else {
                        None
                    };
                    actions_b_to_a.push(MoveAction {
                        image_path: meta.path.clone(),
                        label_path,
                        category: meta.category,
                        from_split: split_b,
                        to_split: split_a,
                    });
                }
                
                // Add to plan
                if !actions_a_to_b.is_empty() {
                    plan.moves.push(GlobalMoveAction {
                        from_split: split_a,
                        to_split: split_b,
                        category: ImageCategory::CTOnly,
                        count: actions_a_to_b.len(),
                        actions: actions_a_to_b,
                    });
                }
                if !actions_b_to_a.is_empty() {
                    plan.moves.push(GlobalMoveAction {
                        from_split: split_b,
                        to_split: split_a,
                        category: ImageCategory::CTOnly,
                        count: actions_b_to_a.len(),
                        actions: actions_b_to_a,
                    });
                }
                
                plan.total_moves = plan.moves.iter().map(|m| m.count).sum();
                plan.iterations_used = 1;
                
                info!("Smart Swap Mode complete: {} total moves planned", plan.total_moves);
            } else {
                // Try one-directional moves if bidirectional swaps not possible
                // This may slightly change split sizes but improves location balance
                info!("No bidirectional swap candidates - trying one-directional location moves");
                
                // If only one split has overrepresented locations, move FROM that split
                let (source_split, source_locations, dest_split) = if !a_overrep_locations.is_empty() && b_overrep_locations.is_empty() {
                    (split_a, a_overrep_locations.clone(), split_b)
                } else if !b_overrep_locations.is_empty() && a_overrep_locations.is_empty() {
                    (split_b, b_overrep_locations.clone(), split_a)
                } else {
                    info!("Neither split has clear overrepresentation - falling back to normal mode");
                    // Fall through to normal mode below
                    (split_a, Vec::new(), split_b)
                };
                
                if !source_locations.is_empty() {
                    let source_images = metadata.get(&source_split).unwrap();
                    let candidates: Vec<&ImageMetadata> = source_images.iter()
                        .filter(|img| img.location.as_ref().map(|l| source_locations.contains(l)).unwrap_or(false))
                        .collect();
                    
                    let move_count = std::cmp::min(candidates.len(), 50); // Cap at 50 moves
                    
                    if move_count > 0 {
                        info!("Planning {} one-directional moves from {:?} to {:?} for locations: {:?}", 
                            move_count, source_split, dest_split, source_locations);
                        
                        let labels_path = dataset_path.join(source_split.as_str()).join("labels");
                        let mut actions = Vec::new();
                        for meta in candidates.iter().take(move_count) {
                            let label_path = if let Some(stem) = meta.path.file_stem() {
                                let lp = labels_path.join(format!("{}.txt", stem.to_string_lossy()));
                                if lp.exists() { Some(lp) } else { None }
                            } else {
                                None
                            };
                            actions.push(MoveAction {
                                image_path: meta.path.clone(),
                                label_path,
                                category: meta.category,
                                from_split: source_split,
                                to_split: dest_split,
                            });
                        }
                        
                        plan.moves.push(GlobalMoveAction {
                            from_split: source_split,
                            to_split: dest_split,
                            category: ImageCategory::CTOnly,
                            count: actions.len(),
                            actions,
                        });
                        
                        plan.total_moves = move_count;
                        plan.iterations_used = 1;
                        
                        info!("One-directional location move complete: {} moves planned", plan.total_moves);
                        plan.projected_stats = Some(initial_stats);
                        return plan;
                    }
                }
                
                // If still nothing, log and fall through to normal mode
                info!("Smart Swap Mode found no candidates - falling through to normal redistribution mode");
            }
        } else {
            info!("No suitable split pair found for swapping - falling through to normal mode");
        }
        
        // Fall through: if swap mode didn't produce a plan, try normal mode anyway
        if plan.moves.is_empty() {
            info!("Attempting normal redistribution mode as fallback");
            // Continue to normal mode below instead of returning
        } else {
            plan.projected_stats = Some(initial_stats);
            return plan;
        }
    }
    
    // Track projected stats as we plan moves
    let mut projected = initial_stats.clone();
    
    // Find splits with excess and splits with deficit
    let mut iterations = 0;
    for _iteration in 0..config.max_iterations {
        iterations += 1;
        
        // Recalculate excess/deficit
        excess.insert(DatasetSplit::Train, projected.train.total_images as i32 - target_train as i32);
        excess.insert(DatasetSplit::Val, projected.val.total_images as i32 - target_val as i32);
        excess.insert(DatasetSplit::Test, projected.test.total_images as i32 - target_test as i32);
        
        // Normal mode: determine source and destination splits
        let (from_split, to_split, move_count) = {
            // Find split with most deficit first
            let to_split = *excess.iter()
                .filter(|(_, &e)| e < -tolerance_count)
                .min_by_key(|(_, &e)| e)
                .map(|(s, _)| s)
                .unwrap_or(&DatasetSplit::Val);
            
            let to_deficit = -(*excess.get(&to_split).unwrap_or(&0));
            if to_deficit <= 0 {
                break; // No deficit to fill
            }
            
            // Find split with most excess - be more flexible if there's a significant deficit
            // If deficit > tolerance, accept any positive excess
            let min_excess = if to_deficit > tolerance_count { 0 } else { tolerance_count };
            
            let from_split = *excess.iter()
                .filter(|(_, &e)| e > min_excess)
                .max_by_key(|(_, &e)| e)
                .map(|(s, _)| s)
                .unwrap_or(&DatasetSplit::Train);
            
            let from_excess = *excess.get(&from_split).unwrap_or(&0);
            if from_excess <= 0 {
                break; // No excess to redistribute
            }
            
            // Calculate how many to move - limit to what's needed
            let move_count = std::cmp::min(from_excess, to_deficit) as usize;
            if move_count == 0 {
                break;
            }
            
            info!("Planning move: {} images from {:?} (excess {}) to {:?} (deficit {})", 
                move_count, from_split, from_excess, to_split, to_deficit);
            
            (from_split, to_split, move_count)
        };
        
        // Get images from source split
        let labels_path = dataset_path.join(from_split.as_str()).join("labels");
        let available = metadata.get_mut(&from_split).unwrap();
        
        // Calculate CT/T balance for destination split to decide which to prefer
        let to_stats = projected.get(to_split);
        let to_ct = to_stats.ct_only;
        let to_t = to_stats.t_only;
        let total_players = to_ct + to_t;
        
        // Determine which player type the destination needs more of
        let prefer_ct = if total_players > 0 {
            let ct_ratio = to_ct as f32 / total_players as f32;
            ct_ratio < config.ct_t_ratio // If CT ratio is below target, prefer CT
        } else {
            true // No players yet, prefer CT
        };
        
        // Calculate location balance for destination split (if enabled)
        let dest_location_counts = &to_stats.location_counts;
        let avg_location_count = if config.balance_locations && !dest_location_counts.is_empty() {
            dest_location_counts.values().sum::<usize>() as f32 / dest_location_counts.len() as f32
        } else {
            0.0
        };
        
        // Sort available images to prioritize:
        // 1. Needed player type (CT/T balance)
        // 2. Locations underrepresented in destination (if balance_locations enabled)
        available.sort_by(|a, b| {
            // First: category priority
            let priority_a = match a.category {
                ImageCategory::CTOnly => if prefer_ct { 0 } else { 1 },
                ImageCategory::TOnly => if prefer_ct { 1 } else { 0 },
                ImageCategory::MultiplePlayer => 2,
                ImageCategory::Background => 3,
                ImageCategory::HardCase => 4,
            };
            let priority_b = match b.category {
                ImageCategory::CTOnly => if prefer_ct { 0 } else { 1 },
                ImageCategory::TOnly => if prefer_ct { 1 } else { 0 },
                ImageCategory::MultiplePlayer => 2,
                ImageCategory::Background => 3,
                ImageCategory::HardCase => 4,
            };
            
            // Primary sort by category
            let cat_cmp = priority_a.cmp(&priority_b);
            if cat_cmp != std::cmp::Ordering::Equal {
                return cat_cmp;
            }
            
            // Secondary sort by location (prefer underrepresented locations in destination)
            if config.balance_locations && avg_location_count > 0.0 {
                let loc_count_a = a.location.as_ref()
                    .and_then(|loc| dest_location_counts.get(loc))
                    .copied()
                    .unwrap_or(0);
                let loc_count_b = b.location.as_ref()
                    .and_then(|loc| dest_location_counts.get(loc))
                    .copied()
                    .unwrap_or(0);
                
                // Prefer images from locations that are underrepresented in destination
                // (lower count = higher priority)
                loc_count_a.cmp(&loc_count_b)
            } else {
                std::cmp::Ordering::Equal
            }
        });
        
        // Shuffle within same priority groups for variety
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        
        let mut actions = Vec::new();
        let mut moved_indices = Vec::new();
        
        for (idx, meta) in available.iter().enumerate() {
            if actions.len() >= move_count {
                break;
            }
            
            let label_path = if let Some(stem) = meta.path.file_stem() {
                let lp = labels_path.join(format!("{}.txt", stem.to_string_lossy()));
                if lp.exists() { Some(lp) } else { None }
            } else {
                None
            };

            actions.push(MoveAction {
                image_path: meta.path.clone(),
                label_path,
                category: meta.category,
                from_split,
                to_split,
            });
            moved_indices.push(idx);
        }

        // Remove moved items from available pool (in reverse order to preserve indices)
        for idx in moved_indices.into_iter().rev() {
            available.remove(idx);
        }

        // Update projected stats
        for action in &actions {
            let from_stats = projected.get_mut(from_split);
            match action.category {
                ImageCategory::CTOnly => from_stats.ct_only = from_stats.ct_only.saturating_sub(1),
                ImageCategory::TOnly => from_stats.t_only = from_stats.t_only.saturating_sub(1),
                ImageCategory::MultiplePlayer => from_stats.multiple_player = from_stats.multiple_player.saturating_sub(1),
                ImageCategory::Background => from_stats.background = from_stats.background.saturating_sub(1),
                ImageCategory::HardCase => from_stats.hard_case = from_stats.hard_case.saturating_sub(1),
            }
            from_stats.total_images = from_stats.total_images.saturating_sub(1);

            let to_stats = projected.get_mut(to_split);
            match action.category {
                ImageCategory::CTOnly => to_stats.ct_only += 1,
                ImageCategory::TOnly => to_stats.t_only += 1,
                ImageCategory::MultiplePlayer => to_stats.multiple_player += 1,
                ImageCategory::Background => to_stats.background += 1,
                ImageCategory::HardCase => to_stats.hard_case += 1,
            }
            to_stats.total_images += 1;
        }

        // Add to plan - aggregate with existing moves for same from/to pair
        if !actions.is_empty() {
            // Find existing move group for this from/to pair, or create new
            let existing = plan.moves.iter_mut().find(|m| m.from_split == from_split && m.to_split == to_split);
            if let Some(move_group) = existing {
                move_group.count += actions.len();
                move_group.actions.extend(actions);
            } else {
                plan.moves.push(GlobalMoveAction {
                    from_split,
                    to_split,
                    category: ImageCategory::CTOnly, // Placeholder - mixed categories
                    count: actions.len(),
                    actions,
                });
            }
            plan.total_moves = plan.moves.iter().map(|m| m.count).sum();
        }
    }
    
    plan.iterations_used = iterations;
    plan.projected_stats = Some(projected);
    
    info!(
        "Split rebalance plan: {} moves in {} groups, {} iterations",
        plan.total_moves,
        plan.moves.len(),
        plan.iterations_used
    );

    plan
}

/// Calculate how much a move would improve overall balance (sum of squared deviations)
fn calculate_balance_improvement(
    current: &GlobalBalanceStats,
    from_split: DatasetSplit,
    to_split: DatasetSplit,
    category: ImageCategory,
    count: usize,
    target: &TargetRatios,
) -> f32 {
    // Calculate current total deviation
    let current_deviation = calculate_total_deviation(current, target);
    
    // Simulate the move
    let mut simulated = current.clone();
    
    let from_stats = simulated.get_mut(from_split);
    match category {
        ImageCategory::Background => from_stats.background = from_stats.background.saturating_sub(count),
        _ => {
            // For player types, distribute evenly among CT/T/Multi
            let each = count / 3;
            from_stats.ct_only = from_stats.ct_only.saturating_sub(each);
            from_stats.t_only = from_stats.t_only.saturating_sub(each);
            from_stats.multiple_player = from_stats.multiple_player.saturating_sub(count - 2 * each);
        }
    }
    from_stats.total_images = from_stats.total_images.saturating_sub(count);

    let to_stats = simulated.get_mut(to_split);
    match category {
        ImageCategory::Background => to_stats.background += count,
        _ => {
            let each = count / 3;
            to_stats.ct_only += each;
            to_stats.t_only += each;
            to_stats.multiple_player += count - 2 * each;
        }
    }
    to_stats.total_images += count;

    let new_deviation = calculate_total_deviation(&simulated, target);
    
    // Return improvement (positive if better)
    current_deviation - new_deviation
}

/// Calculate total deviation from target across all splits (sum of squared differences)
fn calculate_total_deviation(stats: &GlobalBalanceStats, target: &TargetRatios) -> f32 {
    let mut total = 0.0;
    
    for split in [DatasetSplit::Train, DatasetSplit::Val, DatasetSplit::Test] {
        let s = stats.get(split);
        if s.total_images == 0 {
            continue;
        }
        
        let bg_diff = s.get_percentage(ImageCategory::Background) / 100.0 - target.background_ratio;
        let player_diff = s.player_percentage() / 100.0 - target.player_ratio;
        
        total += bg_diff * bg_diff + player_diff * player_diff;
    }
    
    total
}
pub fn calculate_rebalance_plan(
    dataset_path: &PathBuf,
    config: &RebalanceConfig,
    source_stats: &BalanceStats,
) -> RebalancePlan {
    let mut plan = RebalancePlan::new();
    plan.from_split = Some(config.source_split);
    plan.to_split = Some(config.destination_split);
    plan.category = Some(config.category);
    plan.current_stats = Some(source_stats.clone());

    // Calculate how many to move
    let excess = calculate_move_count(source_stats, config.category, &config.target_ratios);
    
    if excess <= 0 {
        info!("No excess images to move for category {:?}", config.category);
        return plan;
    }

    let count_to_move = excess as usize;
    plan.count_to_move = count_to_move;

    // Collect image metadata for the source split
    let mut metadata = collect_image_metadata(dataset_path, config.source_split);

    // Filter to only the target category (or player categories if balancing players)
    let target_categories: Vec<ImageCategory> = match config.category {
        ImageCategory::CTOnly | ImageCategory::TOnly | ImageCategory::MultiplePlayer => {
            if config.preserve_ct_t_balance {
                vec![ImageCategory::CTOnly, ImageCategory::TOnly, ImageCategory::MultiplePlayer]
            } else {
                vec![config.category]
            }
        }
        _ => vec![config.category],
    };

    metadata.retain(|m| target_categories.contains(&m.category));

    // Sort based on strategy
    match config.selection_strategy {
        SelectionStrategy::Random => {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            metadata.shuffle(&mut rng);
        }
        SelectionStrategy::FewestDetections => {
            metadata.sort_by_key(|m| m.detection_count);
        }
        SelectionStrategy::OldestFirst => {
            metadata.sort_by(|a, b| a.path.cmp(&b.path));
        }
        SelectionStrategy::NewestFirst => {
            metadata.sort_by(|a, b| b.path.cmp(&a.path));
        }
    }

    // If preserving CT/T balance, interleave selections from each category
    if config.preserve_ct_t_balance && matches!(config.category, 
        ImageCategory::CTOnly | ImageCategory::TOnly | ImageCategory::MultiplePlayer) 
    {
        let mut by_category: HashMap<ImageCategory, Vec<ImageMetadata>> = HashMap::new();
        for m in metadata {
            by_category.entry(m.category).or_default().push(m);
        }

        let mut selected = Vec::new();
        let mut indices: HashMap<ImageCategory, usize> = HashMap::new();
        let categories = [ImageCategory::CTOnly, ImageCategory::TOnly, ImageCategory::MultiplePlayer];

        while selected.len() < count_to_move {
            let mut added_any = false;
            for cat in &categories {
                if selected.len() >= count_to_move {
                    break;
                }
                if let Some(images) = by_category.get(cat) {
                    let idx = indices.entry(*cat).or_insert(0);
                    if *idx < images.len() {
                        selected.push(images[*idx].clone());
                        *idx += 1;
                        added_any = true;
                    }
                }
            }
            if !added_any {
                break;
            }
        }

        metadata = selected;
    }

    // Take the required number of images
    let labels_path = dataset_path.join(config.source_split.as_str()).join("labels");
    
    for m in metadata.into_iter().take(count_to_move) {
        let label_path = if let Some(stem) = m.path.file_stem() {
            let lp = labels_path.join(format!("{}.txt", stem.to_string_lossy()));
            if lp.exists() { Some(lp) } else { None }
        } else {
            None
        };

        plan.actions.push(MoveAction {
            image_path: m.path,
            label_path,
            category: m.category,
            from_split: config.source_split,
            to_split: config.destination_split,
        });
    }

    // Calculate projected stats
    let mut projected = source_stats.clone();
    for action in &plan.actions {
        match action.category {
            ImageCategory::CTOnly => projected.ct_only = projected.ct_only.saturating_sub(1),
            ImageCategory::TOnly => projected.t_only = projected.t_only.saturating_sub(1),
            ImageCategory::MultiplePlayer => projected.multiple_player = projected.multiple_player.saturating_sub(1),
            ImageCategory::Background => projected.background = projected.background.saturating_sub(1),
            ImageCategory::HardCase => projected.hard_case = projected.hard_case.saturating_sub(1),
        }
        projected.total_images = projected.total_images.saturating_sub(1);
    }
    plan.projected_stats = Some(projected);

    info!(
        "Rebalance plan: move {} {:?} images from {:?} to {:?}",
        plan.actions.len(),
        config.category,
        config.source_split,
        config.destination_split
    );

    plan
}

/// Execute a rebalance plan, moving files between splits
pub fn execute_rebalance_plan(
    dataset_path: &PathBuf,
    plan: &RebalancePlan,
    progress_tx: Option<Sender<RebalanceProgressMessage>>,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> Vec<MoveResult> {
    let mut results = Vec::new();
    let total = plan.actions.len();

    if total == 0 {
        if let Some(tx) = progress_tx {
            let _ = tx.send(RebalanceProgressMessage::Complete {
                success_count: 0,
                failed_count: 0,
                results: vec![],
            });
        }
        return results;
    }

    let to_split = plan.to_split.unwrap_or(DatasetSplit::Val);
    let dest_images = dataset_path.join(to_split.as_str()).join("images");
    let dest_labels = dataset_path.join(to_split.as_str()).join("labels");

    // Ensure destination directories exist
    if let Err(e) = fs::create_dir_all(&dest_images) {
        error!("Failed to create destination images directory: {}", e);
        if let Some(tx) = progress_tx {
            let _ = tx.send(RebalanceProgressMessage::Error(format!(
                "Failed to create destination directory: {}", e
            )));
        }
        return results;
    }
    if let Err(e) = fs::create_dir_all(&dest_labels) {
        error!("Failed to create destination labels directory: {}", e);
        if let Some(tx) = progress_tx {
            let _ = tx.send(RebalanceProgressMessage::Error(format!(
                "Failed to create destination directory: {}", e
            )));
        }
        return results;
    }

    let mut success_count = 0;
    let mut failed_count = 0;

    for (idx, action) in plan.actions.iter().enumerate() {
        // Check cancellation
        if let Some(ref cancel) = cancel_flag {
            if cancel.load(Ordering::Relaxed) {
                warn!("Rebalance cancelled at {}/{}", idx, total);
                if let Some(ref tx) = progress_tx {
                    let _ = tx.send(RebalanceProgressMessage::Cancelled {
                        completed_count: idx,
                        results: results.clone(),
                    });
                }
                return results;
            }
        }

        let filename = action.image_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Calculate destination paths
        let new_image_path = dest_images.join(filename);
        let new_label_path = action.label_path.as_ref().and_then(|lp| {
            lp.file_name().map(|n| dest_labels.join(n))
        });

        // Move image file
        let image_result = move_file(&action.image_path, &new_image_path);
        
        if let Err(e) = image_result {
            error!("Failed to move image {:?}: {}", action.image_path, e);
            results.push(MoveResult {
                action: action.clone(),
                success: false,
                error: Some(format!("Failed to move image: {}", e)),
                new_image_path: None,
                new_label_path: None,
            });
            failed_count += 1;
            continue;
        }

        // Move label file if exists
        let mut label_moved = true;
        let mut final_label_path = None;
        
        if let (Some(src_label), Some(dst_label)) = (&action.label_path, &new_label_path) {
            if src_label.exists() {
                if let Err(e) = move_file(src_label, dst_label) {
                    warn!("Failed to move label {:?}: {}", src_label, e);
                    label_moved = false;
                    // Don't fail entirely - the image was moved successfully
                } else {
                    final_label_path = Some(dst_label.clone());
                }
            }
        }

        results.push(MoveResult {
            action: action.clone(),
            success: true,
            error: if label_moved { None } else { Some("Label move failed".to_string()) },
            new_image_path: Some(new_image_path),
            new_label_path: final_label_path,
        });
        success_count += 1;

        // Send progress update
        if let Some(ref tx) = progress_tx {
            if (idx + 1) % 5 == 0 || idx == total - 1 {
                let _ = tx.send(RebalanceProgressMessage::Progress {
                    current: idx + 1,
                    total,
                    last_moved: filename.to_string(),
                });
            }
        }
    }

    info!(
        "Rebalance complete: {} succeeded, {} failed",
        success_count, failed_count
    );

    if let Some(tx) = progress_tx {
        let _ = tx.send(RebalanceProgressMessage::Complete {
            success_count,
            failed_count,
            results: results.clone(),
        });
    }

    results
}

/// Execute a global rebalance plan (all move groups)
pub fn execute_global_rebalance_plan(
    dataset_path: &PathBuf,
    plan: &GlobalRebalancePlan,
    progress_tx: Option<Sender<RebalanceProgressMessage>>,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> Vec<MoveResult> {
    let mut all_results = Vec::new();
    let total_files = plan.total_moves;
    let mut processed = 0;

    for move_group in &plan.moves {
        // Ensure destination directories exist
        let dest_images = dataset_path.join(move_group.to_split.as_str()).join("images");
        let dest_labels = dataset_path.join(move_group.to_split.as_str()).join("labels");

        if let Err(e) = fs::create_dir_all(&dest_images) {
            error!("Failed to create destination images dir: {}", e);
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(RebalanceProgressMessage::Error(format!(
                    "Failed to create destination directory: {}", e
                )));
            }
            return all_results;
        }
        if let Err(e) = fs::create_dir_all(&dest_labels) {
            error!("Failed to create destination labels dir: {}", e);
        }

        for action in &move_group.actions {
            // Check cancellation
            if let Some(ref cancel) = cancel_flag {
                if cancel.load(Ordering::Relaxed) {
                    warn!("Global rebalance cancelled at {}/{}", processed, total_files);
                    if let Some(ref tx) = progress_tx {
                        let _ = tx.send(RebalanceProgressMessage::Cancelled {
                            completed_count: processed,
                            results: all_results.clone(),
                        });
                    }
                    return all_results;
                }
            }

            let filename = action.image_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            let new_image_path = dest_images.join(filename);
            let new_label_path = action.label_path.as_ref().and_then(|lp| {
                lp.file_name().map(|n| dest_labels.join(n))
            });

            // Move image
            let image_result = move_file(&action.image_path, &new_image_path);
            
            if let Err(e) = image_result {
                error!("Failed to move image {:?}: {}", action.image_path, e);
                all_results.push(MoveResult {
                    action: action.clone(),
                    success: false,
                    error: Some(format!("Failed to move image: {}", e)),
                    new_image_path: None,
                    new_label_path: None,
                });
                continue;
            }

            // Move label if exists
            let mut final_label_path = None;
            if let (Some(src_label), Some(dst_label)) = (&action.label_path, &new_label_path) {
                if src_label.exists() {
                    if let Ok(()) = move_file(src_label, dst_label) {
                        final_label_path = Some(dst_label.clone());
                    }
                }
            }

            all_results.push(MoveResult {
                action: action.clone(),
                success: true,
                error: None,
                new_image_path: Some(new_image_path),
                new_label_path: final_label_path,
            });

            processed += 1;

            // Send progress
            if let Some(ref tx) = progress_tx {
                if processed % 5 == 0 || processed == total_files {
                    let _ = tx.send(RebalanceProgressMessage::Progress {
                        current: processed,
                        total: total_files,
                        last_moved: filename.to_string(),
                    });
                }
            }
        }
    }

    let success_count = all_results.iter().filter(|r| r.success).count();
    let failed_count = all_results.len() - success_count;

    info!(
        "Global rebalance complete: {} succeeded, {} failed",
        success_count, failed_count
    );

    if let Some(tx) = progress_tx {
        let _ = tx.send(RebalanceProgressMessage::Complete {
            success_count,
            failed_count,
            results: all_results.clone(),
        });
    }

    all_results
}
pub fn undo_rebalance(
    results: &[MoveResult],
    progress_tx: Option<Sender<RebalanceProgressMessage>>,
    cancel_flag: Option<Arc<AtomicBool>>,
) -> Vec<MoveResult> {
    let mut undo_results = Vec::new();
    let successful_moves: Vec<_> = results.iter().filter(|r| r.success).collect();
    let total = successful_moves.len();

    if total == 0 {
        if let Some(tx) = progress_tx {
            let _ = tx.send(RebalanceProgressMessage::Complete {
                success_count: 0,
                failed_count: 0,
                results: vec![],
            });
        }
        return undo_results;
    }

    let mut success_count = 0;
    let mut failed_count = 0;

    for (idx, result) in successful_moves.iter().enumerate() {
        // Check cancellation
        if let Some(ref cancel) = cancel_flag {
            if cancel.load(Ordering::Relaxed) {
                warn!("Undo cancelled at {}/{}", idx, total);
                if let Some(ref tx) = progress_tx {
                    let _ = tx.send(RebalanceProgressMessage::Cancelled {
                        completed_count: idx,
                        results: undo_results.clone(),
                    });
                }
                return undo_results;
            }
        }

        let original_action = &result.action;
        let filename = original_action.image_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Move image back
        if let Some(ref new_path) = result.new_image_path {
            if let Err(e) = move_file(new_path, &original_action.image_path) {
                error!("Failed to undo image move: {}", e);
                undo_results.push(MoveResult {
                    action: original_action.clone(),
                    success: false,
                    error: Some(format!("Undo failed: {}", e)),
                    new_image_path: None,
                    new_label_path: None,
                });
                failed_count += 1;
                continue;
            }
        }

        // Move label back
        if let (Some(ref new_label), Some(ref orig_label)) = 
            (&result.new_label_path, &original_action.label_path) 
        {
            if new_label.exists() {
                let _ = move_file(new_label, orig_label);
            }
        }

        undo_results.push(MoveResult {
            action: original_action.clone(),
            success: true,
            error: None,
            new_image_path: Some(original_action.image_path.clone()),
            new_label_path: original_action.label_path.clone(),
        });
        success_count += 1;

        // Send progress update
        if let Some(ref tx) = progress_tx {
            if (idx + 1) % 5 == 0 || idx == total - 1 {
                let _ = tx.send(RebalanceProgressMessage::Progress {
                    current: idx + 1,
                    total,
                    last_moved: filename.to_string(),
                });
            }
        }
    }

    info!(
        "Undo complete: {} succeeded, {} failed",
        success_count, failed_count
    );

    if let Some(tx) = progress_tx {
        let _ = tx.send(RebalanceProgressMessage::Complete {
            success_count,
            failed_count,
            results: undo_results.clone(),
        });
    }

    undo_results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_move_count_excess_background() {
        let stats = BalanceStats {
            total_images: 1000,
            ct_only: 400,
            t_only: 400,
            multiple_player: 50,
            background: 150,  // 15% - target is 10%
            hard_case: 0,
        };
        let target = TargetRatios::default();
        let excess = calculate_move_count(&stats, ImageCategory::Background, &target);
        assert_eq!(excess, 50);  // Need to remove 50 to get to 10%
    }

    #[test]
    fn test_calculate_move_count_no_excess() {
        let stats = BalanceStats {
            total_images: 1000,
            ct_only: 400,
            t_only: 400,
            multiple_player: 50,
            background: 100,  // Exactly 10%
            hard_case: 50,
        };
        let target = TargetRatios::default();
        let excess = calculate_move_count(&stats, ImageCategory::Background, &target);
        assert_eq!(excess, 0);
    }

    #[test]
    fn test_selection_strategy_display() {
        assert_eq!(SelectionStrategy::Random.as_str(), "Random");
        assert_eq!(SelectionStrategy::FewestDetections.as_str(), "Fewest Detections");
    }
}
