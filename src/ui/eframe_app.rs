//! eframe adapter: presentation state, keyboard handling, and GUI composition.

use std::ops::{Deref, DerefMut};

use eframe::egui;
use egui::{Color32, Key, Stroke, TextureHandle, TextureOptions, Vec2};

use crate::{
    app::Command,
    model::{PaintApp, Tool},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Appearance {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorDialogTarget {
    Foreground,
    Background,
}

#[derive(Default)]
pub struct UiState {
    pub status_message: Option<(String, std::time::Instant)>,
    pub appearance: Appearance,
    pub applied_dark_theme: bool,
    pub zoom: f32,
    pub pan: Vec2,
    pub canvas_texture: Option<TextureHandle>,
    pub textured_document_revision: Option<u64>,
    pub layer_thumbnail_textures: Vec<TextureHandle>,
    pub layer_thumbnail_revision: Option<u64>,
    pub is_drawing: bool,
    pub drag_start_canvas: Option<egui::Pos2>,
    pub last_canvas_pos: Option<egui::Pos2>,
    pub current_canvas_pos: Option<egui::Pos2>,
    pub palette: Vec<Color32>,
    pub show_new_image_dialog: bool,
    pub new_image_width: usize,
    pub new_image_height: usize,
    pub show_resize_image_dialog: bool,
    pub resize_image_width: usize,
    pub resize_image_height: usize,
    pub lock_resize_aspect_ratio: bool,
    pub show_rotate_image_dialog: bool,
    pub rotation_angle_mode: u8,
    pub rotation_custom_degrees: u16,
    pub rotation_clockwise: bool,
    pub show_print_dialog: bool,
    pub print_output_path: String,
    pub color_dialog_target: Option<ColorDialogTarget>,
    pub color_dialog_color: Color32,
}

impl UiState {
    fn for_document(width: usize, height: usize) -> Self {
        let mut state = Self {
            appearance: Appearance::System,
            zoom: 1.0,
            new_image_width: width,
            new_image_height: height,
            resize_image_width: width,
            resize_image_height: height,
            lock_resize_aspect_ratio: true,
            print_output_path: std::env::var("HOME")
                .map(|home| format!("{home}/immagine.pdf"))
                .unwrap_or_else(|_| "immagine.pdf".to_string()),
            color_dialog_color: Color32::WHITE,
            ..Default::default()
        };
        state.palette = default_palette();
        state
    }
}

pub struct EframeApp {
    pub document: PaintApp,
    pub ui: UiState,
}

impl Deref for EframeApp {
    type Target = PaintApp;

    fn deref(&self) -> &Self::Target {
        &self.document
    }
}

impl DerefMut for EframeApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.document
    }
}

impl EframeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let document = PaintApp::new_headless();
        let mut app = Self::from_document(document);
        let theme = resolve_appearance(app.ui.appearance, cc.integration_info.system_theme);
        cc.egui_ctx.set_style(style_for_theme(theme));
        app.ui.applied_dark_theme = theme == eframe::Theme::Dark;
        app
    }

    pub fn from_document(document: PaintApp) -> Self {
        let ui = UiState::for_document(document.width, document.height);
        Self { document, ui }
    }

    pub fn execute_command(&mut self, command: Command) {
        let reset_view_after = matches!(
            command,
            Command::NewCanvas { .. } | Command::LoadPng(_) | Command::PasteInNewWindow
        );
        let revision_before = self.document.document_revision;
        let notices = self.document.execute_command(command);
        if reset_view_after && self.document.document_revision != revision_before {
            self.reset_view();
        }
        for notice in notices {
            self.show_notice(&notice);
        }
    }

    pub fn show_notice(&mut self, message: &str) {
        self.ui.status_message = Some((message.to_string(), std::time::Instant::now()));
    }

    pub fn apply_appearance(&mut self, ctx: &egui::Context, system_theme: Option<eframe::Theme>) {
        let theme = resolve_appearance(self.ui.appearance, system_theme);
        let is_dark = theme == eframe::Theme::Dark;
        if self.ui.applied_dark_theme != is_dark {
            ctx.set_style(style_for_theme(theme));
            self.ui.applied_dark_theme = is_dark;
        }
    }

    pub fn update_texture(&mut self, ui: &mut egui::Ui) {
        let revision = self.document.document_revision;
        let document_size = [self.width, self.height];
        let texture_size = self.ui.canvas_texture.as_ref().map(TextureHandle::size);
        let needs_update = self.ui.canvas_texture.is_none()
            || self.ui.textured_document_revision != Some(revision)
            || texture_size != Some(document_size);

        if needs_update {
            let image = self.document.composite();
            if texture_size == Some(document_size) {
                if let Some(texture) = &mut self.ui.canvas_texture {
                    texture.set(image, TextureOptions::NEAREST);
                }
            } else {
                self.ui.canvas_texture = Some(ui.ctx().load_texture(
                    "canvas",
                    image,
                    TextureOptions::NEAREST,
                ));
            }
            self.ui.textured_document_revision = Some(revision);
        }
    }
}

impl eframe::App for EframeApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.apply_appearance(ctx, frame.info().system_theme);

        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(Key::Z) && !i.modifiers.shift {
                self.execute_command(Command::Undo);
            }
            if (i.modifiers.command && i.key_pressed(Key::Y))
                || (i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::Z))
            {
                self.execute_command(Command::Redo);
            }
            if i.modifiers.ctrl && i.key_pressed(Key::X) && self.has_selection() {
                self.execute_command(Command::CutSelection);
            }
            if i.modifiers.ctrl && i.key_pressed(Key::C) && self.has_selection() {
                self.execute_command(Command::CopySelection);
            }
            if i.modifiers.ctrl && i.key_pressed(Key::V) && !i.modifiers.shift {
                self.execute_command(if self.clipboard.is_some() {
                    Command::Paste
                } else {
                    Command::PasteFromSystemClipboard
                });
            }
            if i.modifiers.ctrl
                && i.modifiers.shift
                && i.key_pressed(Key::V)
                && self.clipboard.is_some()
            {
                self.execute_command(Command::PasteInNewWindow);
            }
            if i.modifiers.command && i.key_pressed(Key::A) && !i.modifiers.shift {
                self.execute_command(Command::SelectAll);
            }
            if ((i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::A))
                || i.key_pressed(Key::Escape))
                && self.has_selection()
            {
                self.execute_command(Command::Deselect);
            }
            if i.key_pressed(Key::Delete) && self.has_selection() {
                self.execute_command(Command::DeleteSelection);
            }
            if i.modifiers.command && i.key_pressed(Key::S) {
                let path = self
                    .current_filepath
                    .clone()
                    .unwrap_or_else(|| "output.png".to_string());
                self.execute_command(Command::SavePng(path));
            }
            if i.modifiers.command && i.key_pressed(Key::Num0) {
                self.reset_view();
            }
            if !i.modifiers.command {
                for (key, tool) in [
                    (Key::S, Tool::Select),
                    (Key::M, Tool::Move),
                    (Key::B, Tool::Brush),
                    (Key::E, Tool::Eraser),
                    (Key::I, Tool::Pipette),
                    (Key::G, Tool::Bucket),
                    (Key::L, Tool::Line),
                    (Key::R, Tool::Rectangle),
                    (Key::C, Tool::Circle),
                ] {
                    if i.key_pressed(key) {
                        self.execute_command(Command::SetTool(tool));
                    }
                }
            }
        });

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                super::menu::show_menu_file(self, ui, ctx);
                super::menu::show_menu_edit(self, ui);
                super::menu::show_menu_view(self, ui);
                super::menu::show_menu_image(self, ui);
                super::menu::show_menu_colors(self, ui);
                super::menu::show_menu_settings(self, ui);

                ui.separator();

                if ui
                    .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("↶"))
                    .on_hover_text("Undo (Ctrl+Z)")
                    .clicked()
                {
                    self.execute_command(Command::Undo);
                }
                if ui
                    .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("↷"))
                    .on_hover_text("Redo (Ctrl+Shift+Z)")
                    .clicked()
                {
                    self.execute_command(Command::Redo);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Tela: {}x{} px", self.width, self.height));
                    if let Some(selection) = self.selection.as_ref().map(|region| region.bounds()) {
                        ui.label(format!(
                            "| Sel: {}x{} px",
                            selection.width().abs().round(),
                            selection.height().abs().round()
                        ));
                    }
                });
            });
        });

        super::panels::show_tools_panel(self, ctx);
        super::panels::show_layers_panel(self, ctx);
        super::panels::show_palette_panel(self, ctx);
        super::canvas::show_canvas(self, ctx);
        super::dialogs::show_document_dialogs(self, ctx);
        super::dialogs::show_color_dialog(self, ctx);
    }
}

impl EframeApp {
    pub fn reset_view(&mut self) {
        self.ui.zoom = 1.0;
        self.ui.pan = Vec2::ZERO;
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.ui.zoom = (self.ui.zoom * factor).clamp(0.05, 30.0);
    }
}

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

fn default_palette() -> Vec<Color32> {
    vec![
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
    ]
}

#[cfg(test)]
mod tests {
    use super::{resolve_appearance, style_for_theme, Appearance, EframeApp};
    use crate::{app::Command, model::PaintApp};

    #[test]
    fn command_notices_are_stored_by_the_ui_adapter() {
        let mut app = EframeApp::from_document(PaintApp::new_headless());

        app.execute_command(Command::NewCanvas {
            width: 0,
            height: 10,
        });

        assert!(app
            .ui
            .status_message
            .as_ref()
            .is_some_and(|(message, _)| message.contains("Dimensioni immagine non valide")));
    }

    #[test]
    fn theme_selection_respects_system_and_explicit_preferences() {
        assert_eq!(
            resolve_appearance(Appearance::System, Some(eframe::Theme::Dark)),
            eframe::Theme::Dark
        );
        assert_eq!(
            resolve_appearance(Appearance::System, None),
            eframe::Theme::Light
        );
        assert_eq!(
            resolve_appearance(Appearance::Dark, Some(eframe::Theme::Light)),
            eframe::Theme::Dark
        );
        assert_eq!(
            resolve_appearance(Appearance::Light, Some(eframe::Theme::Dark)),
            eframe::Theme::Light
        );
    }

    #[test]
    fn themes_keep_square_widget_corners() {
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
}
