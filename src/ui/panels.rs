//! # panels.rs — Pannelli dell'interfaccia
//!
//! Contiene i pannelli egui fissi ai lati e in basso nella finestra principale.
//! I pannelli leggono e modificano direttamente lo stato di [`PaintApp`].
//!
//! ## Pannelli
//! - [`show_tools_panel`]  — **Pannello sinistro**: selezione strumento attivo
//!   (Select, Brush, Eraser, Pipette, Bucket, Line, Rectangle, Circle),
//!   slider per la dimensione del pennello e indicazioni sulla navigazione.
//! - [`show_layers_panel`] — **Pannello destro**: lista dei livelli con controlli
//!   visibilità, opacità, riordinamento (↑↓), duplicazione ed eliminazione.
//!   Consente anche di fondere il livello selezionato con quelli sottostanti.
//!   Le azioni sui livelli vengono applicate fuori dal loop di borrow mutabile
//!   per evitare conflitti con il borrow checker di Rust.
//! - [`show_palette_panel`] — **Pannello inferiore**: tavolozza rapida su due righe;
//!   click sinistro per il colore di primo piano, destro per quello di sfondo.

use eframe::egui;
use egui::{Color32, Sense, Stroke, Vec2};

use crate::app::Command;
use crate::model::{ColorDialogTarget, PaintApp, Tool};

// ============================================================================
// PANNELLO STRUMENTI (SX)
// ============================================================================

pub fn show_tools_panel(app: &mut PaintApp, ctx: &egui::Context) {
    egui::SidePanel::left("tools_panel")
        .resizable(false)
        .default_width(170.0)
        .show(ctx, |ui| {
            ui.heading("Strumenti");
            ui.separator();

            for row in [
                [(Tool::Select, "Selezione", "S"), (Tool::Move, "Sposta", "M")],
                [(Tool::Brush, "Pennello", "B"), (Tool::Eraser, "Gomma", "E")],
                [(Tool::Pipette, "Contagocce", "I"), (Tool::Bucket, "Secchiello", "G")],
                [(Tool::Line, "Linea", "L"), (Tool::Rectangle, "Rettangolo", "R")],
            ]
            .iter()
            {
                ui.horizontal(|ui| {
                    for &(tool, label, shortcut) in row {
                        if tool_button(ui, app.current_tool, tool, label, shortcut) {
                            app.execute_command(Command::SetTool(tool));
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                if tool_button(ui, app.current_tool, Tool::Circle, "Ellisse", "C") {
                    app.execute_command(Command::SetTool(Tool::Circle));
                }
            });
            ui.add_space(6.0);
            ui.add_space(6.0);
            if matches!(app.current_tool, Tool::Rectangle | Tool::Circle) {
                let mut fill_shapes = app.fill_shapes;
                if ui.checkbox(&mut fill_shapes, "Riempi forma").changed() {
                    app.execute_command(Command::SetFillShapes(fill_shapes));
                }
            }

            ui.separator();
            ui.label("Dimensione:");
            let mut brush_size = app.brush_size;
            if ui
                .add(egui::Slider::new(&mut brush_size, 1.0..=80.0).suffix(" px"))
                .changed()
            {
                app.execute_command(Command::SetBrushSize(brush_size));
            }

            ui.add_space(10.0);
            ui.separator();
            ui.label(egui::RichText::new("Navigazione:").weak());
            ui.label(egui::RichText::new("• Scrollbar o tool Sposta (M): muovi tela/selezione\n• Ctrl+Scroll: zoom\n• Click destro/centrale o Spazio+Drag: Pan").small().weak());
        });
}

fn tool_button(
    ui: &mut egui::Ui,
    current_tool: Tool,
    tool: Tool,
    label: &str,
    shortcut: &str,
) -> bool {
    let response = ui.add_sized(
        Vec2::splat(32.0),
        egui::SelectableLabel::new(current_tool == tool, ""),
    );
    let clicked = response.clicked();

    let color = if current_tool == tool {
        ui.visuals().widgets.active.fg_stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    paint_tool_icon(ui.painter(), response.rect.shrink(8.0), tool, color);
    response.on_hover_text(format!("{label} ({shortcut})"));
    clicked
}

fn paint_tool_icon(painter: &egui::Painter, rect: egui::Rect, tool: Tool, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let center = rect.center();
    match tool {
        Tool::Select => {
            painter.rect_stroke(rect, 1.0, stroke);
        }
        Tool::Move => {
            painter.line_segment([rect.left_center(), rect.right_center()], stroke);
            painter.line_segment([rect.center_top(), rect.center_bottom()], stroke);
            painter.circle_filled(center, 2.0_f32, color);
        }
        Tool::Brush => {
            painter.line_segment(
                [rect.left_bottom(), rect.right_top()],
                Stroke::new(3.0_f32, color),
            );
            painter.circle_filled(rect.left_bottom(), 2.0_f32, color);
        }
        Tool::Eraser => {
            painter.line_segment(
                [rect.left_bottom(), rect.right_top()],
                Stroke::new(5.0_f32, color),
            );
        }
        Tool::Pipette => {
            painter.line_segment(
                [rect.left_bottom(), rect.right_top()],
                Stroke::new(2.0_f32, color),
            );
            painter.circle_stroke(rect.left_bottom(), 3.0_f32, stroke);
            painter.circle_filled(rect.right_top(), 1.5, color);
        }
        Tool::Bucket => {
            let top_left = rect.left_top() + Vec2::new(2.0, 3.0);
            let top_right = rect.right_top() - Vec2::new(2.0, 5.0);
            let bottom_left = rect.left_bottom() + Vec2::new(3.0, -2.0);
            let bottom_right = rect.right_bottom() - Vec2::new(3.0, 2.0);
            painter.line_segment([top_left, top_right], stroke);
            painter.line_segment([top_left, bottom_left], stroke);
            painter.line_segment([top_right, bottom_right], stroke);
            painter.line_segment([bottom_left, bottom_right], stroke);
            painter.line_segment([top_right, rect.right_top()], stroke);
        }
        Tool::Line => {
            painter.line_segment([rect.left_bottom(), rect.right_top()], stroke);
        }
        Tool::Rectangle => {
            painter.rect_stroke(rect, 0.0, stroke);
        }
        Tool::Circle => {
            painter.circle_stroke(center, rect.width().min(rect.height()) / 2.0, stroke);
        }
    }
}

pub fn show_palette_panel(app: &mut PaintApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("palette_panel")
        .resizable(false)
        .show(ctx, |ui| {
            let mut zoom_indicator_y = None;
            ui.horizontal(|ui| {
                let (preview_rect, preview_response) =
                    ui.allocate_exact_size(Vec2::new(42.0, 40.0), Sense::hover());
                preview_response.on_hover_text("Primo piano: sinistro · Sfondo: destro");
                let foreground_rect = egui::Rect::from_min_size(
                    preview_rect.min + Vec2::new(2.0, 2.0),
                    Vec2::splat(22.0),
                );
                let background_rect = egui::Rect::from_min_size(
                    preview_rect.min + Vec2::new(14.0, 14.0),
                    Vec2::splat(22.0),
                );
                let background_response = ui.interact(
                    background_rect,
                    egui::Id::new("background_color_swatch"),
                    Sense::click(),
                );
                let foreground_response = ui.interact(
                    foreground_rect,
                    egui::Id::new("foreground_color_swatch"),
                    Sense::click(),
                );
                ui.painter()
                    .rect_filled(background_rect, 0.0, app.background_color);
                ui.painter().rect_stroke(
                    background_rect,
                    0.0,
                    Stroke::new(1.0_f32, Color32::DARK_GRAY),
                );
                ui.painter()
                    .rect_filled(foreground_rect, 0.0, app.brush_color);
                ui.painter().rect_stroke(
                    foreground_rect,
                    0.0,
                    Stroke::new(2.0_f32, Color32::from_rgb(61, 174, 233)),
                );
                if background_response.double_clicked_by(egui::PointerButton::Primary) {
                    app.color_dialog_target = Some(ColorDialogTarget::Background);
                    app.color_dialog_color = app.background_color;
                }
                if foreground_response.double_clicked_by(egui::PointerButton::Primary) {
                    app.color_dialog_target = Some(ColorDialogTarget::Foreground);
                    app.color_dialog_color = app.brush_color;
                }
                background_response.on_hover_text("Sfondo · doppio clic per scegliere un colore");
                foreground_response
                    .on_hover_text("Primo piano · doppio clic per scegliere un colore");
                ui.add_space(8.0);

                let swatch_size = 20.0;
                let columns = 15;
                let palette = app.palette.clone();
                let (palette_rect, _) = ui.allocate_exact_size(
                    Vec2::new(swatch_size * columns as f32, swatch_size * 2.0),
                    Sense::hover(),
                );
                zoom_indicator_y = Some(palette_rect.top() + swatch_size * 1.5);
                for (i, &color) in palette.iter().take(30).enumerate() {
                    let col = i % columns;
                    let row = i / columns;
                    let rect = egui::Rect::from_min_size(
                        palette_rect.min
                            + Vec2::new(col as f32 * swatch_size, row as f32 * swatch_size),
                        Vec2::splat(swatch_size),
                    );
                    let response =
                        ui.interact(rect, egui::Id::new(("palette_swatch", i)), Sense::click());
                    ui.painter().rect_filled(rect, 0.0, color);
                    ui.painter()
                        .rect_stroke(rect, 0.0, Stroke::new(1.0_f32, Color32::WHITE));

                    let is_foreground = app.brush_color == color;
                    let is_background = app.background_color == color;
                    if is_foreground || is_background {
                        let selected_color = if is_foreground {
                            Color32::from_rgb(61, 174, 233)
                        } else {
                            Color32::from_rgb(230, 126, 34)
                        };
                        ui.painter().rect_stroke(
                            rect.shrink(2.0),
                            0.0,
                            Stroke::new(1.0_f32, selected_color),
                        );
                    }
                    if is_foreground && is_background {
                        ui.painter().line_segment(
                            [rect.left_top(), rect.right_bottom()],
                            Stroke::new(1.0_f32, Color32::from_rgb(230, 126, 34)),
                        );
                    }

                    if response.clicked_by(egui::PointerButton::Primary) {
                        app.execute_command(Command::SetForegroundColor(color.into()));
                    } else if response.clicked_by(egui::PointerButton::Secondary) {
                        app.execute_command(Command::SetBackgroundColor(color.into()));
                    }
                    response.on_hover_text("Sinistro: primo piano · Destro: sfondo");
                }
            });

            if let Some(y) = zoom_indicator_y {
                egui::Area::new(egui::Id::new("zoom_indicator"))
                    .pivot(egui::Align2::RIGHT_CENTER)
                    .fixed_pos(egui::pos2(ui.max_rect().right() - 8.0, y))
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.small_button("−").on_hover_text("Riduci zoom").clicked() {
                                    app.execute_command(Command::ZoomBy(1.0 / 1.2));
                                }
                                ui.label(format!("{:.0}%", app.zoom * 100.0));
                                if ui.small_button("+").on_hover_text("Aumenta zoom").clicked() {
                                    app.execute_command(Command::ZoomBy(1.2));
                                }
                                if ui.small_button("Reset").clicked() {
                                    app.execute_command(Command::ResetView);
                                }
                            });
                        });
                    });
            }
        });
}

// ============================================================================
// PANNELLO LIVELLI (DX)
// ============================================================================

pub fn show_layers_panel(app: &mut PaintApp, ctx: &egui::Context) {
    egui::SidePanel::right("layers_panel")
        .resizable(true)
        .default_width(220.0)
        .show(ctx, |ui| {
            ui.heading("Livelli");

            ui.horizontal(|ui| {
                if ui
                    .button("➕ Nuovo")
                    .on_hover_text("Crea nuovo livello")
                    .clicked()
                {
                    app.execute_command(Command::CreateLayer);
                }

                if ui
                    .button("📋 Duplica")
                    .on_hover_text("Duplica livello selezionato")
                    .clicked()
                {
                    app.execute_command(Command::DuplicateActiveLayer);
                }
            });

            ui.separator();

            let mut to_delete = None;
            let mut move_up = None;
            let mut move_down = None;
            let mut commands = Vec::new();
            let layers_len = app.layers.len();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, layer) in app.layers.iter().enumerate().rev() {
                    let is_active = app.active_layer_idx == idx;
                    let mut visible = layer.visible;
                    let mut opacity = layer.opacity;

                    let accent = Color32::from_rgb(115, 115, 115);
                    let selected_fill = Color32::from_rgb(215, 215, 215);
                    let selected_text = Color32::from_rgb(35, 35, 35);

                    egui::Frame::group(ui.style())
                        .fill(if is_active {
                            selected_fill
                        } else {
                            ui.visuals().widgets.noninteractive.bg_fill
                        })
                        .stroke(if is_active {
                            Stroke::new(2.0_f32, accent)
                        } else {
                            ui.visuals().widgets.noninteractive.bg_stroke
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (eye_rect, eye_response) =
                                    ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::click());
                                let eye_color = if is_active {
                                    Color32::WHITE
                                } else {
                                    ui.visuals().text_color()
                                };
                                let eye_stroke = Stroke::new(1.5_f32, eye_color);
                                let eye_points = [
                                    eye_rect.left_center(),
                                    eye_rect.center_top(),
                                    eye_rect.right_center(),
                                    eye_rect.center_bottom(),
                                    eye_rect.left_center(),
                                ];
                                ui.painter()
                                    .add(egui::Shape::line(eye_points.to_vec(), eye_stroke));
                                if visible {
                                    ui.painter().circle_filled(
                                        eye_rect.center(),
                                        3.0_f32,
                                        eye_color,
                                    );
                                } else {
                                    ui.painter().line_segment(
                                        [eye_rect.left_bottom(), eye_rect.right_top()],
                                        eye_stroke,
                                    );
                                }
                                let eye_response = eye_response.on_hover_text(if visible {
                                    "Visibile — clicca per nascondere"
                                } else {
                                    "Nascosto — clicca per mostrare"
                                });
                                if eye_response.clicked() {
                                    visible = !visible;
                                    commands.push(Command::SetLayerVisible {
                                        index: idx,
                                        visible,
                                    });
                                }

                                let name_color = if is_active {
                                    selected_text
                                } else {
                                    ui.visuals().text_color()
                                };
                                let name_button = egui::Button::new(
                                    egui::RichText::new(&layer.name).strong().color(name_color),
                                )
                                .fill(if is_active {
                                    selected_fill
                                } else {
                                    ui.visuals().widgets.inactive.bg_fill
                                })
                                .stroke(if is_active {
                                    Stroke::new(1.0_f32, accent)
                                } else {
                                    ui.visuals().widgets.inactive.bg_stroke
                                });
                                if ui
                                    .add_sized([ui.available_width(), 28.0], name_button)
                                    .clicked()
                                {
                                    commands.push(Command::SetActiveLayer(idx));
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if layers_len > 1
                                            && ui
                                                .small_button("🗑")
                                                .on_hover_text("Elimina livello")
                                                .clicked()
                                        {
                                            to_delete = Some(idx);
                                        }
                                        if idx > 0
                                            && ui
                                                .small_button("⬇")
                                                .on_hover_text("Sposta giù")
                                                .clicked()
                                        {
                                            move_down = Some(idx);
                                        }
                                        if idx > 0
                                            && ui
                                                .small_button("⇩")
                                                .on_hover_text(
                                                    "Fondi questo livello e tutti quelli sottostanti",
                                                )
                                                .clicked()
                                        {
                                            commands.push(Command::MergeLayersDown(idx));
                                        }
                                        if idx + 1 < layers_len
                                            && ui
                                                .small_button("⬆")
                                                .on_hover_text("Sposta su")
                                                .clicked()
                                        {
                                            move_up = Some(idx);
                                        }
                                    },
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Opacità:").small());
                                if ui
                                    .add(
                                        egui::Slider::new(&mut opacity, 0.0..=1.0)
                                            .show_value(false),
                                    )
                                    .changed()
                                {
                                    commands.push(Command::SetLayerOpacity {
                                        index: idx,
                                        opacity,
                                    });
                                }
                                ui.label(format!("{:.0}%", opacity * 100.0));
                            });
                        });
                    ui.add_space(2.0);
                }
            });

            for command in commands {
                app.execute_command(command);
            }
            if let Some(idx) = to_delete {
                app.execute_command(Command::DeleteLayer(idx));
            }
            if let Some(idx) = move_up {
                app.execute_command(Command::MoveLayerUp(idx));
            }
            if let Some(idx) = move_down {
                app.execute_command(Command::MoveLayerDown(idx));
            }
        });
}
