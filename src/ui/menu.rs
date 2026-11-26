//! # menu.rs — Barra dei menu dell'applicazione
//!
//! Contiene le voci dei menu a tendina visibili nella barra superiore.
//!
//! ## Funzioni esportate
//! - [`menu_action_button`] — Helper: crea un pulsante da menu con icona,
//!   testo, shortcut e stato abilitato/disabilitato
//! - [`show_menu_file`]     — Menu **File**: Nuovo, Apri, Salva, Salva con nome,
//!   Esci
//! - [`show_menu_edit`]     — Menu **Edit**: Undo/Redo, Cut/Copy/Paste,
//!   Select All, Deselect, Copy/Paste to/from File
//! - [`show_menu_view`]     — Menu **View**: Zoom 100%, Ingrandisci, Rimpicciolisci, Centra
//! - [`show_menu_image`]    — Menu **Image**: Ritaglia, Svuota livello,
//!   Flip, Ruota immagine, Inverti colori, Ridimensiona
//! - [`show_menu_colors`]   — Menu **Colors**: Inverti colori, Modifica tavolozza
//! - [`show_menu_settings`] — Menu **Settings**: Appearance, scorciatoie e barre

use eframe::egui;
use egui::Vec2;

use super::eframe_app::{Appearance, EframeApp};
use crate::app::Command;

// ============================================================================
// HELPER PER VOCI DEI MENU
// ============================================================================

/// Crea un pulsante da menu con icona, testo, shortcut e stato abilitato/disabilitato.
pub fn menu_action_button(
    ui: &mut egui::Ui,
    enabled: bool,
    icon: &str,
    text: &str,
    shortcut: &str,
) -> egui::Response {
    let label = if icon.is_empty() {
        format!("       {}", text)
    } else {
        format!("{}  {}", icon, text)
    };

    let mut btn = egui::Button::new(label);
    if !shortcut.is_empty() {
        btn = btn.shortcut_text(shortcut);
    }
    btn = btn.min_size(Vec2::new(235.0, 24.0));
    ui.add_enabled(enabled, btn)
}

// ============================================================================
// MENU FILE
// ============================================================================

pub fn show_menu_file(app: &mut EframeApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.menu_button("File", |ui| {
        if ui.button("📄 Nuovo...").clicked() {
            app.ui.new_image_width = app.width;
            app.ui.new_image_height = app.height;
            app.ui.show_new_image_dialog = true;
            ui.close_menu();
        }
        if ui.button("📂 Apri...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Documento OpenRaster", &["ora"])
                .add_filter("Immagine PNG", &["png"])
                .add_filter("Immagine JPEG", &["jpg", "jpeg"])
                .add_filter("Immagine BMP", &["bmp"])
                .add_filter("Immagine JPEG", &["jpg", "jpeg"])
                .pick_file()
            {
                if let Some(path_str) = path.to_str() {
                    app.execute_command(open_command_for_path(path_str.to_owned()));
                }
            }
            ui.close_menu();
        }
        ui.separator();
        if ui.button("💾 Salva").clicked() {
            let path = app
                .current_filepath
                .clone()
                .unwrap_or_else(|| "output.png".to_string());
            app.execute_command(save_command_for_path(path));
            ui.close_menu();
        }
        if ui.button("💾 Salva con nome...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Documento OpenRaster", &["ora"])
                .add_filter("Immagine PNG", &["png"])
                .add_filter("Immagine JPEG", &["jpg", "jpeg"])
                .add_filter("Immagine BMP", &["bmp"])
                .add_filter("Immagine WebP", &["webp"])
                .save_file()
            {
                if let Some(path_str) = path.to_str() {
                    app.execute_command(save_command_for_path(path_str.to_owned()));
                }
            }
            ui.close_menu();
        }
        if ui.button("🖨 Stampa...").clicked() {
            app.ui.show_print_dialog = true;
            ui.close_menu();
        }
        ui.separator();
        if ui.button("🚪 Esci").clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
}

fn save_command_for_path(path: String) -> Command {
    let extension = std::path::Path::new(&path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");
    if extension.eq_ignore_ascii_case("ora") {
        Command::SaveOpenRaster(path)
    } else if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        Command::SaveJpeg(path)
    } else if extension.eq_ignore_ascii_case("bmp") {
        Command::SaveBmp(path)
    } else if extension.eq_ignore_ascii_case("webp") {
        Command::SaveWebP(path)
    } else {
        Command::SavePng(path)
    }
}

fn open_command_for_path(path: String) -> Command {
    if std::path::Path::new(&path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ora"))
    {
        Command::LoadOpenRaster(path)
    } else {
        Command::LoadPng(path)
    }
}

// ============================================================================
// MENU EDIT
// ============================================================================

pub fn show_menu_edit(app: &mut EframeApp, ui: &mut egui::Ui) {
    ui.menu_button("Edit", |ui| {
        ui.set_min_width(240.0);

        let can_undo = !app.undo_stack.is_empty();
        let can_redo = !app.redo_stack.is_empty();
        let has_sel = app.has_selection();
        let can_paste = true;
        let can_paste_in_new_window = app.clipboard.is_some();

        if menu_action_button(ui, can_undo, "↶", "Undo", "Ctrl+Z").clicked() {
            app.execute_command(Command::Undo);
            ui.close_menu();
        }
        if menu_action_button(ui, can_redo, "↷", "Redo", "Ctrl+Shift+Z").clicked() {
            app.execute_command(Command::Redo);
            ui.close_menu();
        }
        ui.separator();
        if menu_action_button(ui, has_sel, "✂", "Cut", "Ctrl+X").clicked() {
            app.execute_command(Command::CutSelection);
            ui.close_menu();
        }
        if menu_action_button(ui, has_sel, "📄", "Copy", "Ctrl+C").clicked() {
            app.execute_command(Command::CopySelection);
            ui.close_menu();
        }
        if menu_action_button(ui, can_paste, "📋", "Paste as New Layer", "Ctrl+V").clicked() {
            app.execute_command(if app.clipboard.is_some() {
                Command::Paste
            } else {
                Command::PasteFromSystemClipboard
            });
            ui.close_menu();
        }
        if menu_action_button(
            ui,
            can_paste_in_new_window,
            "",
            "Paste in New Window",
            "Ctrl+Shift+V",
        )
        .clicked()
        {
            app.execute_command(Command::PasteInNewWindow);
            ui.close_menu();
        }
        if menu_action_button(ui, has_sel, "", "Delete Selection", "Del").clicked() {
            app.execute_command(Command::DeleteSelection);
            ui.close_menu();
        }
        ui.separator();
        if menu_action_button(ui, true, "⛶", "Select All", "Ctrl+A").clicked() {
            app.execute_command(Command::SelectAll);
            ui.close_menu();
        }
        if menu_action_button(ui, has_sel, "⬚", "Deselect", "Ctrl+Shift+A").clicked() {
            app.execute_command(Command::Deselect);
            ui.close_menu();
        }
        ui.separator();
        if menu_action_button(ui, has_sel, "", "Copy to File...", "").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Immagine PNG", &["png"])
                .save_file()
            {
                if let Some(path) = path.to_str() {
                    app.execute_command(Command::CopySelectionToFile(path.to_owned()));
                }
            }
            ui.close_menu();
        }
        if menu_action_button(ui, true, "", "Paste From File...", "").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Immagini", &["png", "jpg", "jpeg"])
                .pick_file()
            {
                if let Some(path) = path.to_str() {
                    app.execute_command(Command::PasteFromFile(path.to_owned()));
                }
            }
            ui.close_menu();
        }
    });
}

// ============================================================================
// MENU VIEW
// ============================================================================

pub fn show_menu_view(app: &mut EframeApp, ui: &mut egui::Ui) {
    ui.menu_button("View", |ui| {
        if ui.button("🔍 Zoom 100% (Ctrl+0)").clicked() {
            app.reset_view();
            ui.close_menu();
        }
        if ui.button("➕ Ingrandisci").clicked() {
            app.zoom_by(1.25);
            ui.close_menu();
        }
        if ui.button("➖ Rimpicciolisci").clicked() {
            app.zoom_by(1.0 / 1.25);
            ui.close_menu();
        }
        if ui.button("🎯 Centra Vista").clicked() {
            app.reset_view();
            ui.close_menu();
        }
    });
}

// ============================================================================
// MENU IMAGE
// ============================================================================

pub fn show_menu_image(app: &mut EframeApp, ui: &mut egui::Ui) {
    ui.menu_button("Image", |ui| {
        let has_sel = app.has_selection();
        if ui
            .add_enabled(has_sel, egui::Button::new("✂ Ritaglia alla selezione"))
            .clicked()
        {
            app.execute_command(Command::CropToSelection);
            ui.close_menu();
        }
        if ui.button("🧹 Svuota livello attivo").clicked() {
            app.execute_command(Command::ClearActiveLayer);
            ui.close_menu();
        }
        ui.separator();
        if ui.button("↔ Rifletti orizzontalmente").clicked() {
            app.execute_command(Command::FlipHorizontal);
            ui.close_menu();
        }
        if ui.button("↕ Rifletti verticalmente").clicked() {
            app.execute_command(Command::FlipVertical);
            ui.close_menu();
        }
        if ui.button("🔄 Ruota immagine...").clicked() {
            app.ui.rotation_angle_mode = 0;
            app.ui.rotation_custom_degrees = 2;
            app.ui.rotation_clockwise = true;
            app.ui.show_rotate_image_dialog = true;
            ui.close_menu();
        }
        if ui.button("🎨 Inverti colori").clicked() {
            app.execute_command(Command::InvertColors);
            ui.close_menu();
        }
        ui.separator();
        if ui.button("📐 Ridimensiona / Scala...").clicked() {
            app.ui.resize_image_width = app.width;
            app.ui.resize_image_height = app.height;
            app.ui.lock_resize_aspect_ratio = true;
            app.ui.show_resize_image_dialog = true;
            ui.close_menu();
        }
    });
}

// ============================================================================
// MENU COLORS
// ============================================================================

pub fn show_menu_colors(app: &mut EframeApp, ui: &mut egui::Ui) {
    ui.menu_button("Colors", |ui| {
        if ui.button("🎨 Inverti colori").clicked() {
            app.execute_command(Command::InvertColors);
            ui.close_menu();
        }
        if ui.button("Modifica tavolozza...").clicked() {
            app.show_notice("Not yet implemented: Modifica tavolozza");
            ui.close_menu();
        }
    });
}

// ============================================================================
// MENU SETTINGS
// ============================================================================

pub fn show_menu_settings(app: &mut EframeApp, ui: &mut egui::Ui) {
    ui.menu_button("Settings", |ui| {
        ui.menu_button("Appearance", |ui| {
            if ui
                .selectable_label(app.ui.appearance == Appearance::System, "System")
                .on_hover_text(
                    "Segue il tema di sistema quando il sistema operativo lo rende disponibile.",
                )
                .clicked()
            {
                app.ui.appearance = Appearance::System;
            }
            if ui
                .selectable_label(app.ui.appearance == Appearance::Dark, "Dark")
                .clicked()
            {
                app.ui.appearance = Appearance::Dark;
            }
            if ui
                .selectable_label(app.ui.appearance == Appearance::Light, "Light")
                .clicked()
            {
                app.ui.appearance = Appearance::Light;
            }
        });
        ui.separator();
        if ui.button("Configura scorciatoie...").clicked() {
            app.show_notice("Not yet implemented: Configura scorciatoie");
            ui.close_menu();
        }
        if ui.button("Configura barre degli strumenti...").clicked() {
            app.show_notice("Not yet implemented: Configura barre degli strumenti");
            ui.close_menu();
        }
    });
}
