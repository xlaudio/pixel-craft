//! # document — Tela, livelli e modifiche al documento
//!
//! Contiene il dimensionamento della tela e il compositing. Le operazioni
//! specifiche sono separate in [`layers`] e [`selection`].

use eframe::egui;
use egui::{Color32, ColorImage};

use crate::model::{Layer, PaintApp};

const MAX_CANVAS_DIMENSION: usize = 16_384;
const MAX_CANVAS_PIXELS: usize = 100_000_000;

mod layers;
mod selection;

pub(super) fn valid_canvas_dimensions(width: usize, height: usize) -> bool {
    width > 0
        && height > 0
        && width <= MAX_CANVAS_DIMENSION
        && height <= MAX_CANVAS_DIMENSION
        && width
            .checked_mul(height)
            .is_some_and(|pixels| pixels <= MAX_CANVAS_PIXELS)
}

fn resize_layer_pixels(
    pixels: &[Color32],
    old_width: usize,
    old_height: usize,
    width: usize,
    height: usize,
) -> Vec<Color32> {
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for pixel in pixels {
        rgba.extend_from_slice(&[pixel.r(), pixel.g(), pixel.b(), pixel.a()]);
    }
    let source = image::RgbaImage::from_raw(old_width as u32, old_height as u32, rgba)
        .expect("layer pixel buffer must match canvas dimensions");
    image::imageops::resize(
        &source,
        width as u32,
        height as u32,
        image::imageops::FilterType::Lanczos3,
    )
    .pixels()
    .map(|pixel| Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]))
    .collect()
}

impl PaintApp {
    pub fn new_canvas(&mut self, width: usize, height: usize, notices: &mut Vec<String>) {
        if !valid_canvas_dimensions(width, height) {
            notices.push("Dimensioni immagine non valide (massimo 100 megapixel)".to_string());
            return;
        }
        self.push_undo_snapshot();
        self.width = width;
        self.height = height;
        let mut bg_layer = Layer::new("Sfondo", width, height);
        bg_layer.pixels.fill(Color32::WHITE);
        let draw_layer = Layer::new("Livello 1", width, height);
        self.layers = vec![bg_layer, draw_layer];
        self.active_layer_idx = 1;
        self.selection = None;
        self.current_filepath = None;
        self.mark_document_changed();
    }

    pub fn resize_canvas(&mut self, width: usize, height: usize, notices: &mut Vec<String>) {
        if !valid_canvas_dimensions(width, height) {
            notices.push("Dimensioni immagine non valide (massimo 100 megapixel)".to_string());
            return;
        }
        if width == self.width && height == self.height {
            return;
        }

        self.push_undo_snapshot();
        let old_width = self.width;
        let old_height = self.height;

        for layer in &mut self.layers {
            layer.pixels = resize_layer_pixels(&layer.pixels, old_width, old_height, width, height);
        }

        if let Some(selection) = &mut self.selection {
            let scale_x = width as f32 / old_width as f32;
            let scale_y = height as f32 / old_height as f32;
            selection.min.x *= scale_x;
            selection.max.x *= scale_x;
            selection.min.y *= scale_y;
            selection.max.y *= scale_y;
        }

        self.width = width;
        self.height = height;
        self.mark_document_changed();
        notices.push(format!("Immagine ridimensionata a {}x{} px", width, height));
    }

    pub fn composite(&self) -> ColorImage {
        let total_pixels = self.width * self.height;
        let mut final_pixels = vec![Color32::TRANSPARENT; total_pixels];

        let first_layer_opaque = self.layers.first().is_some_and(|l| {
            l.visible && (l.opacity - 1.0).abs() < 0.001 && l.pixels.len() >= total_pixels
        });

        if !first_layer_opaque {
            for y in 0..self.height {
                let y_check = (y / 16) % 2 == 0;
                let row_offset = y * self.width;
                for x in 0..self.width {
                    let is_check = y_check ^ ((x / 16) % 2 == 0);
                    final_pixels[row_offset + x] = if is_check {
                        Color32::from_gray(245)
                    } else {
                        Color32::from_gray(215)
                    };
                }
            }
        }

        for (idx, layer) in self.layers.iter().enumerate() {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }

            let is_full_opacity = (layer.opacity - 1.0).abs() < 0.001;
            let count = layer.pixels.len().min(total_pixels);

            if idx == 0 && is_full_opacity && first_layer_opaque {
                final_pixels[..count].copy_from_slice(&layer.pixels[..count]);
            } else {
                for (destination, &source) in final_pixels.iter_mut().take(count).zip(&layer.pixels)
                {
                    *destination = composite_pixel(*destination, source, layer.opacity);
                }
            }
        }

        ColorImage::from_rgba_unmultiplied(
            [self.width, self.height],
            bytemuck::cast_slice(&final_pixels),
        )
    }
}

fn composite_pixel(destination: Color32, source: Color32, opacity: f32) -> Color32 {
    let [source_red, source_green, source_blue, source_alpha] = source.to_srgba_unmultiplied();
    let [destination_red, destination_green, destination_blue, destination_alpha] =
        destination.to_srgba_unmultiplied();
    let source_alpha = source_alpha as f32 / 255.0 * opacity.clamp(0.0, 1.0);
    if source_alpha <= 0.0 {
        return destination;
    }

    let destination_alpha = destination_alpha as f32 / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    if output_alpha <= 0.0 {
        return Color32::TRANSPARENT;
    }

    let blend_channel = |source: u8, destination: u8| {
        ((source as f32 * source_alpha
            + destination as f32 * destination_alpha * (1.0 - source_alpha))
            / output_alpha)
            .round() as u8
    };

    Color32::from_rgba_unmultiplied(
        blend_channel(source_red, destination_red),
        blend_channel(source_green, destination_green),
        blend_channel(source_blue, destination_blue),
        (output_alpha * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::{resize_layer_pixels, valid_canvas_dimensions};
    use crate::model::{ClipboardData, PaintApp};
    use egui::{Color32, Pos2, Rect, Vec2};

    #[test]
    fn canvas_dimensions_are_bounded_and_nonzero() {
        assert!(valid_canvas_dimensions(10_000, 10_000));
        assert!(!valid_canvas_dimensions(10_001, 10_000));
        assert!(!valid_canvas_dimensions(16_385, 1));
        assert!(!valid_canvas_dimensions(0, 100));
    }

    #[test]
    fn resizing_layer_preserves_solid_color_and_pixel_count() {
        let color = Color32::from_rgba_unmultiplied(20, 80, 160, 255);
        let pixels = vec![color; 4];

        let resized = resize_layer_pixels(&pixels, 2, 2, 5, 3);

        assert_eq!(resized.len(), 15);
        assert!(resized.iter().all(|pixel| *pixel == color));
    }

    #[test]
    fn paste_creates_a_new_layer_at_the_selection() {
        let mut app = PaintApp::new_headless();
        let original_layer_count = app.layers.len();
        let pasted_color = Color32::from_rgb(200, 30, 40);
        app.clipboard = Some(ClipboardData {
            width: 2,
            height: 1,
            pixels: vec![pasted_color, Color32::TRANSPARENT],
        });
        app.selection = Some(Rect::from_min_size(
            Pos2::new(8.0, 9.0),
            Vec2::new(2.0, 1.0),
        ));

        let mut notices = Vec::new();
        app.paste(&mut notices);

        assert_eq!(app.layers.len(), original_layer_count + 1);
        assert_eq!(app.active_layer_idx, original_layer_count);
        assert_eq!(
            app.layers[1].pixels[9 * app.width + 8],
            Color32::TRANSPARENT
        );
        assert_eq!(app.layers[2].pixels[9 * app.width + 8], pasted_color);
        assert_eq!(
            app.layers[2].pixels[9 * app.width + 9],
            Color32::TRANSPARENT
        );
        assert_eq!(app.get_selection_bounds(), Some((8, 9, 10, 10)));
    }

    #[test]
    fn copying_a_selection_then_pasting_creates_a_new_layer() {
        let mut app = PaintApp::new_headless();
        let original_layer_count = app.layers.len();
        let copied_color = Color32::from_rgb(25, 100, 200);
        app.layers[app.active_layer_idx].pixels[4 * app.width + 3] = copied_color;
        app.selection = Some(Rect::from_min_size(
            Pos2::new(3.0, 4.0),
            Vec2::new(1.0, 1.0),
        ));

        let mut notices = Vec::new();
        app.copy_selection(&mut notices);
        app.paste(&mut notices);

        assert_eq!(app.layers.len(), original_layer_count + 1);
        assert_eq!(app.active_layer_idx, original_layer_count);
        assert_eq!(
            app.layers[app.active_layer_idx].pixels[4 * app.width + 3],
            copied_color
        );
        assert_eq!(app.get_selection_bounds(), Some((3, 4, 4, 5)));
    }

    #[test]
    fn merge_layers_down_flattens_selected_and_lower_layers_only() {
        use crate::model::Layer;

        let mut app = PaintApp::new_headless();
        let pixel_count = app.width * app.height;
        let mut bottom = Layer::new("Fondo", app.width, app.height);
        bottom.pixels.fill(Color32::from_rgb(255, 0, 0));
        let mut middle = Layer::new("Centro", app.width, app.height);
        middle.pixels.fill(Color32::from_rgb(0, 255, 0));
        let mut selected = Layer::new("Selezionato", app.width, app.height);
        selected
            .pixels
            .fill(Color32::from_rgba_unmultiplied(0, 0, 255, 128));
        let mut upper = Layer::new("Superiore", app.width, app.height);
        upper.pixels.fill(Color32::from_rgb(255, 255, 0));
        app.layers = vec![bottom, middle, selected, upper];
        app.active_layer_idx = 3;
        app.layers[2].opacity = 1.0;

        let mut notices = Vec::new();
        app.merge_layers_down(2, &mut notices);

        assert_eq!(app.layers.len(), 2);
        assert_eq!(app.layers[0].name, "Selezionato (Fusi)");
        assert_eq!(app.layers[0].pixels.len(), pixel_count);
        assert_eq!(app.layers[0].pixels[0], Color32::from_rgb(0, 127, 128));
        assert_eq!(app.layers[1].name, "Superiore");
        assert_eq!(app.active_layer_idx, 1);
        assert!(app.document_revision > 0);

        app.undo();
        assert_eq!(app.layers.len(), 4);
        assert_eq!(app.layers[2].name, "Selezionato");
    }
}
