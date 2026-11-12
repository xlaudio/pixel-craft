//! eframe integration: keyboard handling and top-level GUI composition.

use eframe::egui;
use egui::Key;

use crate::app::Command;
use crate::model::{PaintApp, Tool};

impl eframe::App for PaintApp {
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
            if i.modifiers.command && i.key_pressed(Key::X) && self.has_selection() {
                self.execute_command(Command::CutSelection);
            }
            if i.modifiers.command && i.key_pressed(Key::C) && self.has_selection() {
                self.execute_command(Command::CopySelection);
            }
            if i.modifiers.command && i.key_pressed(Key::V) && !i.modifiers.shift {
                self.execute_command(Command::PasteFromSystemClipboard);
            }
            if i.modifiers.command
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
                self.execute_command(Command::ResetView);
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
                    if let Some(selection) = self.selection {
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
