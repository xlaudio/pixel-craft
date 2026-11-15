//! Layer compositing and pixel-level transformations.

use egui::Color32;

use crate::model::{Layer, PaintApp};

use super::composite_pixel;

impl PaintApp {
    pub fn merge_layers_down(&mut self, index: usize, notices: &mut Vec<String>) {
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
        self.mark_document_changed();
        notices.push("Livelli sottostanti fusi".to_string());
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
        self.mark_document_changed();
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
        self.mark_document_changed();
    }

    pub fn invert_colors(&mut self) {
        self.push_undo_snapshot();
        if let Some(layer) = self.layers.get_mut(self.active_layer_idx) {
            for pixel in &mut layer.pixels {
                if pixel.a() > 0 {
                    *pixel = Color32::from_rgba_unmultiplied(
                        255 - pixel.r(),
                        255 - pixel.g(),
                        255 - pixel.b(),
                        pixel.a(),
                    );
                }
            }
        }
        self.mark_document_changed();
    }
}

#[cfg(test)]
mod tests {
    use crate::model::PaintApp;
    use egui::Color32;

    #[test]
    fn pixel_transforms_update_only_the_active_layer() {
        let mut app = PaintApp::new_headless();
        app.width = 2;
        app.height = 2;

        let red = Color32::from_rgb(255, 0, 0);
        let green = Color32::from_rgb(0, 255, 0);
        let blue = Color32::from_rgb(0, 0, 255);
        app.layers[0].pixels = vec![Color32::WHITE; 4];
        app.layers[1].pixels = vec![red, green, blue, Color32::TRANSPARENT];

        app.flip_horizontal();
        assert_eq!(
            app.layers[1].pixels,
            vec![green, red, Color32::TRANSPARENT, blue]
        );

        app.flip_vertical();
        assert_eq!(
            app.layers[1].pixels,
            vec![Color32::TRANSPARENT, blue, green, red]
        );

        app.invert_colors();
        assert_eq!(
            app.layers[1].pixels,
            vec![
                Color32::TRANSPARENT,
                Color32::from_rgb(255, 255, 0),
                Color32::from_rgb(255, 0, 255),
                Color32::from_rgb(0, 255, 255)
            ]
        );
        assert_eq!(app.layers[0].pixels, vec![Color32::WHITE; 4]);
    }
}
