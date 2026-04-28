use crate::state::FilterState;

/// Handles navigation logic for both filtered and unfiltered image browsing
pub struct Navigator {
    total_images: usize,
}

impl Navigator {
    /// Create a new Navigator with the total number of images
    pub fn new(total_images: usize) -> Self {
        Self { total_images }
    }

    /// Calculate the next image index
    pub fn next(&self, current_index: usize, filter: &FilterState) -> Option<usize> {
        if filter.is_active() {
            // Navigate through filtered list
            if let Some(current_virtual) = filter.get_filtered_index(current_index) {
                let next_virtual = current_virtual + 1;
                if next_virtual < filter.filtered_count() {
                    return filter.get_actual_index(next_virtual);
                }
            }
            None
        } else {
            // Normal navigation
            if self.total_images > 0 && current_index < self.total_images - 1 {
                Some(current_index + 1)
            } else {
                None
            }
        }
    }

    /// Calculate the previous image index
    pub fn prev(&self, current_index: usize, filter: &FilterState) -> Option<usize> {
        if filter.is_active() {
            // Navigate through filtered list
            if let Some(current_virtual) = filter.get_filtered_index(current_index) {
                if current_virtual > 0 {
                    return filter.get_actual_index(current_virtual - 1);
                }
            }
            None
        } else {
            // Normal navigation
            if current_index > 0 {
                Some(current_index - 1)
            } else {
                None
            }
        }
    }

    /// Calculate the first image index
    pub fn first(&self, filter: &FilterState) -> Option<usize> {
        if filter.is_active() {
            // Jump to first filtered image
            if !filter.filtered_indices.is_empty() {
                filter.get_actual_index(0)
            } else {
                None
            }
        } else {
            // Normal jump
            if self.total_images > 0 {
                Some(0)
            } else {
                None
            }
        }
    }

    /// Calculate the last image index
    pub fn last(&self, filter: &FilterState) -> Option<usize> {
        if filter.is_active() {
            // Jump to last filtered image
            let count = filter.filtered_count();
            if count > 0 {
                filter.get_actual_index(count - 1)
            } else {
                None
            }
        } else {
            // Normal jump
            if self.total_images > 0 {
                Some(self.total_images - 1)
            } else {
                None
            }
        }
    }

    /// Calculate index after jumping by offset (positive = forward, negative = backward)
    pub fn jump_by_offset(
        &self,
        current_index: usize,
        offset: isize,
        filter: &FilterState,
    ) -> Option<usize> {
        if filter.is_active() {
            // Jump through filtered list
            if let Some(current_virtual) = filter.get_filtered_index(current_index) {
                let total_filtered = filter.filtered_count();
                let new_virtual = if offset < 0 {
                    current_virtual.saturating_sub((-offset) as usize)
                } else {
                    (current_virtual + offset as usize).min(total_filtered.saturating_sub(1))
                };
                filter.get_actual_index(new_virtual)
            } else {
                None
            }
        } else {
            // Normal jump
            if self.total_images == 0 {
                return None;
            }

            let new_index = if offset < 0 {
                // Jump backward
                current_index.saturating_sub((-offset) as usize)
            } else {
                // Jump forward
                (current_index + offset as usize).min(self.total_images - 1)
            };

            Some(new_index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_without_filter() {
        let nav = Navigator::new(5);
        let filter = FilterState::new();

        // Test next
        assert_eq!(nav.next(0, &filter), Some(1));
        assert_eq!(nav.next(4, &filter), None);

        // Test prev
        assert_eq!(nav.prev(1, &filter), Some(0));
        assert_eq!(nav.prev(0, &filter), None);

        // Test first/last
        assert_eq!(nav.first(&filter), Some(0));
        assert_eq!(nav.last(&filter), Some(4));

        // Test jump by offset
        assert_eq!(nav.jump_by_offset(2, 2, &filter), Some(4));
        assert_eq!(nav.jump_by_offset(2, -2, &filter), Some(0));
    }

    #[test]
    fn test_navigation_empty() {
        let nav = Navigator::new(0);
        let filter = FilterState::new();

        assert_eq!(nav.next(0, &filter), None);
        assert_eq!(nav.prev(0, &filter), None);
        assert_eq!(nav.first(&filter), None);
        assert_eq!(nav.last(&filter), None);
    }
}
