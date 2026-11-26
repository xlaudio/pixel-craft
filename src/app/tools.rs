//! # tools — Algoritmi degli strumenti di disegno
//!
//! Implementa il disegno di punti e tratti, il riempimento a secchiello,
//! il contagocce e la rasterizzazione di linee, rettangoli ed ellissi.
//! Gli algoritmi modificano i pixel del livello attivo e marcano la tela
//! come da aggiornare.

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use egui::{Color32, Pos2};
use std::collections::VecDeque;

use crate::model::{PaintApp, SelectionRegion, SelectionTool, TextLayerData, Tool};

impl PaintApp {
    pub fn begin_text_edit(&mut self, pos: Pos2, font_size: f32, font_family: &str) -> usize {
        self.push_undo_snapshot();
        self.layers.push(crate::model::Layer {
            name: "Text".to_string(),
            visible: true,
            opacity: 1.0,
            pixels: vec![Color32::TRANSPARENT; self.width * self.height],
            text_data: Some(TextLayerData {
                content: String::new(),
                pos,
                font_size,
                font_family: font_family.to_string(),
                color: self.brush_color,
            }),
        });
        self.active_layer_idx = self.layers.len() - 1;
        self.active_layer_idx
    }

    pub fn update_text_edit(
        &mut self,
        content: &str,
        font_size: f32,
        font_family: &str,
        color: Color32,
    ) {
        let index = self.active_layer_idx;
        let Some(layer) = self.layers.get_mut(index) else {
            return;
        };
        if let Some(data) = &mut layer.text_data {
            data.content = content.to_string();
            data.font_size = font_size;
            data.font_family = font_family.to_string();
            data.color = color;
        }
        layer.pixels.fill(Color32::TRANSPARENT);
        self.mark_document_changed();
    }

    pub fn rasterize_text_layer(&mut self, index: usize) {
        let Some(layer) = self.layers.get(index) else {
            return;
        };
        let Some(data) = layer.text_data.clone() else {
            return;
        };
        let width = self.width;
        let height = self.height;
        let mut pixels = vec![Color32::TRANSPARENT; width * height];
        if !data.content.is_empty() {
            rasterize_text(&data, width, height, &mut pixels);
        }
        self.layers[index].pixels = pixels;
        self.mark_document_changed();
    }

    pub fn commit_text_edit(&mut self) -> bool {
        let index = self.active_layer_idx;
        let Some(layer) = self.layers.get_mut(index) else {
            return false;
        };
        let Some(data) = layer.text_data.clone() else {
            return false;
        };
        if data.content.trim().is_empty() {
            self.layers.remove(index);
            self.active_layer_idx = self
                .active_layer_idx
                .saturating_sub(1)
                .min(self.layers.len().saturating_sub(1));
        } else {
            let name = data.content.trim().chars().take(15).collect::<String>();
            self.layers[index].name = name;
            self.rasterize_text_layer(index);
            // A text layer must contain the committed pixels before its
            // editable metadata is discarded. Keep the metadata only while
            // editing; the canvas and all exporters consume `pixels`.
            self.layers[index].text_data = None;
        }
        self.mark_document_changed();
        true
    }

    pub fn begin_selection_gesture(&mut self, tool: SelectionTool, point: Pos2) {
        self.selection_tools.begin(tool, point);
    }

    pub fn update_selection_gesture(&mut self, point: Pos2) {
        self.selection_tools.update(point);
    }

    pub fn finish_selection_gesture(&mut self, point: Pos2) {
        let selection = self.selection_tools.finish(point);
        if selection.is_some() || self.selection.is_some() {
            self.push_undo_snapshot();
            self.selection = selection;
            self.mark_document_changed();
        }
    }

    pub fn selection_gesture_preview(&self, point: Pos2) -> Option<SelectionRegion> {
        self.selection_tools.preview(point)
    }

    pub fn draw_point_on_active(&mut self, cx: f32, cy: f32) {
        if self.layers.is_empty() {
            return;
        }

        let radius = (self.brush_size / 2.0).max(0.5);
        let r_i32 = radius.ceil() as i32;
        let center_x = cx as i32;
        let center_y = cy as i32;
        let r_sq = radius * radius;

        let target_color = match self.current_tool {
            Tool::Brush => self.brush_color,
            Tool::Eraser => Color32::TRANSPARENT,
            _ => self.brush_color,
        };

        let layer = &mut self.layers[self.active_layer_idx];
        let width = self.width as i32;
        let height = self.height as i32;

        for dy in -r_i32..=r_i32 {
            for dx in -r_i32..=r_i32 {
                let dist_sq = (dx as f32) * (dx as f32) + (dy as f32) * (dy as f32);
                if dist_sq <= r_sq {
                    let px = center_x + dx;
                    let py = center_y + dy;
                    if px >= 0 && px < width && py >= 0 && py < height {
                        let idx = (py as usize) * self.width + (px as usize);
                        layer.pixels[idx] = target_color;
                    }
                }
            }
        }
        self.mark_document_changed();
    }

    pub fn spray_point_on_active(&mut self, cx: f32, cy: f32) {
        if self.layers.is_empty() || self.width == 0 || self.height == 0 {
            return;
        }

        let spread = self.brush_size.max(4.0);
        let droplet_count = (self.brush_size as usize * 2).clamp(16, 120);
        let mut seed = self.spray_seed;
        let mut next_random = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 32) as u32
        };
        let layer = &mut self.layers[self.active_layer_idx];

        for _ in 0..droplet_count {
            let radial_distance = ((next_random() as f32 / u32::MAX as f32).sqrt()) * spread;
            let angle = (next_random() as f32 / u32::MAX as f32) * std::f32::consts::TAU;
            let x = (cx + angle.cos() * radial_distance).round() as i32;
            let y = (cy + angle.sin() * radial_distance).round() as i32;
            if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
                layer.pixels[y as usize * self.width + x as usize] = self.brush_color;
            }
        }

        self.spray_seed = seed;
        self.mark_document_changed();
    }

    pub fn draw_continuous_line(&mut self, p0: Pos2, p1: Pos2) {
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = (self.brush_size * 0.25).clamp(0.5, 4.0);
        let steps = (dist / step).ceil() as usize;

        if steps == 0 {
            self.draw_point_on_active(p1.x, p1.y);
            return;
        }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = p0.x + dx * t;
            let y = p0.y + dy * t;
            self.draw_point_on_active(x, y);
        }
    }

    pub fn flood_fill(&mut self, start_x: i32, start_y: i32, fill_color: Color32) {
        if start_x < 0
            || start_x >= self.width as i32
            || start_y < 0
            || start_y >= self.height as i32
        {
            return;
        }
        if self.layers.is_empty() {
            return;
        }

        let width = self.width;
        let height = self.height;
        let layer = &mut self.layers[self.active_layer_idx];
        let start_idx = (start_y as usize) * width + (start_x as usize);
        let target_color = layer.pixels[start_idx];
        if target_color == fill_color {
            return;
        }

        let mut queue = VecDeque::new();
        let mut visited = vec![false; width * height];
        queue.push_back((start_x, start_y));
        visited[start_idx] = true;

        while let Some((cx, cy)) = queue.pop_front() {
            let idx = (cy as usize) * width + (cx as usize);
            layer.pixels[idx] = fill_color;

            let neighbors = [(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)];

            for (nx, ny) in neighbors {
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let n_idx = (ny as usize) * width + (nx as usize);
                    if !visited[n_idx] && layer.pixels[n_idx] == target_color {
                        visited[n_idx] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
        self.mark_document_changed();
    }

    pub fn pick_color(&mut self, x: i32, y: i32) {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return;
        }
        let comp = self.composite();
        let idx = (y as usize * self.width + x as usize) * 4;
        let raw = comp.as_raw();
        if idx + 3 < raw.len() {
            self.brush_color = Color32::from_rgb(raw[idx], raw[idx + 1], raw[idx + 2]);
        }
    }

    pub fn bake_shape(&mut self, p0: Pos2, p1: Pos2) {
        if self.layers.is_empty() {
            return;
        }

        match self.current_tool {
            Tool::Line => {
                self.draw_continuous_line(p0, p1);
            }
            Tool::Rectangle => {
                let min_x = p0.x.min(p1.x);
                let max_x = p0.x.max(p1.x);
                let min_y = p0.y.min(p1.y);
                let max_y = p0.y.max(p1.y);

                if self.fill_shapes {
                    let x_start = (min_x.floor() as i32).clamp(0, self.width as i32 - 1);
                    let x_end = (max_x.ceil() as i32).clamp(0, self.width as i32 - 1);
                    let y_start = (min_y.floor() as i32).clamp(0, self.height as i32 - 1);
                    let y_end = (max_y.ceil() as i32).clamp(0, self.height as i32 - 1);

                    {
                        let layer = &mut self.layers[self.active_layer_idx];
                        for py in y_start..=y_end {
                            for px in x_start..=x_end {
                                let idx = (py as usize) * self.width + (px as usize);
                                layer.pixels[idx] = self.background_color;
                            }
                        }
                    }
                    self.draw_continuous_line(Pos2::new(min_x, min_y), Pos2::new(max_x, min_y));
                    self.draw_continuous_line(Pos2::new(max_x, min_y), Pos2::new(max_x, max_y));
                    self.draw_continuous_line(Pos2::new(max_x, max_y), Pos2::new(min_x, max_y));
                    self.draw_continuous_line(Pos2::new(min_x, max_y), Pos2::new(min_x, min_y));
                } else {
                    self.draw_continuous_line(Pos2::new(min_x, min_y), Pos2::new(max_x, min_y));
                    self.draw_continuous_line(Pos2::new(max_x, min_y), Pos2::new(max_x, max_y));
                    self.draw_continuous_line(Pos2::new(max_x, max_y), Pos2::new(min_x, max_y));
                    self.draw_continuous_line(Pos2::new(min_x, max_y), Pos2::new(min_x, min_y));
                }
            }
            Tool::Circle => {
                let min_x = p0.x.min(p1.x);
                let max_x = p0.x.max(p1.x);
                let min_y = p0.y.min(p1.y);
                let max_y = p0.y.max(p1.y);

                let rx = (max_x - min_x) / 2.0;
                let ry = (max_y - min_y) / 2.0;
                let cx = min_x + rx;
                let cy = min_y + ry;

                if rx < 0.5 || ry < 0.5 {
                    self.draw_point_on_active(cx, cy);
                    return;
                }

                let x_start =
                    ((cx - rx - self.brush_size).floor() as i32).clamp(0, self.width as i32 - 1);
                let x_end =
                    ((cx + rx + self.brush_size).ceil() as i32).clamp(0, self.width as i32 - 1);
                let y_start =
                    ((cy - ry - self.brush_size).floor() as i32).clamp(0, self.height as i32 - 1);
                let y_end =
                    ((cy + ry + self.brush_size).ceil() as i32).clamp(0, self.height as i32 - 1);

                let layer = &mut self.layers[self.active_layer_idx];
                let stroke_half = (self.brush_size / 2.0).max(0.5);
                let fill_color = self.background_color;
                let stroke_color = self.brush_color;
                let norm_thickness = stroke_half / ((rx + ry) / 2.0);

                for py in y_start..=y_end {
                    for px in x_start..=x_end {
                        let dx = (px as f32 + 0.5 - cx) / rx;
                        let dy = (py as f32 + 0.5 - cy) / ry;
                        let dist_sq = dx * dx + dy * dy;
                        let idx = (py as usize) * self.width + (px as usize);

                        if self.fill_shapes {
                            let dist = dist_sq.sqrt();
                            if (dist - 1.0).abs() <= norm_thickness {
                                layer.pixels[idx] = stroke_color;
                            } else if dist_sq <= 1.0 {
                                layer.pixels[idx] = fill_color;
                            }
                        } else {
                            let dist = dist_sq.sqrt();
                            if (dist - 1.0).abs() <= norm_thickness {
                                layer.pixels[idx] = stroke_color;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        self.mark_document_changed();
    }
}

fn rasterize_text(data: &TextLayerData, width: usize, height: usize, pixels: &mut [Color32]) {
    let Some(font_bytes) = load_font_bytes(&data.font_family) else {
        return;
    };
    let Ok(font) = FontRef::try_from_slice(&font_bytes) else {
        return;
    };
    let font_size = data.font_size.clamp(6.0, 400.0);
    let scale = PxScale {
        x: font_size,
        y: font_size,
    };
    let scaled = font.as_scaled(scale);
    let line_height = scaled.height() + scaled.line_gap();
    let mut cursor_x = data.pos.x;
    let mut baseline = data.pos.y + scaled.ascent();

    for ch in data.content.chars() {
        if ch == '\n' {
            cursor_x = data.pos.x;
            baseline += line_height;
            continue;
        }
        let glyph_id = font.glyph_id(ch);
        let glyph = glyph_id.with_scale_and_position(scale, point(cursor_x, baseline));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                if coverage <= 0.0 {
                    return;
                }
                let px = bounds.min.x.floor() as i32 + x as i32;
                let py = bounds.min.y.floor() as i32 + y as i32;
                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    let alpha = (coverage.clamp(0.0, 1.0) * data.color.a() as f32 / 255.0 * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    if alpha > 0 {
                        let dst = &mut pixels[py as usize * width + px as usize];
                        let source = Color32::from_rgba_unmultiplied(
                            data.color.r(),
                            data.color.g(),
                            data.color.b(),
                            alpha,
                        );
                        *dst = blend_pixel(*dst, source);
                    }
                }
            });
        }
        cursor_x += scaled.h_advance(glyph_id);
    }
}

fn blend_pixel(destination: Color32, source: Color32) -> Color32 {
    let sa = source.a() as f32 / 255.0;
    let da = destination.a() as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return Color32::TRANSPARENT;
    }
    let blend = |sc: u8, dc: u8| {
        ((sc as f32 / 255.0 * sa + dc as f32 / 255.0 * da * (1.0 - sa)) / out_a * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(
        blend(source.r(), destination.r()),
        blend(source.g(), destination.g()),
        blend(source.b(), destination.b()),
        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn load_font_bytes(font_family: &str) -> Option<Vec<u8>> {
    let candidates = match font_family {
        "Liberation Sans" => vec![
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ],
        "Noto Sans" => vec![
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ],
        "Arial" => vec![
            "/Library/Fonts/Arial.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
        ],
        _ => vec![
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        ],
    };
    candidates.iter().find_map(|path| std::fs::read(path).ok())
}
