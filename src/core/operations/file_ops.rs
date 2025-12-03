use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// Result type for file operations
pub type FileOpResult<T> = Result<T, FileOpError>;

/// Error types for file operations
#[derive(Debug)]
pub enum FileOpError {
    CopyFailed(String),
    RemoveFailed(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for FileOpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOpError::CopyFailed(msg) => write!(f, "Copy failed: {}", msg),
            FileOpError::RemoveFailed(msg) => write!(f, "Remove failed: {}", msg),
            FileOpError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for FileOpError {}

impl From<std::io::Error> for FileOpError {
    fn from(error: std::io::Error) -> Self {
        FileOpError::IoError(error)
    }
}

/// Move a file from source to destination using copy + remove pattern
/// for cross-drive compatibility.
///
/// # Arguments
/// * `src` - Source file path
/// * `dest` - Destination file path
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err(FileOpError)` if copy or remove failed
pub fn move_file(src: &PathBuf, dest: &PathBuf) -> FileOpResult<()> {
    info!("Moving file from {:?} to {:?}", src, dest);

    // Copy the file to the destination
    if let Err(e) = fs::copy(src, dest) {
        error!("Failed to copy file from {:?} to {:?}: {}", src, dest, e);
        return Err(FileOpError::CopyFailed(format!(
            "Failed to copy from {:?} to {:?}: {}",
            src, dest, e
        )));
    }

    // Remove the original file after successful copy
    if let Err(e) = fs::remove_file(src) {
        error!("Failed to remove original file {:?} after copy: {}", src, e);
        // Try to clean up the destination file
        let _ = fs::remove_file(dest);
        return Err(FileOpError::RemoveFailed(format!(
            "Failed to remove original file {:?}: {}",
            src, e
        )));
    }

    info!("File moved successfully");
    Ok(())
}

/// Restore a file from temporary location back to its original location.
/// This is essentially the reverse of `move_file`.
///
/// # Arguments
/// * `temp_path` - Temporary file path
/// * `original_path` - Original file path to restore to
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err(FileOpError)` if restoration failed
pub fn restore_file(temp_path: &PathBuf, original_path: &PathBuf) -> FileOpResult<()> {
    info!("Restoring file from {:?} to {:?}", temp_path, original_path);
    move_file(temp_path, original_path)
}

/// Get the corresponding label file path for an image file path.
/// Converts `/images/` or `\images\` directory to `/labels/` or `\labels/`
/// and changes extension to `.txt`.
///
/// # Arguments
/// * `image_path` - Path to the image file
///
/// # Returns
/// * `Some(PathBuf)` if the path conversion was successful
/// * `None` if the path couldn't be converted to a string
pub fn get_label_path_for_image(image_path: &Path) -> Option<PathBuf> {
    image_path.to_str().map(|img_str| {
        let label_str = img_str
            .replace("\\images\\", "\\labels\\")
            .replace("/images/", "/labels/");
        PathBuf::from(label_str).with_extension("txt")
    })
}

/// Delete an image file and its corresponding label file if it exists.
/// Moves both files to a temporary directory for potential undo.
///
/// # Arguments
/// * `image_path` - Path to the image file
/// * `temp_dir` - Temporary directory to move files to
/// * `timestamp` - Unique timestamp for naming temp files
///
/// # Returns
/// * `Ok((temp_image_path, temp_label_path))` with paths to the moved files
/// * `Err(FileOpError)` if the operation failed
pub fn delete_image_with_label(
    image_path: &PathBuf,
    temp_dir: &PathBuf,
    timestamp: u128,
) -> FileOpResult<(PathBuf, Option<PathBuf>)> {
    // Get image filename for temp path
    let image_filename = image_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| FileOpError::CopyFailed("Failed to get image filename".to_string()))?;

    // Create temp image path
    let temp_image_name = format!("{}_{}", timestamp, image_filename);
    let temp_image_path = temp_dir.join(&temp_image_name);

    // Move image to temp location
    move_file(image_path, &temp_image_path)?;

    // Handle label file if it exists
    let temp_label_path = if let Some(label_path) = get_label_path_for_image(image_path) {
        if label_path.exists() {
            info!("Label file exists, moving to temp: {:?}", label_path);
            let temp_label_name = format!(
                "{}_{}",
                timestamp,
                label_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("label.txt")
            );
            let temp_lbl = temp_dir.join(&temp_label_name);

            match move_file(&label_path, &temp_lbl) {
                Ok(_) => {
                    info!("Label moved to temp successfully: {:?}", temp_lbl);
                    Some(temp_lbl)
                }
                Err(e) => {
                    error!("Failed to move label to temp: {}", e);
                    None
                }
            }
        } else {
            info!("Label file doesn't exist");
            None
        }
    } else {
        info!("No label path computed");
        None
    };

    Ok((temp_image_path, temp_label_path))
}

/// Restore an image and its label from temporary locations.
///
/// # Arguments
/// * `temp_image_path` - Temporary image file path
/// * `original_image_path` - Original image file path
/// * `temp_label_path` - Optional temporary label file path
/// * `original_label_path` - Optional original label file path
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err(FileOpError)` if restoration failed
pub fn restore_image_with_label(
    temp_image_path: &PathBuf,
    original_image_path: &PathBuf,
    temp_label_path: &Option<PathBuf>,
    original_label_path: &Option<PathBuf>,
) -> FileOpResult<()> {
    // Restore image file
    restore_file(temp_image_path, original_image_path)?;

    // Restore label file if it exists
    if let (Some(temp_label), Some(orig_label)) = (temp_label_path, original_label_path) {
        if let Err(e) = restore_file(temp_label, orig_label) {
            error!("Failed to restore label file: {}", e);
            // Image was already restored, so we continue despite label error
        }
    }

    Ok(())
}
