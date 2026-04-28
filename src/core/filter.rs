use crate::core::dataset::{parse_label_file, LabelInfo};
use crate::core::operations::get_label_path_for_image;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Team filter options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TeamFilter {
    /// Show all images regardless of team
    #[default]
    All,
    /// Only images with at least one T player
    TOnly,
    /// Only images with at least one CT player
    CTOnly,
    /// Only images with both T and CT players
    Both,
    /// Only images with T players but no CT
    TExclusive,
    /// Only images with CT players but no T
    CTExclusive,
}

/// Player count filter options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlayerCountFilter {
    /// Show all images regardless of player count
    #[default]
    Any,
    /// Only images with exactly one player
    Single,
    /// Only images with multiple players (2+)
    Multiple,
    /// Only images with no players (background)
    Background,
}

/// Filter criteria configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FilterCriteria {
    pub team: TeamFilter,
    pub player_count: PlayerCountFilter,
}

impl FilterCriteria {
    /// Check if any filters are active
    pub fn is_active(&self) -> bool {
        self.team != TeamFilter::All || self.player_count != PlayerCountFilter::Any
    }

    /// Clear all filters
    pub fn clear(&mut self) {
        self.team = TeamFilter::All;
        self.player_count = PlayerCountFilter::Any;
    }
}

/// Analyze label to determine team composition
fn analyze_team_composition(label: &LabelInfo) -> (bool, bool) {
    let mut has_t = false;
    let mut has_ct = false;

    for detection in &label.detections {
        match detection.class_id {
            0 => has_t = true,  // T
            1 => has_ct = true, // CT
            _ => {}
        }
    }

    (has_t, has_ct)
}

/// Check if an image matches the filter criteria
fn matches_criteria(label_info: Option<&LabelInfo>, criteria: &FilterCriteria) -> bool {
    // Handle player count filter for background images
    if criteria.player_count == PlayerCountFilter::Background {
        return label_info.map(|l| l.detections.is_empty()).unwrap_or(true);
    }

    // If no label info and not looking for background, doesn't match
    let label = match label_info {
        Some(l) => l,
        None => return false,
    };

    let player_count = label.detections.len();

    // Check player count filter
    let count_match = match criteria.player_count {
        PlayerCountFilter::Any => player_count > 0,
        PlayerCountFilter::Single => player_count == 1,
        PlayerCountFilter::Multiple => player_count >= 2,
        PlayerCountFilter::Background => player_count == 0,
    };

    if !count_match {
        return false;
    }

    // Check team filter
    let (has_t, has_ct) = analyze_team_composition(label);

    match criteria.team {
        TeamFilter::All => true,
        TeamFilter::TOnly => has_t,
        TeamFilter::CTOnly => has_ct,
        TeamFilter::Both => has_t && has_ct,
        TeamFilter::TExclusive => has_t && !has_ct,
        TeamFilter::CTExclusive => has_ct && !has_t,
    }
}

/// Apply filters to a list of image files and return filtered indices
///
/// # Arguments
/// * `image_files` - List of all image file paths
/// * `criteria` - Filter criteria to apply
///
/// # Returns
/// * Vector of indices that match the filter criteria
pub fn apply_filters(image_files: &[PathBuf], criteria: &FilterCriteria) -> Vec<usize> {
    if !criteria.is_active() {
        // No filters active, return all indices
        return (0..image_files.len()).collect();
    }

    image_files
        .iter()
        .enumerate()
        .filter_map(|(idx, img_path)| {
            // Get label path and parse it
            let label_path = get_label_path_for_image(img_path)?;
            let label_info = parse_label_file(&label_path);

            // Check if matches criteria
            if matches_criteria(label_info.as_ref(), criteria) {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dataset::YoloDetection;

    fn create_test_label(class_ids: Vec<u32>) -> LabelInfo {
        LabelInfo {
            detections: class_ids
                .into_iter()
                .map(|id| YoloDetection {
                    class_id: id,
                    x_center: 0.5,
                    y_center: 0.5,
                    width: 0.1,
                    height: 0.1,
                })
                .collect(),
            resolution: None,
            map: None,
            timestamp: None,
        }
    }

    #[test]
    fn test_team_filter_t_only() {
        let label = create_test_label(vec![0]); // T player
        let criteria = FilterCriteria {
            team: TeamFilter::TOnly,
            player_count: PlayerCountFilter::Any,
        };
        assert!(matches_criteria(Some(&label), &criteria));
    }

    #[test]
    fn test_team_filter_both() {
        let label = create_test_label(vec![0, 1]); // T and CT
        let criteria = FilterCriteria {
            team: TeamFilter::Both,
            player_count: PlayerCountFilter::Any,
        };
        assert!(matches_criteria(Some(&label), &criteria));
    }

    #[test]
    fn test_player_count_single() {
        let label = create_test_label(vec![0]);
        let criteria = FilterCriteria {
            team: TeamFilter::All,
            player_count: PlayerCountFilter::Single,
        };
        assert!(matches_criteria(Some(&label), &criteria));
    }

    #[test]
    fn test_player_count_multiple() {
        let label = create_test_label(vec![0, 1]);
        let criteria = FilterCriteria {
            team: TeamFilter::All,
            player_count: PlayerCountFilter::Multiple,
        };
        assert!(matches_criteria(Some(&label), &criteria));
    }

    #[test]
    fn test_background_filter() {
        let label = create_test_label(vec![]);
        let criteria = FilterCriteria {
            team: TeamFilter::All,
            player_count: PlayerCountFilter::Background,
        };
        assert!(matches_criteria(Some(&label), &criteria));
    }
}
