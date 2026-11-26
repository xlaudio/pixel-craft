//! Layer compositing and pixel-level transformations.

use egui::Color32;

use crate::model::{Layer, PaintApp};

use super::{composite_pixel, valid_canvas_dimensions};

fn rotate_pixels(pixels: &[Color32], width: usize, height: usize, clockwise: bool) -> Vec<Color32> {
    let mut rotated = vec![Color32::TRANSPARENT; width * height];
    let new_width = height;

    for y in 0..height {
        for x in 0..width {
            let (new_x, new_y) = if clockwise {
                (height - 1 - y, x)
            } else {
                (y, width - 1 - x)
            };
            rotated[new_y * new_width + new_x] = pixels[y * width + x];
        }
    }

    rotated
}

fn effective_rotation(angle_degrees: u16, clockwise: bool) -> Option<u16> {
    if angle_degrees > 360 {
        return None;
    }
    let angle = angle_degrees % 360;
    Some(if clockwise {
        angle
    } else {
        (360 - angle) % 360
    })
}

fn rotated_dimensions(width: usize, height: usize, angle_degrees: u16) -> Option<(usize, usize)> {
    let radians = f64::from(angle_degrees).to_radians();
    let (sin, cos) = radians.sin_cos();
    let (rotated_width, rotated_height) = match angle_degrees {
        0 | 180 => (width, height),
        90 | 270 => (height, width),
        _ => (
            (width as f64 * cos.abs() + height as f64 * sin.abs()).round() as usize,
            (width as f64 * sin.abs() + height as f64 * cos.abs()).round() as usize,
        ),
    };
    valid_canvas_dimensions(rotated_width, rotated_height)
        .then_some((rotated_width, rotated_height))
}

fn rotate_pixels_arbitrary(
    pixels: &[Color32],
    width: usize,
    height: usize,
    rotated_width: usize,
    rotated_height: usize,
    angle_degrees: u16,
) -> Vec<Color32> {
    let radians = f64::from(angle_degrees).to_radians();
    let (sin, cos) = radians.sin_cos();
    let source_center_x = width as f64 / 2.0;
    let source_center_y = height as f64 / 2.0;
    let destination_center_x = rotated_width as f64 / 2.0;
    let destination_center_y = rotated_height as f64 / 2.0;
    let mut rotated = vec![Color32::TRANSPARENT; rotated_width * rotated_height];

    for y in 0..rotated_height {
        for x in 0..rotated_width {
            let destination_x = x as f64 + 0.5 - destination_center_x;
            let destination_y = y as f64 + 0.5 - destination_center_y;
            let source_x = cos * destination_x + sin * destination_y + source_center_x - 0.5;
            let source_y = -sin * destination_x + cos * destination_y + source_center_y - 0.5;
            let source_x = source_x.round() as isize;
            let source_y = source_y.round() as isize;

            if source_x >= 0
                && source_y >= 0
                && source_x < width as isize
                && source_y < height as isize
            {
                rotated[y * rotated_width + x] =
                    pixels[source_y as usize * width + source_x as usize];
            }
        }
    }

    rotated
}

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
                text_data: None,
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

    pub fn rotate_clockwise(&mut self) {
        self.rotate(true);
    }

    pub fn rotate_counter_clockwise(&mut self) {
        self.rotate(false);
    }

    pub fn dimensions_after_rotation(
        &self,
        angle_degrees: u16,
        clockwise: bool,
    ) -> Option<(usize, usize)> {
        let angle = effective_rotation(angle_degrees, clockwise)?;
        rotated_dimensions(self.width, self.height, angle)
    }

    pub fn rotate_image(&mut self, angle_degrees: u16, clockwise: bool, notices: &mut Vec<String>) {
        let Some(angle) = effective_rotation(angle_degrees, clockwise) else {
            notices
                .push("L'angolo di rotazione deve essere compreso tra 0 e 360 gradi.".to_string());
            return;
        };
        if angle == 0 {
            return;
        }
        let Some((rotated_width, rotated_height)) =
            rotated_dimensions(self.width, self.height, angle)
        else {
            notices.push(
                "La rotazione supererebbe le dimensioni massime consentite per l'immagine."
                    .to_string(),
            );
            return;
        };

        match angle {
            90 => self.rotate(true),
            180 => {
                self.push_undo_snapshot();
                for layer in &mut self.layers {
                    layer.pixels.reverse();
                }
                self.selection = self.selection.as_ref().map(|selection| {
                    selection.rotated(
                        self.width,
                        self.height,
                        rotated_width,
                        rotated_height,
                        angle,
                    )
                });
                self.moving_selection = None;
                self.mark_document_changed();
            }
            270 => self.rotate(false),
            _ => {
                self.push_undo_snapshot();
                let old_width = self.width;
                let old_height = self.height;
                for layer in &mut self.layers {
                    layer.pixels = rotate_pixels_arbitrary(
                        &layer.pixels,
                        old_width,
                        old_height,
                        rotated_width,
                        rotated_height,
                        angle,
                    );
                }
                self.selection = self.selection.as_ref().map(|selection| {
                    selection.rotated(old_width, old_height, rotated_width, rotated_height, angle)
                });
                self.moving_selection = None;
                self.width = rotated_width;
                self.height = rotated_height;
                self.mark_document_changed();
            }
        }
    }

    fn rotate(&mut self, clockwise: bool) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        self.push_undo_snapshot();
        let old_width = self.width;
        let old_height = self.height;

        for layer in &mut self.layers {
            layer.pixels = rotate_pixels(&layer.pixels, old_width, old_height, clockwise);
        }

        self.selection = self.selection.as_ref().map(|selection| {
            selection.rotated(
                old_width,
                old_height,
                old_height,
                old_width,
                if clockwise { 90 } else { 270 },
            )
        });
        self.moving_selection = None;
        self.width = old_height;
        self.height = old_width;
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
    use crate::{
        app::{Command, CommandEngine},
        model::PaintApp,
    };
    use egui::{Color32, Pos2, Rect};

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

    #[test]
    fn clockwise_rotation_transforms_all_layers_and_is_undoable() {
        let mut app = PaintApp::new_headless();
        app.width = 2;
        app.height = 3;
        app.layers[0].pixels = vec![
            Color32::from_rgb(1, 0, 0),
            Color32::from_rgb(2, 0, 0),
            Color32::from_rgb(3, 0, 0),
            Color32::from_rgb(4, 0, 0),
            Color32::from_rgb(5, 0, 0),
            Color32::from_rgb(6, 0, 0),
        ];
        app.layers[1].pixels = app.layers[0].pixels.clone();
        app.selection = Some(crate::model::SelectionRegion::Rectangle(
            Rect::from_min_max(Pos2::new(0.0, 1.0), Pos2::new(2.0, 3.0)),
        ));
        let original_pixels = app.layers[0].pixels.clone();
        let mut engine = CommandEngine;

        engine.execute(&mut app, Command::RotateClockwise);

        assert_eq!((app.width, app.height), (3, 2));
        let expected = vec![
            Color32::from_rgb(5, 0, 0),
            Color32::from_rgb(3, 0, 0),
            Color32::from_rgb(1, 0, 0),
            Color32::from_rgb(6, 0, 0),
            Color32::from_rgb(4, 0, 0),
            Color32::from_rgb(2, 0, 0),
        ];
        assert_eq!(app.layers[0].pixels, expected);
        assert_eq!(app.layers[1].pixels, expected);
        assert_eq!(
            app.selection,
            Some(crate::model::SelectionRegion::Rectangle(
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(2.0, 2.0),)
            ))
        );

        engine.execute(&mut app, Command::Undo);
        assert_eq!((app.width, app.height), (2, 3));
        assert_eq!(app.layers[0].pixels, original_pixels);
    }

    #[test]
    fn counter_clockwise_rotation_maps_pixels_and_selection() {
        let mut app = PaintApp::new_headless();
        app.width = 2;
        app.height = 3;
        app.layers[1].pixels = (1..=6)
            .map(|value| Color32::from_rgb(value, 0, 0))
            .collect();
        app.selection = Some(crate::model::SelectionRegion::Rectangle(
            Rect::from_min_max(Pos2::new(0.0, 1.0), Pos2::new(2.0, 3.0)),
        ));

        app.rotate_counter_clockwise();

        assert_eq!((app.width, app.height), (3, 2));
        assert_eq!(
            app.layers[1].pixels,
            vec![
                Color32::from_rgb(2, 0, 0),
                Color32::from_rgb(4, 0, 0),
                Color32::from_rgb(6, 0, 0),
                Color32::from_rgb(1, 0, 0),
                Color32::from_rgb(3, 0, 0),
                Color32::from_rgb(5, 0, 0),
            ]
        );
        assert_eq!(
            app.selection,
            Some(crate::model::SelectionRegion::Rectangle(
                Rect::from_min_max(Pos2::new(1.0, 0.0), Pos2::new(3.0, 2.0),)
            ))
        );
    }

    #[test]
    fn arbitrary_rotation_preserves_ellipse_selection_as_a_polygon_and_is_undoable() {
        let mut app = PaintApp::new_headless();
        app.width = 5;
        app.height = 5;
        app.layers[0].pixels = vec![Color32::WHITE; 25];
        app.layers[1].pixels = vec![Color32::TRANSPARENT; 25];
        app.selection = Some(crate::model::SelectionRegion::Ellipse(Rect::from_min_max(
            Pos2::new(1.0, 1.0),
            Pos2::new(4.0, 4.0),
        )));
        let mut notices = Vec::new();

        app.rotate_image(45, true, &mut notices);

        assert!(notices.is_empty());
        assert!(matches!(
            app.selection,
            Some(crate::model::SelectionRegion::Polygon(_))
        ));

        app.undo();

        assert!(matches!(
            app.selection,
            Some(crate::model::SelectionRegion::Ellipse(_))
        ));
    }

    #[test]
    fn custom_rotation_expands_canvas_and_rotates_all_layers() {
        let mut app = PaintApp::new_headless();
        app.width = 143;
        app.height = 118;
        app.layers[0].pixels = vec![Color32::WHITE; app.width * app.height];
        app.layers[1].pixels = vec![Color32::from_rgb(10, 20, 30); app.width * app.height];
        let mut notices = Vec::new();

        assert_eq!(app.dimensions_after_rotation(2, true), Some((147, 123)));
        app.rotate_image(2, true, &mut notices);

        assert!(notices.is_empty());
        assert_eq!((app.width, app.height), (147, 123));
        assert_eq!(app.layers[0].pixels.len(), 147 * 123);
        assert_eq!(app.layers[1].pixels.len(), 147 * 123);
        assert!(app.layers[1]
            .pixels
            .iter()
            .any(|pixel| *pixel == Color32::TRANSPARENT));
        assert!(app.layers[1].pixels.iter().any(|pixel| pixel.a() > 0));
    }

    #[test]
    fn invalid_or_zero_rotation_does_not_change_the_document() {
        let mut app = PaintApp::new_headless();
        app.width = 2;
        app.height = 1;
        app.layers[1].pixels = vec![Color32::RED, Color32::BLUE];
        let revision = app.document_revision;
        let mut notices = Vec::new();

        app.rotate_image(0, true, &mut notices);
        assert_eq!(app.document_revision, revision);
        assert!(notices.is_empty());

        app.rotate_image(361, true, &mut notices);
        assert_eq!(app.document_revision, revision);
        assert_eq!(notices.len(), 1);
        assert_eq!((app.width, app.height), (2, 1));
        assert_eq!(app.layers[1].pixels, vec![Color32::RED, Color32::BLUE]);
    }
}
