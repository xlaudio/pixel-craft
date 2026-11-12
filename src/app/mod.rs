//! # app — Inizializzazione e coordinamento dello stato applicativo
//!
//! Questo modulo costruisce [`PaintApp`], configura l'aspetto e il tema,
//! gestisce le notifiche e sincronizza il compositing con la texture egui.
//! Le operazioni specifiche sono separate nei sottomoduli:
//! - [`history`] — cronologia undo/redo
//! - [`document`] — tela, livelli, selezioni e compositing
//! - [`tools`] — strumenti e algoritmi di disegno
//! - [`file_io`] — caricamento, salvataggio e appunti su file

pub mod commands;
mod document;
mod file_io;
mod history;
mod tools;

pub use commands::{Command, CommandEngine};

use eframe::egui;
use egui::{Color32, Stroke, TextureOptions, Vec2};
use std::collections::VecDeque;

use crate::model::{Appearance, Layer, PaintApp, Tool};

fn resolve_appearance(
    appearance: Appearance,
    system_theme: Option<eframe::Theme>,
) -> eframe::Theme {
    match appearance {
        Appearance::System => system_theme.unwrap_or(eframe::Theme::Light),
        Appearance::Dark => eframe::Theme::Dark,
        Appearance::Light => eframe::Theme::Light,
    }
}

fn style_for_theme(theme: eframe::Theme) -> egui::Style {
    let mut style = egui::Style::default();
    let mut visuals = theme.egui_visuals();
    let accent = Color32::from_rgb(61, 174, 233);

    if theme == eframe::Theme::Light {
        visuals.window_fill = Color32::from_rgb(239, 240, 241);
        visuals.panel_fill = Color32::from_rgb(252, 252, 252);
        visuals.extreme_bg_color = Color32::WHITE;
    }

    visuals.widgets.hovered.bg_fill = accent;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.open.bg_fill = accent;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = Stroke::new(1.0_f32, accent);
    let square_corners = egui::Rounding::same(0.0);
    visuals.window_rounding = square_corners;
    visuals.menu_rounding = square_corners;
    visuals.widgets.noninteractive.rounding = square_corners;
    visuals.widgets.inactive.rounding = square_corners;
    visuals.widgets.hovered.rounding = square_corners;
    visuals.widgets.active.rounding = square_corners;
    visuals.widgets.open.rounding = square_corners;
    visuals.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.6 };
    visuals.window_shadow = egui::epaint::Shadow::NONE;
    visuals.popup_shadow = egui::epaint::Shadow::NONE;

    style.spacing.item_spacing = Vec2::new(8.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(8.0);
    style.spacing.menu_margin = egui::Margin::same(4.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.interact_size = Vec2::new(40.0, 24.0);
    style.spacing.scroll.bar_width = 12.0;
    style.spacing.scroll.handle_min_length = 24.0;
    style.visuals = visuals;
    style
}

impl PaintApp {
    /// Execute a user action through the shared command boundary.
    pub fn execute_command(&mut self, command: Command) {
        CommandEngine.execute(self, command);
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::new_headless();
        let theme = resolve_appearance(app.appearance, cc.integration_info.system_theme);
        cc.egui_ctx.set_style(style_for_theme(theme));
        app.applied_dark_theme = theme == eframe::Theme::Dark;
        app
    }

    /// Create application state without an eframe window or creation context.
    ///
    /// Non-GUI adapters can construct the same state and execute commands
    /// directly, while the GUI adds its theme and texture integration.
    pub fn new_headless() -> Self {
        let appearance = Appearance::System;

        let width = 1024;
        let height = 768;
        let print_output_path = std::env::var("HOME")
            .map(|home| format!("{home}/immagine.pdf"))
            .unwrap_or_else(|_| "immagine.pdf".to_string());

        let mut bg_layer = Layer::new("Sfondo", width, height);
        bg_layer.pixels.fill(Color32::WHITE);
        let draw_layer = Layer::new("Livello 1", width, height);

        let palette = vec![
            Color32::BLACK,
            Color32::from_rgb(128, 128, 128),
            Color32::WHITE,
            Color32::from_rgb(251, 140, 0),
            Color32::from_rgb(253, 216, 53),
            Color32::from_rgb(255, 255, 0),
            Color32::from_rgb(0, 255, 0),
            Color32::from_rgb(0, 255, 255),
            Color32::from_rgb(0, 0, 255),
            Color32::from_rgb(255, 0, 255),
            Color32::from_rgb(255, 128, 128),
            Color32::from_rgb(128, 255, 128),
            Color32::from_rgb(173, 216, 230),
            Color32::from_rgb(192, 192, 192),
            Color32::from_rgb(128, 0, 0),
            Color32::from_rgb(128, 64, 0),
            Color32::from_rgb(128, 128, 0),
            Color32::from_rgb(0, 128, 0),
            Color32::from_rgb(0, 128, 128),
            Color32::from_rgb(0, 0, 128),
            Color32::from_rgb(128, 0, 128),
            Color32::from_rgb(128, 128, 255),
            Color32::from_rgb(255, 255, 128),
            Color32::from_rgb(255, 0, 0),
            Color32::from_rgb(255, 128, 0),
            Color32::from_rgb(128, 255, 0),
            Color32::from_rgb(0, 128, 255),
            Color32::from_rgb(255, 0, 128),
            Color32::from_rgb(64, 64, 64),
            Color32::from_rgb(192, 128, 64),
        ];

        Self {
            width,
            height,
            layers: vec![bg_layer, draw_layer],
            active_layer_idx: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 25,
            selection: None,
            clipboard: None,
            status_message: None,
            dirty_canvas: true,
            current_tool: Tool::Brush,
            brush_size: 10.0,
            brush_color: Color32::from_rgb(200, 50, 50),
            background_color: Color32::WHITE,
            fill_shapes: false,
            appearance,
            applied_dark_theme: false,
            zoom: 1.0,
            pan: Vec2::ZERO,
            canvas_texture: None,
            is_drawing: false,
            drag_start_canvas: None,
            last_canvas_pos: None,
            current_canvas_pos: None,
            moving_selection: None,
            palette,
            current_filepath: None,
            show_new_image_dialog: false,
            new_image_width: width,
            new_image_height: height,
            show_resize_image_dialog: false,
            resize_image_width: width,
            resize_image_height: height,
            lock_resize_aspect_ratio: true,
            show_print_dialog: false,
            print_output_path,
            color_dialog_target: None,
            color_dialog_color: Color32::WHITE,
            _flood_queue: VecDeque::new(),
        }
    }

    pub fn show_notice(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), std::time::Instant::now()));
    }

    pub fn apply_appearance(&mut self, ctx: &egui::Context, system_theme: Option<eframe::Theme>) {
        let theme = resolve_appearance(self.appearance, system_theme);
        let is_dark = theme == eframe::Theme::Dark;
        if self.applied_dark_theme != is_dark {
            ctx.set_style(style_for_theme(theme));
            self.applied_dark_theme = is_dark;
        }
    }

    pub fn update_texture(&mut self, ui: &mut egui::Ui) {
        if self.dirty_canvas || self.canvas_texture.is_none() {
            let img_data = self.composite();
            let need_new_texture = match &self.canvas_texture {
                Some(tex) => tex.size() != [self.width, self.height],
                None => true,
            };

            if need_new_texture {
                self.canvas_texture = Some(ui.ctx().load_texture(
                    "canvas",
                    img_data,
                    TextureOptions::NEAREST,
                ));
            } else if let Some(tex) = &mut self.canvas_texture {
                tex.set(img_data, TextureOptions::NEAREST);
            }

            self.dirty_canvas = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_appearance, style_for_theme};
    use crate::model::Appearance;

    #[test]
    fn both_themes_use_square_widget_corners() {
        let square = egui::Rounding::same(0.0);
        for theme in [eframe::Theme::Light, eframe::Theme::Dark] {
            let visuals = style_for_theme(theme).visuals;
            assert_eq!(visuals.window_rounding, square);
            assert_eq!(visuals.menu_rounding, square);
            assert_eq!(visuals.widgets.noninteractive.rounding, square);
            assert_eq!(visuals.widgets.inactive.rounding, square);
            assert_eq!(visuals.widgets.hovered.rounding, square);
            assert_eq!(visuals.widgets.active.rounding, square);
            assert_eq!(visuals.widgets.open.rounding, square);
        }
    }

    #[test]
    fn appearance_uses_system_theme_when_available() {
        assert_eq!(
            resolve_appearance(Appearance::System, Some(eframe::Theme::Dark)),
            eframe::Theme::Dark
        );
    }

    #[test]
    fn appearance_falls_back_to_light_when_system_theme_is_unavailable() {
        assert_eq!(
            resolve_appearance(Appearance::System, None),
            eframe::Theme::Light
        );
    }

    #[test]
    fn explicit_appearance_overrides_system_theme() {
        assert_eq!(
            resolve_appearance(Appearance::Dark, Some(eframe::Theme::Light)),
            eframe::Theme::Dark
        );
        assert_eq!(
            resolve_appearance(Appearance::Light, Some(eframe::Theme::Dark)),
            eframe::Theme::Light
        );
    }
}
