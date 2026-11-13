//! # document — Tela, livelli e modifiche al documento
//!
//! Contiene le operazioni sul documento pittorico: creazione della tela,
//! selezioni e appunti in memoria, trasformazioni dei livelli e compositing
//! dei livelli visibili nell'immagine finale.

use eframe::egui;
use egui::{Color32, ColorImage, Pos2, Rect, Vec2};

use crate::model::{ClipboardData, Layer, MovingSelection, PaintApp};

const MAX_CANVAS_DIMENSION: usize = 16_384;
const MAX_CANVAS_PIXELS: usize = 100_000_000;

fn valid_canvas_dimensions(width: usize, height: usize) -> bool {
    width > 0
        && height > 0
        && width <= MAX_CANVAS_DIMENSION
        && height <= MAX_CANVAS_DIMENSION
        && width
            .checked_mul(height)
            .is_some_and(|pixels| pixels <= MAX_CANVAS_PIXELS)
}

fn clipboard_image_to_data(image: &arboard::ImageData<'_>) -> Result<ClipboardData, String> {
    if !valid_canvas_dimensions(image.width, image.height) {
        return Err("Dimensioni immagine negli appunti non valide".to_string());
    }

    let expected_bytes = image
        .width
        .checked_mul(image.height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Dimensioni immagine negli appunti non valide".to_string())?;
    if image.bytes.len() != expected_bytes {
        return Err("Dati immagine negli appunti non validi".to_string());
    }

    let pixels = image
        .bytes
        .chunks_exact(4)
        .map(|rgba| Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]))
        .collect();
    Ok(ClipboardData {
        width: image.width,
        height: image.height,
        pixels,
    })
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
    pub fn new_canvas(&mut self, width: usize, height: usize) {
        if !valid_canvas_dimensions(width, height) {
            self.show_notice("Dimensioni immagine non valide (massimo 100 megapixel)");
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
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.selection = None;
        self.current_filepath = None;
        self.canvas_texture = None;
        self.dirty_canvas = true;
    }

    pub fn resize_canvas(&mut self, width: usize, height: usize) {
        if !valid_canvas_dimensions(width, height) {
            self.show_notice("Dimensioni immagine non valide (massimo 100 megapixel)");
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
        self.canvas_texture = None;
        self.dirty_canvas = true;
        self.show_notice(&format!(
            "Immagine ridimensionata a {}x{} px",
            width, height
        ));
    }

    pub fn has_selection(&self) -> bool {
        if let Some(rect) = self.selection {
            rect.width().abs() >= 1.0 && rect.height().abs() >= 1.0
        } else {
            false
        }
    }

    pub fn get_selection_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let rect = self.selection?;
        let min_x =
            (rect.min.x.min(rect.max.x).floor() as i32).clamp(0, self.width as i32) as usize;
        let max_x = (rect.min.x.max(rect.max.x).ceil() as i32).clamp(0, self.width as i32) as usize;
        let min_y =
            (rect.min.y.min(rect.max.y).floor() as i32).clamp(0, self.height as i32) as usize;
        let max_y =
            (rect.min.y.max(rect.max.y).ceil() as i32).clamp(0, self.height as i32) as usize;
        if max_x > min_x && max_y > min_y {
            Some((min_x, min_y, max_x, max_y))
        } else {
            None
        }
    }

    pub fn select_all(&mut self) {
        self.selection = Some(Rect::from_min_max(
            Pos2::ZERO,
            Pos2::new(self.width as f32, self.height as f32),
        ));
    }

    pub fn deselect(&mut self) {
        self.selection = None;
    }

    pub fn begin_selection_move(&mut self, position: Pos2) -> bool {
        let Some(selection) = self.selection else {
            return false;
        };
        if !selection.contains(position) {
            return false;
        }
        let Some((x0, y0, x1, y1)) = self.get_selection_bounds() else {
            return false;
        };
        let Some(layer) = self.layers.get(self.active_layer_idx) else {
            return false;
        };
        let width = x1 - x0;
        let height = y1 - y0;
        let mut pixels = Vec::with_capacity(width * height);
        for y in y0..y1 {
            let start = y * self.width + x0;
            pixels.extend_from_slice(&layer.pixels[start..start + width]);
        }

        self.push_undo_snapshot();
        self.moving_selection = Some(MovingSelection {
            origin: Rect::from_min_max(
                Pos2::new(x0 as f32, y0 as f32),
                Pos2::new(x1 as f32, y1 as f32),
            ),
            x: x0,
            y: y0,
            width,
            height,
            layer_idx: self.active_layer_idx,
            pixels,
        });
        true
    }

    pub fn finish_selection_move(&mut self, delta: Vec2) {
        let Some(moving) = self.moving_selection.take() else {
            return;
        };
        let max_x = self.width.saturating_sub(moving.width);
        let max_y = self.height.saturating_sub(moving.height);
        let x = (moving.x as f32 + delta.x).round().clamp(0.0, max_x as f32) as usize;
        let y = (moving.y as f32 + delta.y).round().clamp(0.0, max_y as f32) as usize;

        let clear_color = if moving.layer_idx == 0 && self.layers.len() > 1 {
            Color32::WHITE
        } else {
            Color32::TRANSPARENT
        };
        if let Some(layer) = self.layers.get_mut(moving.layer_idx) {
            for row in 0..moving.height {
                let start = (moving.y + row) * self.width + moving.x;
                layer.pixels[start..start + moving.width].fill(clear_color);
            }
            for row in 0..moving.height {
                let source_start = row * moving.width;
                let target_start = (y + row) * self.width + x;
                for col in 0..moving.width {
                    let pixel = moving.pixels[source_start + col];
                    if pixel.a() > 0 {
                        layer.pixels[target_start + col] = pixel;
                    }
                }
            }
        }
        self.selection = Some(moving.origin.translate(Vec2::new(
            x as f32 - moving.x as f32,
            y as f32 - moving.y as f32,
        )));
        self.dirty_canvas = true;
    }

    pub fn copy_selection(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.get_selection_bounds() {
            let w = x1 - x0;
            let h = y1 - y0;
            let mut pixels = Vec::with_capacity(w * h);
            if let Some(layer) = self.layers.get(self.active_layer_idx) {
                for y in y0..y1 {
                    for x in x0..x1 {
                        pixels.push(layer.pixels[y * self.width + x]);
                    }
                }
                self.clipboard = Some(ClipboardData {
                    width: w,
                    height: h,
                    pixels,
                });
                self.show_notice(&format!("Copiato negli appunti ({}x{} px)", w, h));
            }
        }
    }

    pub fn cut_selection(&mut self) {
        if self.has_selection() {
            self.copy_selection();
            self.delete_selection();
        }
    }

    pub fn merge_layers_down(&mut self, index: usize) {
        if index == 0 || index >= self.layers.len() {
            return;
        }

        self.push_undo_snapshot();
        let mut pixels = vec![Color32::TRANSPARENT; self.width * self.height];
        for layer in &self.layers[..=index] {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            for (destination, &source) in pixels.iter_mut().zip(&layer.pixels) {
                *destination = composite_pixel(*destination, source, layer.opacity);
            }
        }

        let name = format!("{} (Fusi)", self.layers[index].name);
        self.layers.drain(..=index);
        self.layers.insert(
            0,
            Layer {
                name,
                visible: true,
                opacity: 1.0,
                pixels,
            },
        );
        self.active_layer_idx = if self.active_layer_idx > index {
            self.active_layer_idx - index
        } else {
            0
        };
        self.dirty_canvas = true;
        self.show_notice("Livelli sottostanti fusi");
    }

    pub fn delete_selection(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.get_selection_bounds() {
            self.push_undo_snapshot();
            let clear_col = if self.active_layer_idx == 0 && self.layers.len() > 1 {
                Color32::WHITE
            } else {
                Color32::TRANSPARENT
            };
            if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
                for y in y0..y1 {
                    for x in x0..x1 {
                        layer.pixels[y * self.width + x] = clear_col;
                    }
                }
            }
            self.selection = None;
            self.dirty_canvas = true;
        }
    }

    pub fn paste(&mut self) {
        if let Some(clip) = self.clipboard.clone() {
            let (px, py) = if let Some(rect) = self.selection {
                (
                    (rect.min.x.min(rect.max.x).floor() as i32).clamp(0, self.width as i32)
                        as usize,
                    (rect.min.y.min(rect.max.y).floor() as i32).clamp(0, self.height as i32)
                        as usize,
                )
            } else {
                (0, 0)
            };

            let pasted_width = clip.width.min(self.width.saturating_sub(px));
            let pasted_height = clip.height.min(self.height.saturating_sub(py));
            if pasted_width == 0 || pasted_height == 0 {
                self.show_notice("L'immagine incollata è fuori dalla tela");
                return;
            }

            self.push_undo_snapshot();
            let mut layer = Layer::new(
                &format!("Livello {}", self.layers.len()),
                self.width,
                self.height,
            );
            for cy in 0..pasted_height {
                for cx in 0..pasted_width {
                    layer.pixels[(py + cy) * self.width + px + cx] =
                        clip.pixels[cy * clip.width + cx];
                }
            }
            self.layers.push(layer);
            self.active_layer_idx = self.layers.len() - 1;

            self.selection = Some(Rect::from_min_size(
                Pos2::new(px as f32, py as f32),
                Vec2::new(pasted_width as f32, pasted_height as f32),
            ));
            self.dirty_canvas = true;
            self.show_notice(&format!(
                "Immagine incollata su un nuovo livello ({}x{} px)",
                pasted_width, pasted_height
            ));
        } else {
            self.show_notice("Appunti vuoti");
        }
    }

    pub fn paste_from_system_clipboard(&mut self) {
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_image()) {
            Ok(image) => match clipboard_image_to_data(&image) {
                Ok(data) => {
                    self.clipboard = Some(data);
                    self.paste();
                }
                Err(error) => {
                    eprintln!("{error}");
                    self.show_notice(&error);
                }
            },
            Err(error) => {
                if self.clipboard.is_some() {
                    self.show_notice(&format!(
                        "Immagine di sistema non disponibile ({error}); uso gli appunti interni"
                    ));
                    self.paste();
                } else {
                    let message = format!("Nessuna immagine negli appunti di sistema: {error}");
                    eprintln!("{message}");
                    self.show_notice(&message);
                }
            }
        }
    }

    pub fn paste_in_new_window(&mut self) {
        if let Some(clip) = self.clipboard.clone() {
            self.new_canvas(clip.width, clip.height);
            if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
                for cy in 0..clip.height {
                    for cx in 0..clip.width {
                        let src = clip.pixels[cy * clip.width + cx];
                        if src.a() > 0 {
                            layer.pixels[cy * self.width + cx] = src;
                        }
                    }
                }
            }
            self.selection = Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(clip.width as f32, clip.height as f32),
            ));
            self.canvas_texture = None;
            self.dirty_canvas = true;
        } else {
            self.show_notice("Appunti vuoti");
        }
    }

    pub fn crop_to_selection(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.get_selection_bounds() {
            self.push_undo_snapshot();
            let new_w = x1 - x0;
            let new_h = y1 - y0;
            for layer in &mut self.layers {
                let mut new_pixels = Vec::with_capacity(new_w * new_h);
                for y in y0..y1 {
                    for x in x0..x1 {
                        new_pixels.push(layer.pixels[y * self.width + x]);
                    }
                }
                layer.pixels = new_pixels;
            }
            self.width = new_w;
            self.height = new_h;
            self.selection = None;
            self.canvas_texture = None;
            self.dirty_canvas = true;
        }
    }

    pub fn flip_horizontal(&mut self) {
        self.push_undo_snapshot();
        if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
            for y in 0..self.height {
                let row_start = y * self.width;
                let row_end = row_start + self.width;
                layer.pixels[row_start..row_end].reverse();
            }
        }
        self.dirty_canvas = true;
    }

    pub fn flip_vertical(&mut self) {
        self.push_undo_snapshot();
        if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
            for y in 0..(self.height / 2) {
                let top_y = y;
                let bot_y = self.height - 1 - y;
                for x in 0..self.width {
                    let idx_top = top_y * self.width + x;
                    let idx_bot = bot_y * self.width + x;
                    layer.pixels.swap(idx_top, idx_bot);
                }
            }
        }
        self.dirty_canvas = true;
    }

    pub fn invert_colors(&mut self) {
        self.push_undo_snapshot();
        if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
            for p in &mut layer.pixels {
                if p.a() > 0 {
                    *p = Color32::from_rgba_unmultiplied(
                        255 - p.r(),
                        255 - p.g(),
                        255 - p.b(),
                        p.a(),
                    );
                }
            }
        }
        self.dirty_canvas = true;
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
    use super::{clipboard_image_to_data, resize_layer_pixels, valid_canvas_dimensions};
    use crate::model::{ClipboardData, PaintApp};
    use egui::{Color32, Pos2, Rect, Vec2};
    use std::borrow::Cow;

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

        app.paste();

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

        app.copy_selection();
        app.paste();

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

        app.merge_layers_down(2);

        assert_eq!(app.layers.len(), 2);
        assert_eq!(app.layers[0].name, "Selezionato (Fusi)");
        assert_eq!(app.layers[0].pixels.len(), pixel_count);
        assert_eq!(app.layers[0].pixels[0], Color32::from_rgb(0, 127, 128));
        assert_eq!(app.layers[1].name, "Superiore");
        assert_eq!(app.active_layer_idx, 1);
        assert!(app.dirty_canvas);

        app.undo();
        assert_eq!(app.layers.len(), 4);
        assert_eq!(app.layers[2].name, "Selezionato");
    }

    #[test]
    fn system_clipboard_image_keeps_rgba_channels() {
        let image = arboard::ImageData {
            width: 1,
            height: 1,
            bytes: Cow::Borrowed(&[10, 20, 30, 40]),
        };

        let data = clipboard_image_to_data(&image).expect("image data should be valid");

        assert_eq!(
            data.pixels,
            vec![Color32::from_rgba_unmultiplied(10, 20, 30, 40)]
        );
    }

    #[test]
    fn system_clipboard_image_rejects_mismatched_pixel_data() {
        let image = arboard::ImageData {
            width: 1,
            height: 1,
            bytes: Cow::Borrowed(&[10, 20, 30]),
        };

        assert!(clipboard_image_to_data(&image).is_err());
    }
}
