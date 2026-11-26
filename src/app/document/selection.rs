//! Selection editing and clipboard operations for the active document.

use egui::{Color32, Pos2, Rect, Vec2};

use crate::model::{ClipboardData, Layer, MovingSelection, PaintApp, SelectionRegion};

use super::valid_canvas_dimensions;

pub(super) fn clipboard_image_to_data(
    image: &arboard::ImageData<'_>,
) -> Result<ClipboardData, String> {
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

impl PaintApp {
    pub fn has_selection(&self) -> bool {
        self.selection
            .as_ref()
            .is_some_and(SelectionRegion::is_valid)
    }

    pub fn get_selection_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let rect = self.selection.as_ref()?.bounds();
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
        let selection = Some(SelectionRegion::Rectangle(Rect::from_min_max(
            Pos2::ZERO,
            Pos2::new(self.width as f32, self.height as f32),
        )));
        if self.selection != selection {
            self.push_undo_snapshot();
            self.selection = selection;
            self.mark_document_changed();
        }
    }

    pub fn deselect(&mut self) {
        if self.selection.is_some() {
            self.push_undo_snapshot();
            self.selection = None;
            self.mark_document_changed();
        }
    }

    pub fn crop_to_selection(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.get_selection_bounds() {
            self.push_undo_snapshot();
            let width = x1 - x0;
            let height = y1 - y0;
            let selection = self
                .selection
                .clone()
                .expect("selection bounds require selection");
            for layer in &mut self.layers {
                let mut pixels = Vec::with_capacity(width * height);
                for y in y0..y1 {
                    for x in x0..x1 {
                        let pixel = layer.pixels[y * self.width + x];
                        pixels.push(if selection.contains_pixel(x, y) {
                            pixel
                        } else {
                            Color32::TRANSPARENT
                        });
                    }
                }
                layer.pixels = pixels;
            }
            self.width = width;
            self.height = height;
            self.selection = None;
            self.mark_document_changed();
        }
    }

    pub fn begin_selection_move(&mut self, position: Pos2) -> bool {
        let Some(selection) = self.selection.clone() else {
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
        let mut mask = Vec::with_capacity(width * height);
        for y in y0..y1 {
            for x in x0..x1 {
                let selected = selection.contains_pixel(x, y);
                mask.push(selected);
                pixels.push(if selected {
                    layer.pixels[y * self.width + x]
                } else {
                    Color32::TRANSPARENT
                });
            }
        }

        self.push_undo_snapshot();
        self.moving_selection = Some(MovingSelection {
            origin: selection,
            x: x0,
            y: y0,
            width,
            height,
            layer_idx: self.active_layer_idx,
            pixels,
            mask,
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
                for col in 0..moving.width {
                    let mask_index = row * moving.width + col;
                    if moving.mask[mask_index] {
                        let source_index = (moving.y + row) * self.width + moving.x + col;
                        layer.pixels[source_index] = clear_color;
                    }
                }
            }
            if let Some(data) = &mut layer.text_data {
                data.pos += Vec2::new(x as f32 - moving.x as f32, y as f32 - moving.y as f32);
            }
            for row in 0..moving.height {
                for col in 0..moving.width {
                    let mask_index = row * moving.width + col;
                    if moving.mask[mask_index] {
                        let target_index = (y + row) * self.width + x + col;
                        layer.pixels[target_index] = moving.pixels[mask_index];
                    }
                }
            }
        }
        self.selection = Some(moving.origin.translated(Vec2::new(
            x as f32 - moving.x as f32,
            y as f32 - moving.y as f32,
        )));
        self.mark_document_changed();
    }

    pub fn copy_selection(&mut self, notices: &mut Vec<String>) {
        if let Some((x0, y0, x1, y1)) = self.get_selection_bounds() {
            let width = x1 - x0;
            let height = y1 - y0;
            let mut pixels = Vec::with_capacity(width * height);
            if let Some(layer) = self.layers.get(self.active_layer_idx) {
                for y in y0..y1 {
                    for x in x0..x1 {
                        pixels.push(
                            if self
                                .selection
                                .as_ref()
                                .is_some_and(|selection| selection.contains_pixel(x, y))
                            {
                                layer.pixels[y * self.width + x]
                            } else {
                                Color32::TRANSPARENT
                            },
                        );
                    }
                }
                self.clipboard = Some(ClipboardData {
                    width,
                    height,
                    pixels,
                });
                notices.push(format!("Copiato negli appunti ({}x{} px)", width, height));
            }
        }
    }

    pub fn cut_selection(&mut self, notices: &mut Vec<String>) {
        if self.has_selection() {
            self.copy_selection(notices);
            self.delete_selection();
        }
    }

    pub fn delete_selection(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.get_selection_bounds() {
            self.push_undo_snapshot();
            let clear_color = if self.active_layer_idx == 0 && self.layers.len() > 1 {
                Color32::WHITE
            } else {
                Color32::TRANSPARENT
            };
            if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
                for y in y0..y1 {
                    for x in x0..x1 {
                        if self
                            .selection
                            .as_ref()
                            .is_some_and(|selection| selection.contains_pixel(x, y))
                        {
                            layer.pixels[y * self.width + x] = clear_color;
                        }
                    }
                }
            }
            self.selection = None;
            self.mark_document_changed();
        }
    }

    pub fn paste(&mut self, notices: &mut Vec<String>) {
        if let Some(clip) = self.clipboard.clone() {
            let (px, py) = if let Some(rect) = &self.selection {
                let rect = rect.bounds();
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
                notices.push("L'immagine incollata è fuori dalla tela".to_string());
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

            self.selection = Some(SelectionRegion::Rectangle(Rect::from_min_size(
                Pos2::new(px as f32, py as f32),
                Vec2::new(pasted_width as f32, pasted_height as f32),
            )));
            self.mark_document_changed();
            notices.push(format!(
                "Immagine incollata su un nuovo livello ({}x{} px)",
                pasted_width, pasted_height
            ));
        } else {
            notices.push("Appunti vuoti".to_string());
        }
    }

    pub fn paste_from_system_clipboard(&mut self, notices: &mut Vec<String>) {
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_image()) {
            Ok(image) => match clipboard_image_to_data(&image) {
                Ok(data) => {
                    self.clipboard = Some(data);
                    self.paste(notices);
                }
                Err(error) => {
                    eprintln!("{error}");
                    notices.push(error);
                }
            },
            Err(error) => {
                if self.clipboard.is_some() {
                    notices.push(format!(
                        "Immagine di sistema non disponibile ({error}); uso gli appunti interni"
                    ));
                    self.paste(notices);
                } else {
                    let message = format!("Nessuna immagine negli appunti di sistema: {error}");
                    eprintln!("{message}");
                    notices.push(message);
                }
            }
        }
    }

    pub fn paste_in_new_window(&mut self, notices: &mut Vec<String>) {
        if let Some(clip) = self.clipboard.clone() {
            self.new_canvas(clip.width, clip.height, notices);
            if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
                for cy in 0..clip.height {
                    for cx in 0..clip.width {
                        let source = clip.pixels[cy * clip.width + cx];
                        if source.a() > 0 {
                            layer.pixels[cy * self.width + cx] = source;
                        }
                    }
                }
            }
            self.selection = Some(SelectionRegion::Rectangle(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(clip.width as f32, clip.height as f32),
            )));
            self.mark_document_changed();
        } else {
            notices.push("Appunti vuoti".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::clipboard_image_to_data;
    use crate::model::{Layer, PaintApp, SelectionRegion};
    use egui::Color32;
    use egui::{Pos2, Rect, Vec2};
    use std::borrow::Cow;

    #[test]
    fn cropping_selection_resizes_all_layers_and_clears_selection() {
        let mut app = PaintApp::new_headless();
        app.width = 2;
        app.height = 2;
        app.layers = vec![
            Layer {
                name: "Fondo".to_string(),
                visible: true,
                opacity: 1.0,
                pixels: vec![Color32::WHITE; 4],
                text_data: None,
            },
            Layer {
                name: "Disegno".to_string(),
                visible: true,
                opacity: 1.0,
                pixels: vec![
                    Color32::BLACK,
                    Color32::from_gray(1),
                    Color32::from_gray(2),
                    Color32::from_gray(3),
                ],
                text_data: None,
            },
        ];
        app.selection = Some(SelectionRegion::Rectangle(Rect::from_min_size(
            Pos2::new(1.0, 0.0),
            Vec2::new(1.0, 2.0),
        )));

        app.crop_to_selection();

        assert_eq!((app.width, app.height), (1, 2));
        assert_eq!(app.layers[0].pixels, vec![Color32::WHITE; 2]);
        assert_eq!(
            app.layers[1].pixels,
            vec![Color32::from_gray(1), Color32::from_gray(3)]
        );
        assert!(app.selection.is_none());
    }

    #[test]
    fn ellipse_copy_and_delete_only_affect_pixels_inside_the_region() {
        let mut app = PaintApp::new_headless();
        app.width = 5;
        app.height = 5;
        app.layers[1].pixels = (0..25)
            .map(|index| Color32::from_rgb(index, 0, 0))
            .collect();
        app.selection = Some(SelectionRegion::Ellipse(Rect::from_min_max(
            Pos2::ZERO,
            Pos2::new(5.0, 5.0),
        )));
        let corner_pixel = app.layers[1].pixels[0];
        let center_pixel = app.layers[1].pixels[2 * 5 + 2];
        let mut notices = Vec::new();

        app.copy_selection(&mut notices);

        let clipboard = app.clipboard.as_ref().expect("ellipse copied");
        assert_eq!(clipboard.pixels[0], Color32::TRANSPARENT);
        assert_eq!(clipboard.pixels[2 * 5 + 2], center_pixel);

        app.delete_selection();

        assert_eq!(app.layers[1].pixels[0], corner_pixel);
        assert_eq!(app.layers[1].pixels[2 * 5 + 2], Color32::TRANSPARENT);
    }

    #[test]
    fn moving_ellipse_preserves_unselected_pixels_and_moves_its_mask() {
        let mut app = PaintApp::new_headless();
        app.width = 7;
        app.height = 7;
        app.layers[1].pixels = (0..49)
            .map(|index| Color32::from_rgb(index, 0, 0))
            .collect();
        app.selection = Some(SelectionRegion::Ellipse(Rect::from_min_max(
            Pos2::new(1.0, 1.0),
            Pos2::new(5.0, 5.0),
        )));
        let unselected_corner = app.layers[1].pixels[1 * 7 + 1];
        let selected_pixel = app.layers[1].pixels[3 * 7 + 1];

        assert!(app.begin_selection_move(Pos2::new(3.0, 3.0)));
        app.finish_selection_move(Vec2::new(1.0, 0.0));

        assert_eq!(app.layers[1].pixels[1 * 7 + 1], unselected_corner);
        assert_eq!(app.layers[1].pixels[3 * 7 + 1], Color32::TRANSPARENT);
        assert_eq!(app.layers[1].pixels[3 * 7 + 2], selected_pixel);
        assert!(matches!(app.selection, Some(SelectionRegion::Ellipse(_))));
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
