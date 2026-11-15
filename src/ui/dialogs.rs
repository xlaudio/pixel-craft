//! Dialoghi per la creazione, il ridimensionamento e la stampa dei documenti.

use eframe::egui;
use egui::{Color32, Vec2};

use super::eframe_app::{ColorDialogTarget, EframeApp};
use crate::app::Command;

pub fn show_document_dialogs(app: &mut EframeApp, ctx: &egui::Context) {
    const MAX_DIMENSION: usize = 16_384;
    const MAX_PIXELS: usize = 100_000_000;

    if app.ui.show_new_image_dialog {
        let mut open = app.ui.show_new_image_dialog;
        let mut create_requested = false;
        let mut cancel_requested = false;
        egui::Window::new("Nuova immagine")
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(360.0, 210.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.weak("Dimensioni della tela in pixel");
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.button("?").on_hover_text(
                                "Inserisci larghezza e altezza in pixel. Puoi usare la rotellina del mouse sopra un campo.",
                            );
                        },
                    );
                });
                ui.add_space(6.0);
                dimension_input(ui, "Larghezza:", &mut app.ui.new_image_width, MAX_DIMENSION);
                dimension_input(ui, "Altezza:", &mut app.ui.new_image_height, MAX_DIMENSION);
                let dimensions_valid = app
                    .ui
                    .new_image_width
                    .checked_mul(app.ui.new_image_height)
                    .is_some_and(|pixels| pixels <= MAX_PIXELS);
                if !dimensions_valid {
                    ui.colored_label(Color32::RED, "La tela non può superare 100 megapixel.");
                }
                ui.add_space(8.0);
                ui.separator();
                (create_requested, cancel_requested) =
                    dialog_actions(ui, "Crea", dimensions_valid);
            });
        app.ui.show_new_image_dialog = open && !cancel_requested && !create_requested;
        if create_requested {
            app.execute_command(Command::NewCanvas {
                width: app.ui.new_image_width,
                height: app.ui.new_image_height,
            });
        }
    }

    if app.ui.show_resize_image_dialog {
        let mut open = app.ui.show_resize_image_dialog;
        let original_ratio = app.width as f64 / app.height as f64;
        let mut resize_requested = false;
        let mut cancel_requested = false;
        egui::Window::new("Ridimensiona immagine")
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(360.0, 230.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.weak(format!(
                        "Dimensioni attuali: {} x {} px",
                        app.width, app.height
                    ));
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.button("?").on_hover_text(
                                "Modifica le dimensioni in pixel. Attiva Mantieni proporzioni per conservare il rapporto originale.",
                            );
                        },
                    );
                });
                ui.add_space(6.0);
                let width_changed =
                    dimension_input(ui, "Larghezza:", &mut app.ui.resize_image_width, MAX_DIMENSION);
                if width_changed && app.ui.lock_resize_aspect_ratio {
                    app.ui.resize_image_height = ((app.ui.resize_image_width as f64 / original_ratio)
                        .round() as usize)
                        .clamp(1, MAX_DIMENSION);
                }
                let height_changed =
                    dimension_input(ui, "Altezza:", &mut app.ui.resize_image_height, MAX_DIMENSION);
                if height_changed && app.ui.lock_resize_aspect_ratio {
                    app.ui.resize_image_width = ((app.ui.resize_image_height as f64 * original_ratio)
                        .round() as usize)
                        .clamp(1, MAX_DIMENSION);
                }
                ui.add_space(4.0);
                ui.checkbox(&mut app.ui.lock_resize_aspect_ratio, "Mantieni proporzioni");
                let dimensions_valid = app
                    .ui
                    .resize_image_width
                    .checked_mul(app.ui.resize_image_height)
                    .is_some_and(|pixels| pixels <= MAX_PIXELS);
                if !dimensions_valid {
                    ui.colored_label(Color32::RED, "La tela non può superare 100 megapixel.");
                }
                ui.add_space(8.0);
                ui.separator();
                (resize_requested, cancel_requested) =
                    dialog_actions(ui, "Ridimensiona", dimensions_valid);
            });
        app.ui.show_resize_image_dialog = open && !cancel_requested && !resize_requested;
        if resize_requested {
            app.execute_command(Command::ResizeCanvas {
                width: app.ui.resize_image_width,
                height: app.ui.resize_image_height,
            });
        }
    }

    if app.ui.show_print_dialog {
        let mut open = app.ui.show_print_dialog;
        let mut print_requested = false;
        let mut cancel_requested = false;
        egui::Window::new("Stampa immagine")
            .collapsible(false)
            .resizable(false)
            .default_width(470.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Stampante").strong());
                });
                ui.add_space(8.0);
                egui::Grid::new("print_dialog_details")
                    .num_columns(3)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Nome:");
                        egui::ComboBox::from_id_source("print_dialog_printer")
                            .selected_text("Stampa su file (PDF)")
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "Stampa su file (PDF)");
                            });
                        ui.add_enabled(false, egui::Button::new("Proprietà"));
                        ui.end_row();

                        ui.label("Posizione:");
                        ui.label("File locale");
                        ui.label("");
                        ui.end_row();

                        ui.label("Tipo:");
                        ui.label("Documento PDF");
                        ui.label("");
                        ui.end_row();
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("File di output:");
                    ui.add(
                        egui::TextEdit::singleline(&mut app.ui.print_output_path)
                            .desired_width(310.0),
                    );
                    if ui.button("Sfoglia...").clicked() {
                        let mut dialog = rfd::FileDialog::new()
                            .add_filter("Documento PDF", &["pdf"])
                            .set_file_name("immagine.pdf");
                        if let Some(parent) =
                            std::path::Path::new(&app.ui.print_output_path).parent()
                        {
                            dialog = dialog.set_directory(parent);
                        }
                        if let Some(path) = dialog.save_file() {
                            app.ui.print_output_path = path.to_string_lossy().into_owned();
                        }
                    }
                });
                ui.add_space(8.0);
                ui.separator();
                (print_requested, cancel_requested) = dialog_actions(ui, "Stampa", true);
            });
        app.ui.show_print_dialog = open && !cancel_requested && !print_requested;
        if print_requested {
            if app.ui.print_output_path.trim().is_empty() {
                app.show_notice("Specifica il percorso del file PDF.");
                app.ui.show_print_dialog = true;
            } else {
                let path = app.ui.print_output_path.clone();
                app.execute_command(Command::SavePdf(path));
            }
        }
    }
}

pub fn show_color_dialog(app: &mut EframeApp, ctx: &egui::Context) {
    let Some(target) = app.ui.color_dialog_target else {
        return;
    };

    let title = match target {
        ColorDialogTarget::Foreground => "Colore primo piano",
        ColorDialogTarget::Background => "Colore sfondo",
    };
    let mut open = true;
    let mut accept_requested = false;
    let mut cancel_requested = false;
    egui::Window::new(title)
        .id(egui::Id::new("custom_color_dialog"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Colori base");
                    egui::Grid::new("color_dialog_palette")
                        .num_columns(6)
                        .spacing([3.0, 3.0])
                        .show(ui, |ui| {
                            for (index, color) in app.ui.palette.iter().copied().enumerate() {
                                if ui
                                    .add_sized([22.0, 22.0], egui::Button::new("").fill(color))
                                    .clicked()
                                {
                                    app.ui.color_dialog_color = color;
                                }
                                if index % 6 == 5 {
                                    ui.end_row();
                                }
                            }
                        });
                    ui.add_space(12.0);
                    ui.label("Anteprima");
                    egui::color_picker::show_color(
                        ui,
                        app.ui.color_dialog_color,
                        Vec2::new(150.0, 48.0),
                    );
                });

                ui.separator();
                ui.vertical(|ui| {
                    egui::color_picker::color_picker_color32(
                        ui,
                        &mut app.ui.color_dialog_color,
                        egui::color_picker::Alpha::OnlyBlend,
                    );
                });
            });

            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                cancel_requested = ui.button("Annulla").clicked();
                accept_requested = ui.button("OK").clicked();
            });
        });

    if accept_requested {
        match target {
            ColorDialogTarget::Foreground => app.execute_command(Command::SetForegroundColor(
                app.ui.color_dialog_color.into(),
            )),
            ColorDialogTarget::Background => app.execute_command(Command::SetBackgroundColor(
                app.ui.color_dialog_color.into(),
            )),
        }
        app.ui.color_dialog_target = None;
    } else if cancel_requested || !open {
        app.ui.color_dialog_target = None;
    }
}

fn dimension_input(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut usize,
    max_dimension: usize,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized([82.0, 28.0], egui::Label::new(label).wrap(false));
        let input_response = ui.add_sized(
            [190.0, 28.0],
            egui::DragValue::new(value)
                .speed(1.0)
                .clamp_range(1..=max_dimension),
        );
        changed |= input_response.changed();

        let spinner_id = ui.make_persistent_id(("dimension_spinner", label));
        let (spinner_rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 28.0), egui::Sense::hover());
        let up_rect = egui::Rect::from_min_size(spinner_rect.min, Vec2::new(18.0, 14.0));
        let down_rect = egui::Rect::from_min_size(
            egui::pos2(spinner_rect.min.x, spinner_rect.min.y + 14.0),
            Vec2::new(18.0, 14.0),
        );
        let up_response = ui.interact(up_rect, spinner_id.with("up"), egui::Sense::click());
        let down_response = ui.interact(down_rect, spinner_id.with("down"), egui::Sense::click());
        paint_spinner_arrow(ui, up_rect, &up_response, true);
        paint_spinner_arrow(ui, down_rect, &down_response, false);
        if up_response.clicked() {
            *value = value.saturating_add(1).min(max_dimension);
            changed = true;
        } else if down_response.clicked() {
            *value = value.saturating_sub(1).max(1);
            changed = true;
        }

        if input_response.hovered() || up_response.hovered() || down_response.hovered() {
            let scroll_y = ui.input_mut(|input| {
                let delta = input.raw_scroll_delta.y;
                if delta != 0.0 {
                    input.raw_scroll_delta.y = 0.0;
                    input.smooth_scroll_delta.y = 0.0;
                }
                delta
            });
            if scroll_y != 0.0 {
                if scroll_y > 0.0 {
                    *value = value.saturating_add(1).min(max_dimension);
                } else {
                    *value = value.saturating_sub(1).max(1);
                }
                changed = true;
            }
        }
    });
    changed
}

fn paint_spinner_arrow(ui: &egui::Ui, rect: egui::Rect, response: &egui::Response, up: bool) {
    let visuals = ui.style().interact(response);
    ui.painter().rect_filled(rect, 0.0, visuals.bg_fill);
    ui.painter().rect_stroke(rect, 0.0, visuals.bg_stroke);
    let center = rect.center();
    let points = if up {
        [
            egui::pos2(center.x - 4.0, center.y + 2.0),
            egui::pos2(center.x + 4.0, center.y + 2.0),
            egui::pos2(center.x, center.y - 2.0),
        ]
    } else {
        [
            egui::pos2(center.x - 4.0, center.y - 2.0),
            egui::pos2(center.x + 4.0, center.y - 2.0),
            egui::pos2(center.x, center.y + 2.0),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        points.to_vec(),
        visuals.fg_stroke.color,
        egui::Stroke::NONE,
    ));
}

fn dialog_actions(ui: &mut egui::Ui, primary_label: &str, valid: bool) -> (bool, bool) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let cancel = ui
            .add_sized([96.0, 34.0], egui::Button::new("Annulla"))
            .clicked();
        let primary = ui
            .add_enabled_ui(valid, |ui| {
                ui.add_sized([112.0, 34.0], egui::Button::new(primary_label))
            })
            .inner
            .clicked();
        (primary, cancel)
    })
    .inner
}
