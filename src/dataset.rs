use std::path::PathBuf;
use std::fs;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DatasetSplit {
    Train,
    Val,
    Test,
}

impl DatasetSplit {
    pub fn as_str(&self) -> &str {
        match self {
            DatasetSplit::Train => "train",
            DatasetSplit::Val => "val",
            DatasetSplit::Test => "test",
        }
    }
}

pub struct Dataset {
    dataset_path: Option<PathBuf>,
    current_split: DatasetSplit,
    image_files: Vec<PathBuf>,
}

impl Dataset {
    pub fn new() -> Self {
        Self {
            dataset_path: None,
            current_split: DatasetSplit::Train,
            image_files: Vec::new(),
        }
    }
    
    pub fn load(&mut self, path: PathBuf) {
        self.dataset_path = Some(path);
        self.load_current_split();
    }
    
    pub fn load_current_split(&mut self) {
        self.image_files.clear();
        
        if let Some(base_path) = &self.dataset_path {
            // Navigate to split/images folder
            let images_path = base_path
                .join(self.current_split.as_str())
                .join("images");
            
            // Load all image files from the split directory
            if let Ok(entries) = fs::read_dir(&images_path) {
                info!("Reading images from: {:?}", images_path);
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        let ext = ext.to_string_lossy().to_lowercase();
                        if ext == "png" || ext == "jpg" || ext == "jpeg" {
                            self.image_files.push(path);
                        }
                    }
                }
                info!("Found {} images in {:?}", self.image_files.len(), images_path);
            } else {
                warn!("Failed to read directory: {:?}", images_path);
            }
            
            // Sort files for consistent ordering
            self.image_files.sort();
        }
    }
    
    pub fn change_split(&mut self, new_split: DatasetSplit) {
        if self.current_split != new_split {
            self.current_split = new_split;
            self.load_current_split();
        }
    }
    
    pub fn get_image_files(&self) -> &Vec<PathBuf> {
        &self.image_files
    }
    
    pub fn current_split(&self) -> DatasetSplit {
        self.current_split
    }
    
    pub fn dataset_path(&self) -> Option<&PathBuf> {
        self.dataset_path.as_ref()
    }
}
