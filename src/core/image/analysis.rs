use image::DynamicImage;
use kmeans_colors::get_kmeans;
use palette::{FromColor, Lab, Srgb};

/// RGB threshold value below which a color is considered "near black"
pub const BLACK_THRESHOLD: f32 = 10.0;

/// Calculates the dominant color in an image using k-means clustering
/// 
/// This function samples pixels from the image (up to 10,000 samples) and uses
/// k-means clustering in the LAB color space to identify the most common colors.
/// Returns the RGB values of the most dominant color.
pub fn calculate_dominant_color(img: &DynamicImage) -> Option<(u8, u8, u8)> {
    // Convert image to RGB
    let img_rgb = img.to_rgb8();
    let (width, height) = img_rgb.dimensions();
    
    // Sample pixels (to avoid processing too many pixels)
    // Increased to 100k for better accuracy on 640x640 CS2 screenshots (~24% coverage)
    let max_samples = 100000;
    let step = ((width * height) as f32 / max_samples as f32).sqrt().ceil() as u32;
    let step = step.max(1);
    
    let mut lab_pixels: Vec<Lab> = Vec::new();
    
    for y in (0..height).step_by(step as usize) {
        for x in (0..width).step_by(step as usize) {
            let pixel = img_rgb.get_pixel(x, y);
            let rgb = Srgb::new(
                pixel[0] as f32 / 255.0,
                pixel[1] as f32 / 255.0,
                pixel[2] as f32 / 255.0,
            );
            lab_pixels.push(Lab::from_color(rgb));
        }
    }
    
    if lab_pixels.is_empty() {
        return None;
    }
    
    // Run k-means with k=3 to find dominant colors
    let k = 3;
    let max_iter = 20;
    let converge = 1.0;
    let verbose = false;
    let seed = 0;
    
    let result = get_kmeans(
        k,
        max_iter,
        converge,
        verbose,
        &lab_pixels,
        seed,
    );
    
    // Get the centroid with the most members (dominant color)
    let mut centroids_with_counts: Vec<_> = result.centroids
        .iter()
        .enumerate()
        .map(|(i, centroid)| {
            let count = result.indices.iter().filter(|&&idx| idx == i as u8).count();
            (centroid, count)
        })
        .collect();
    
    centroids_with_counts.sort_by(|a, b| b.1.cmp(&a.1));
    
    if let Some((dominant_lab, _)) = centroids_with_counts.first() {
        let rgb: Srgb = Srgb::from_color(**dominant_lab);
        let r = (rgb.red * 255.0).clamp(0.0, 255.0) as u8;
        let g = (rgb.green * 255.0).clamp(0.0, 255.0) as u8;
        let b = (rgb.blue * 255.0).clamp(0.0, 255.0) as u8;
        
        Some((r, g, b))
    } else {
        None
    }
}

/// Determines if a color is near black based on threshold
/// 
/// A color is considered near black if all RGB values are below BLACK_THRESHOLD
pub fn is_near_black(color: (u8, u8, u8)) -> bool {
    let (r, g, b) = color;
    let r_f = r as f32;
    let g_f = g as f32;
    let b_f = b as f32;
    
    // Check if all RGB values are below the threshold
    r_f < BLACK_THRESHOLD && g_f < BLACK_THRESHOLD && b_f < BLACK_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_near_black() {
        assert!(is_near_black((0, 0, 0))); // Pure black
        assert!(is_near_black((5, 5, 5))); // Near black
        assert!(is_near_black((9, 9, 9))); // Near black
        assert!(!is_near_black((10, 10, 10))); // Not near black
        assert!(!is_near_black((50, 50, 50))); // Gray
        assert!(!is_near_black((255, 255, 255))); // White
        assert!(!is_near_black((9, 9, 15))); // One channel above threshold
    }
}
