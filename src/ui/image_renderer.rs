use crate::config::AppConfig;
use crate::core::dataset::{LabelInfo, YoloDetection};
use eframe::egui::{self, Color32, Painter, Rect, Vec2};

/// Image rendering utilities for displaying images and bounding boxes
pub struct ImageRenderer;

impl ImageRenderer {
    /// Calculate the scaling factor to fit an image within the available space
    /// while maintaining aspect ratio and not exceeding 1.0 (no upscaling)
    pub fn calculate_image_scale(img_size: Vec2, available_size: Vec2) -> f32 {
        (available_size.x / img_size.x)
            .min(available_size.y / img_size.y)
            .min(1.0)
    }

    /// Draw all bounding boxes from label data onto the image
    ///
    /// # Arguments
    /// * `painter` - The egui Painter to draw with
    /// * `label` - The label information containing detections
    /// * `image_rect` - The rectangle where the image is displayed on screen
    /// * `actual_image_size` - The actual loaded image dimensions
    /// * `config` - Application configuration for class names and colors
    pub fn draw_bounding_boxes(
        painter: &Painter,
        label: &LabelInfo,
        image_rect: Rect,
        actual_image_size: Vec2,
        config: &AppConfig,
    ) {
        // Parse the original resolution from label metadata if available
        // This is the resolution the YOLO coordinates were generated for
        let original_resolution = Self::parse_resolution_from_label(label);
        
        // Get the displayed image size from the rect
        let displayed_size = image_rect.size();
        
        for (i, detection) in label.detections.iter().enumerate() {
            Self::draw_single_box(
                painter,
                detection,
                i,
                image_rect,
                original_resolution,
                actual_image_size,
                displayed_size,
                config,
            );
        }
    }

    /// Parse resolution from label metadata (e.g., "2560x1440")
    /// Returns the resolution as Vec2 if found, otherwise None
    fn parse_resolution_from_label(label: &LabelInfo) -> Option<Vec2> {
        if let Some(res_str) = &label.resolution {
            let parts: Vec<&str> = res_str.split('x').collect();
            if parts.len() == 2 {
                if let (Ok(width), Ok(height)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                    return Some(Vec2::new(width, height));
                }
            }
        }
        None
    }

    /// Draw a single bounding box with label text
    ///
    /// # Arguments
    /// * `painter` - The egui Painter to draw with
    /// * `detection` - The detection to draw
    /// * `index` - The index of the detection (0-based)
    /// * `image_rect` - The rectangle where the image is displayed on screen
    /// * `original_resolution` - The resolution the YOLO coords were generated for (from metadata)
    /// * `actual_image_size` - The actual current image file dimensions
    /// * `displayed_size` - The size of the displayed image on screen
    /// * `config` - Application configuration for class names and colors
    fn draw_single_box(
        painter: &Painter,
        detection: &YoloDetection,
        index: usize,
        image_rect: Rect,
        original_resolution: Option<Vec2>,
        actual_image_size: Vec2,
        displayed_size: Vec2,
        config: &AppConfig,
    ) {
        // YOLO coordinates are normalized (0-1) relative to the ORIGINAL resolution
        // We need to: normalized -> original pixels -> actual pixels -> displayed pixels
        
        // Use original resolution if available, otherwise use actual image size
        let reference_size = original_resolution.unwrap_or(actual_image_size);
        
        // Step 1: Convert normalized YOLO coordinates to pixel coordinates in the original resolution
        let pixel_center_x = detection.x_center * reference_size.x;
        let pixel_center_y = detection.y_center * reference_size.y;
        let pixel_width = detection.width * reference_size.x;
        let pixel_height = detection.height * reference_size.y;
        
        // Step 2: Scale from original resolution to actual image size (if different)
        let scale_to_actual_x = actual_image_size.x / reference_size.x;
        let scale_to_actual_y = actual_image_size.y / reference_size.y;
        
        let actual_center_x = pixel_center_x * scale_to_actual_x;
        let actual_center_y = pixel_center_y * scale_to_actual_y;
        let actual_width = pixel_width * scale_to_actual_x;
        let actual_height = pixel_height * scale_to_actual_y;
        
        // Step 3: Scale from actual image size to displayed size
        let scale_to_display_x = displayed_size.x / actual_image_size.x;
        let scale_to_display_y = displayed_size.y / actual_image_size.y;
        
        let bbox_center_x = actual_center_x * scale_to_display_x;
        let bbox_center_y = actual_center_y * scale_to_display_y;
        let bbox_width = actual_width * scale_to_display_x;
        let bbox_height = actual_height * scale_to_display_y;

        // Calculate top-left corner
        let bbox_x = bbox_center_x - (bbox_width / 2.0);
        let bbox_y = bbox_center_y - (bbox_height / 2.0);

        // Create rect in screen space (offset by image position)
        let bbox_rect = Rect::from_min_size(
            egui::pos2(image_rect.min.x + bbox_x, image_rect.min.y + bbox_y),
            egui::vec2(bbox_width, bbox_height),
        );

        // Get colors for this class from config
        let (stroke_color, fill_color) = config.get_class_colors(detection.class_id);

        // Draw filled rectangle
        painter.rect_filled(bbox_rect, 0.0, fill_color);

        // Draw border
        painter.rect_stroke(bbox_rect, 0.0, egui::Stroke::new(2.0, stroke_color));

        // Draw label text
        let class_name = config.get_class_name(detection.class_id);
        let label_text = format!("{} #{}", class_name, index + 1);
        let font_id = egui::FontId::proportional(14.0);
        let text_galley = painter.layout_no_wrap(label_text, font_id, Color32::WHITE);

        // Draw text background
        let text_pos = bbox_rect.min + egui::vec2(2.0, -18.0);
        let text_bg_rect =
            Rect::from_min_size(text_pos, egui::vec2(text_galley.size().x + 6.0, 16.0));
        painter.rect_filled(text_bg_rect, 2.0, stroke_color);

        // Draw text
        painter.galley(text_pos + egui::vec2(3.0, 0.0), text_galley, Color32::WHITE);
    }
}
