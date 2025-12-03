use std::path::PathBuf;
use std::time::Instant;

/// Represents a single deletion that can be undone or redone
#[derive(Clone)]
pub struct UndoState {
    pub image_path: PathBuf,
    pub label_path: Option<PathBuf>,
    pub image_filename: String,
    pub deleted_at: Instant,
    pub temp_image_path: PathBuf,
    pub temp_label_path: Option<PathBuf>,
}

/// Manages undo and redo stacks for image deletion operations
pub struct UndoManager {
    undo_stack: Vec<UndoState>,
    redo_stack: Vec<UndoState>,
}

impl UndoManager {
    /// Create a new empty undo manager
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Push a new deletion onto the undo stack and clear the redo stack
    /// This is called when a user deletes an image
    pub fn push_delete(&mut self, state: UndoState) {
        self.undo_stack.push(state);
        // Clear redo stack when a new action is performed (standard behavior)
        self.redo_stack.clear();
    }

    /// Pop the most recent deletion from the undo stack
    /// Returns the state to restore, and pushes it onto the redo stack
    pub fn undo(&mut self) -> Option<UndoState> {
        if let Some(state) = self.undo_stack.pop() {
            let state_clone = state.clone();
            self.redo_stack.push(state);
            Some(state_clone)
        } else {
            None
        }
    }

    /// Pop the most recent undo from the redo stack
    /// Returns the state to re-delete, and pushes it onto the undo stack
    pub fn redo(&mut self) -> Option<UndoState> {
        if let Some(state) = self.redo_stack.pop() {
            let state_clone = state.clone();
            self.undo_stack.push(state);
            Some(state_clone)
        } else {
            None
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the number of available undos
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of available redos
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear all undo and redo history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get a reference to the undo stack (for cleanup operations)
    pub fn undo_stack(&self) -> &Vec<UndoState> {
        &self.undo_stack
    }

    /// Get a reference to the redo stack (for cleanup operations)
    pub fn redo_stack(&self) -> &Vec<UndoState> {
        &self.redo_stack
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Instant;

    fn create_test_undo_state(filename: &str) -> UndoState {
        UndoState {
            image_path: PathBuf::from(format!("/images/{}", filename)),
            label_path: Some(PathBuf::from(format!("/labels/{}.txt", filename))),
            image_filename: filename.to_string(),
            deleted_at: Instant::now(),
            temp_image_path: PathBuf::from(format!("/temp/{}", filename)),
            temp_label_path: Some(PathBuf::from(format!("/temp/{}.txt", filename))),
        }
    }

    #[test]
    fn test_new_manager_is_empty() {
        let manager = UndoManager::new();
        assert_eq!(manager.undo_count(), 0);
        assert_eq!(manager.redo_count(), 0);
        assert!(!manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_push_delete_adds_to_undo_stack() {
        let mut manager = UndoManager::new();
        let state = create_test_undo_state("test1.jpg");

        manager.push_delete(state);

        assert_eq!(manager.undo_count(), 1);
        assert_eq!(manager.redo_count(), 0);
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_undo_moves_to_redo_stack() {
        let mut manager = UndoManager::new();
        let state = create_test_undo_state("test1.jpg");

        manager.push_delete(state);
        let undone = manager.undo();

        assert!(undone.is_some());
        assert_eq!(manager.undo_count(), 0);
        assert_eq!(manager.redo_count(), 1);
        assert!(!manager.can_undo());
        assert!(manager.can_redo());
    }

    #[test]
    fn test_redo_moves_to_undo_stack() {
        let mut manager = UndoManager::new();
        let state = create_test_undo_state("test1.jpg");

        manager.push_delete(state);
        manager.undo();
        let redone = manager.redo();

        assert!(redone.is_some());
        assert_eq!(manager.undo_count(), 1);
        assert_eq!(manager.redo_count(), 0);
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_new_delete_clears_redo_stack() {
        let mut manager = UndoManager::new();

        // Delete, undo to populate redo stack
        manager.push_delete(create_test_undo_state("test1.jpg"));
        manager.undo();
        assert_eq!(manager.redo_count(), 1);

        // New delete should clear redo stack
        manager.push_delete(create_test_undo_state("test2.jpg"));
        assert_eq!(manager.redo_count(), 0);
        assert_eq!(manager.undo_count(), 1);
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut manager = UndoManager::new();

        // Delete 3 images
        manager.push_delete(create_test_undo_state("test1.jpg"));
        manager.push_delete(create_test_undo_state("test2.jpg"));
        manager.push_delete(create_test_undo_state("test3.jpg"));

        assert_eq!(manager.undo_count(), 3);

        // Undo 2
        manager.undo();
        manager.undo();
        assert_eq!(manager.undo_count(), 1);
        assert_eq!(manager.redo_count(), 2);

        // Redo 1
        manager.redo();
        assert_eq!(manager.undo_count(), 2);
        assert_eq!(manager.redo_count(), 1);
    }

    #[test]
    fn test_undo_when_empty_returns_none() {
        let mut manager = UndoManager::new();
        assert!(manager.undo().is_none());
    }

    #[test]
    fn test_redo_when_empty_returns_none() {
        let mut manager = UndoManager::new();
        assert!(manager.redo().is_none());
    }

    #[test]
    fn test_clear() {
        let mut manager = UndoManager::new();

        manager.push_delete(create_test_undo_state("test1.jpg"));
        manager.push_delete(create_test_undo_state("test2.jpg"));
        manager.undo();

        assert_eq!(manager.undo_count(), 1);
        assert_eq!(manager.redo_count(), 1);

        manager.clear();

        assert_eq!(manager.undo_count(), 0);
        assert_eq!(manager.redo_count(), 0);
        assert!(!manager.can_undo());
        assert!(!manager.can_redo());
    }
}
