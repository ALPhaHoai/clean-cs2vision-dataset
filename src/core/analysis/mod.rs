mod balance_analyzer;
mod rebalancer;

pub use balance_analyzer::{
    analyze_dataset, analyze_dataset_with_progress, categorize_image, get_recommendations,
    BalanceProgressMessage, BalanceStats, ImageCategory, TargetRatios,
    // Integrity analysis exports
    analyze_dataset_integrity, analyze_dataset_integrity_with_progress,
    IntegrityIssue, IntegrityIssueType, IntegrityProgressMessage, IntegrityStats,
};

pub use rebalancer::{
    calculate_move_count, calculate_rebalance_plan, collect_image_metadata,
    execute_rebalance_plan, find_best_destination_split, undo_rebalance,
    analyze_all_splits, calculate_global_rebalance_plan, execute_global_rebalance_plan,
    ImageMetadata, MoveAction, MoveResult, RebalanceConfig, RebalancePlan, 
    RebalanceProgressMessage, SelectionStrategy, SplitRatios,
    GlobalBalanceStats, GlobalMoveAction, GlobalRebalancePlan, GlobalRebalanceConfig,
};
