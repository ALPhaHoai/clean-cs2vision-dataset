use crate::config::AppConfig;
use crate::label_parser::{LabelInfo, YoloDetection};
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
    /// * `scaled_size` - The scaled size of the image
    /// * `config` - Application configuration for class names and colors
    pub fn draw_bounding_boxes(
        painter: &Painter,
        label: &LabelInfo,
        image_rect: Rect,
        scaled_size: Vec2,
        config: &AppConfig,
    ) {
        for (i, detection) in label.detections.iter().enumerate() {
            Self::draw_single_box(painter, detection, i, image_rect, scaled_size, config);
        }
    }

    /// Draw a single bounding box with label text
    ///
    /// # Arguments
    /// * `painter` - The egui Painter to draw with
    /// * `detection` - The detection to draw
    /// * `index` - The index of the detection (0-based)
    /// * `image_rect` - The rectangle where the image is displayed on screen
    /// * `scaled_size` - The scaled size of the image
    /// * `config` - Application configuration for class names and colors
    fn draw_single_box(
        painter: &Painter,
        detection: &YoloDetection,
        index: usize,
        image_rect: Rect,
        scaled_size: Vec2,
        config: &AppConfig,
    ) {
        // Convert normalized YOLO coordinates to screen coordinates
        // YOLO format: center_x, center_y, width, height (all normalized 0-1)
        let bbox_center_x = detection.x_center * scaled_size.x;
        let bbox_center_y = detection.y_center * scaled_size.y;
        let bbox_width = detection.width * scaled_size.x;
        let bbox_height = detection.height * scaled_size.y;

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
