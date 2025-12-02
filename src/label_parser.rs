use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct YoloDetection {
    pub class_id: u32,
    pub x_center: f32,
    pub y_center: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct LabelInfo {
    pub detections: Vec<YoloDetection>,
    pub resolution: Option<String>,
    pub map: Option<String>,
    pub timestamp: Option<String>,
}

/// Parse a YOLO format label file and return the label information.
/// 
/// # Arguments
/// * `label_path` - Path to the label file (.txt)
/// 
/// # Returns
/// * `Some(LabelInfo)` if the file exists and can be parsed
/// * `None` if the file doesn't exist or cannot be read
pub fn parse_label_file(label_path: &PathBuf) -> Option<LabelInfo> {
    // Read and parse label file
    let content = fs::read_to_string(label_path).ok()?;
    
    let mut detections = Vec::new();
    let mut resolution = None;
    let mut map = None;
    let mut timestamp = None;
    
    for line in content.lines() {
        let line = line.trim();
        
        // Parse metadata from comment line
        // Format: # Resolution: 2560x1440, Map: de_dust2, Time: 1764637338
        if line.starts_with('#') {
            let parts: Vec<&str> = line[1..].split(',').collect();
            for part in parts {
                let part = part.trim();
                if let Some(res) = part.strip_prefix("Resolution:") {
                    resolution = Some(res.trim().to_string());
                } else if let Some(m) = part.strip_prefix("Map:") {
                    map = Some(m.trim().to_string());
                } else if let Some(t) = part.strip_prefix("Time:") {
                    timestamp = Some(t.trim().to_string());
                }
            }
        } else if !line.is_empty() {
            // Parse detection line
            // Format: class_id x_center y_center width height
            let values: Vec<&str> = line.split_whitespace().collect();
            if values.len() == 5 {
                if let (Ok(class_id), Ok(x), Ok(y), Ok(w), Ok(h)) = (
                    values[0].parse::<u32>(),
                    values[1].parse::<f32>(),
                    values[2].parse::<f32>(),
                    values[3].parse::<f32>(),
                    values[4].parse::<f32>(),
                ) {
                    detections.push(YoloDetection {
                        class_id,
                        x_center: x,
                        y_center: y,
                        width: w,
                        height: h,
                    });
                }
            }
        }
    }
    
    Some(LabelInfo {
        detections,
        resolution,
        map,
        timestamp,
    })
}
