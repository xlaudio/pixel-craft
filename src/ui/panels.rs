//! # panels.rs — Pannelli dell'interfaccia
//!
//! Contiene i pannelli egui fissi ai lati e in basso nella finestra principale.
//! I pannelli leggono lo stato del documento e aggiornano quello UI
//! attraverso l'adattatore [`EframeApp`].
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

use super::eframe_app::{ColorDialogTarget, EframeApp};
use crate::app::Command;
use crate::model::Tool;

// ============================================================================
// PANNELLO STRUMENTI (SX)
// ============================================================================

pub fn show_tools_panel(app: &mut EframeApp, ctx: &egui::Context) {
    egui::SidePanel::left("tools_panel")
        .resizable(false)
        .default_width(104.0)
        .show(ctx, |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                for row in [
                    [(Tool::Select, "Selezione rettangolare", "S"), (Tool::Move, "Sposta", "M")],
                    [
                        (Tool::EllipseSelect, "Selezione ellittica", ""),
                        (Tool::LassoSelect, "Selezione libera", ""),
                    ],
                    [(Tool::Brush, "Pennello", "B"), (Tool::Spray, "Spray", "A")],
                    [(Tool::Eraser, "Gomma", "E"), (Tool::Pipette, "Contagocce", "I")],
                    [(Tool::Bucket, "Secchiello", "G"), (Tool::Line, "Linea", "L")],
                    [
                        (Tool::ConnectedLines, "Linee collegate", "P"),
                        (Tool::Rectangle, "Rettangolo", "R"),
                    ],
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
            });

            if matches!(app.current_tool, Tool::Rectangle | Tool::Circle) {
                let mut fill_shapes = app.fill_shapes;
                if ui.checkbox(&mut fill_shapes, "Riempi forma").changed() {
                    app.execute_command(Command::SetFillShapes(fill_shapes));
                }
            }

            ui.separator();
            let mut brush_size = app.brush_size;
            ui.horizontal(|ui| {
                ui.label("⌀")
                    .on_hover_text("Dimensione pennello e spray in pixel");
                if ui
                    .add(
                        egui::DragValue::new(&mut brush_size)
                            .clamp_range(1.0..=80.0)
                            .speed(1.0)
                            .suffix(" px"),
                    )
                    .changed()
                {
                    app.execute_command(Command::SetBrushSize(brush_size));
                }
            });

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
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
    let response = ui.interact(rect, egui::Id::new(("tool_button", label)), Sense::click());
    let selected = current_tool == tool;
    let visuals = ui.visuals();
    let dark_mode = visuals.dark_mode;
    let accent = Color32::from_rgb(61, 174, 233);
    let pressed = response.is_pointer_button_down_on() && !selected;
    let button_rect = if pressed {
        rect.translate(Vec2::new(0.0, 1.0))
    } else {
        rect
    };
    let fill = if selected {
        if dark_mode {
            Color32::from_rgb(43, 74, 92)
        } else {
            Color32::from_rgb(224, 242, 251)
        }
    } else if response.hovered() {
        if dark_mode {
            Color32::from_gray(65)
        } else {
            Color32::from_gray(238)
        }
    } else {
        visuals.widgets.inactive.bg_fill
    };
    let border = if selected {
        accent
    } else if response.hovered() {
        if dark_mode {
            Color32::from_gray(145)
        } else {
            Color32::from_gray(165)
        }
    } else {
        if dark_mode {
            Color32::from_gray(78)
        } else {
            Color32::from_gray(195)
        }
    };

    let painter = ui.painter();
    painter.rect_filled(
        rect.translate(Vec2::new(0.0, 1.0)),
        0.0,
        Color32::from_black_alpha(if dark_mode { 45 } else { 28 }),
    );
    painter.rect_filled(button_rect, 0.0, fill);
    painter.rect_stroke(
        button_rect,
        0.0,
        Stroke::new(if selected { 1.5_f32 } else { 1.0_f32 }, border),
    );

    let clicked = response.clicked();
    let color = if selected {
        if dark_mode {
            Color32::from_rgb(133, 211, 248)
        } else {
            Color32::from_rgb(31, 119, 163)
        }
    } else {
        visuals.text_color()
    };
    paint_tool_icon(painter, button_rect.shrink(7.0), tool, color);
    response.on_hover_text(format!("{label} ({shortcut})"));
    clicked
}

fn paint_tool_icon(painter: &egui::Painter, rect: egui::Rect, tool: Tool, color: Color32) {
    let stroke = Stroke::new(1.8_f32, color);
    let center = rect.center();
    match tool {
        Tool::Select => {
            painter.rect_stroke(rect, 0.0, stroke);
        }
        Tool::EllipseSelect => {
            let points = (0..24)
                .map(|step| {
                    let angle = step as f32 * (std::f32::consts::TAU / 24.0);
                    egui::pos2(
                        center.x + angle.cos() * rect.width() / 2.0,
                        center.y + angle.sin() * rect.height() / 2.0,
                    )
                })
                .collect();
            painter.add(egui::Shape::closed_line(points, stroke));
        }
        Tool::LassoSelect => {
            let points = [
                rect.left_bottom(),
                rect.left_top() + Vec2::new(2.0, 1.0),
                rect.right_top() - Vec2::new(2.0, 0.0),
                rect.right_center() + Vec2::new(-1.0, 2.0),
                rect.center_bottom() + Vec2::new(1.0, -2.0),
            ];
            painter.add(egui::Shape::closed_line(points.to_vec(), stroke));
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
        Tool::Spray => {
            painter.line_segment(
                [
                    rect.left_bottom() + Vec2::new(1.0, -1.0),
                    rect.center() + Vec2::new(1.0, 1.0),
                ],
                Stroke::new(2.0_f32, color),
            );
            painter.line_segment(
                [
                    rect.center() + Vec2::new(1.0, 1.0),
                    rect.right_top() - Vec2::new(2.0, 1.0),
                ],
                stroke,
            );
            painter.circle_filled(rect.right_top() + Vec2::new(-1.0, 1.0), 1.0, color);
            painter.circle_filled(rect.right_center() + Vec2::new(2.0, 0.0), 1.0, color);
            painter.circle_filled(rect.center_top() + Vec2::new(2.0, -1.0), 1.0, color);
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
        Tool::ConnectedLines => {
            let points = [
                rect.left_bottom(),
                rect.left_top() + Vec2::new(2.0, 1.0),
                rect.right_center() + Vec2::new(-1.0, -1.0),
                rect.right_top(),
            ];
            painter.add(egui::Shape::line(points.to_vec(), stroke));
            for point in points {
                painter.circle_filled(point, 1.5, color);
            }
        }
        Tool::Rectangle => {
            painter.rect_stroke(rect, 0.0, stroke);
        }
        Tool::Circle => {
            painter.circle_stroke(center, rect.width().min(rect.height()) / 2.0, stroke);
        }
    }
}

pub fn show_palette_panel(app: &mut EframeApp, ctx: &egui::Context) {
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
                    app.ui.color_dialog_target = Some(ColorDialogTarget::Background);
                    app.ui.color_dialog_color = app.background_color;
                }
                if foreground_response.double_clicked_by(egui::PointerButton::Primary) {
                    app.ui.color_dialog_target = Some(ColorDialogTarget::Foreground);
                    app.ui.color_dialog_color = app.brush_color;
                }
                background_response.on_hover_text("Sfondo · doppio clic per scegliere un colore");
                foreground_response
                    .on_hover_text("Primo piano · doppio clic per scegliere un colore");
                ui.add_space(8.0);

                let swatch_size = 20.0;
                let columns = 15;
                let palette = app.ui.palette.clone();
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
                                    app.zoom_by(1.0 / 1.2);
                                }
                                ui.label(format!("{:.0}%", app.ui.zoom * 100.0));
                                if ui.small_button("+").on_hover_text("Aumenta zoom").clicked() {
                                    app.zoom_by(1.2);
                                }
                                if ui.small_button("Reset").clicked() {
                                    app.reset_view();
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

pub fn show_layers_panel(app: &mut EframeApp, ctx: &egui::Context) {
    update_layer_thumbnails(app, ctx);
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
                    let thumbnail = app.ui.layer_thumbnail_textures.get(idx);

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
                                    ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::click());
                                let eye_color = if is_active {
                                    selected_text
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

                                let (thumbnail_rect, _) = ui.allocate_exact_size(
                                    Vec2::splat(LAYER_THUMBNAIL_SIZE),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    thumbnail_rect,
                                    0.0,
                                    ui.visuals().extreme_bg_color,
                                );
                                ui.painter().rect_stroke(
                                    thumbnail_rect,
                                    0.0,
                                    Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
                                );
                                if let Some(texture) = thumbnail {
                                    ui.painter().image(
                                        texture.id(),
                                        thumbnail_rect.shrink(1.0),
                                        egui::Rect::from_min_max(
                                            egui::Pos2::ZERO,
                                            egui::Pos2::new(1.0, 1.0),
                                        ),
                                        Color32::WHITE,
                                    );
                                }
                                let thumbnail_response = ui.interact(
                                    thumbnail_rect,
                                    egui::Id::new(("layer_thumbnail", idx)),
                                    Sense::click(),
                                );
                                if thumbnail_response.clicked() {
                                    commands.push(Command::SetActiveLayer(idx));
                                }
                                thumbnail_response.on_hover_text("Anteprima livello");

                                let (lock_rect, lock_response) =
                                    ui.allocate_exact_size(Vec2::new(20.0, 24.0), Sense::hover());
                                paint_reserved_lock(ui, lock_rect);
                                lock_response.on_hover_text("Blocco livello (in futuro)");

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
                                    .add_sized([ui.available_width(), 48.0], name_button)
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

const LAYER_THUMBNAIL_SIZE: f32 = 48.0;

fn update_layer_thumbnails(app: &mut EframeApp, ctx: &egui::Context) {
    let revision = app.document_revision;
    if app.ui.layer_thumbnail_revision == Some(revision)
        && app.ui.layer_thumbnail_textures.len() == app.layers.len()
    {
        return;
    }

    let images: Vec<_> = app
        .layers
        .iter()
        .map(|layer| layer_thumbnail(layer, app.width, app.height))
        .collect();
    for (index, image) in images.into_iter().enumerate() {
        if let Some(texture) = app.ui.layer_thumbnail_textures.get_mut(index) {
            texture.set(image, egui::TextureOptions::NEAREST);
        } else {
            app.ui.layer_thumbnail_textures.push(ctx.load_texture(
                format!("layer_thumbnail_{index}"),
                image,
                egui::TextureOptions::NEAREST,
            ));
        }
    }
    app.ui.layer_thumbnail_textures.truncate(app.layers.len());
    app.ui.layer_thumbnail_revision = Some(revision);
}

fn layer_thumbnail(layer: &crate::model::Layer, width: usize, height: usize) -> egui::ColorImage {
    let size = LAYER_THUMBNAIL_SIZE as usize;
    let mut image = egui::ColorImage::new([size, size], Color32::from_gray(232));
    for y in 0..size {
        for x in 0..size {
            if (x / 8 + y / 8) % 2 == 1 {
                image[(x, y)] = Color32::from_gray(205);
            }
        }
    }

    if width == 0 || height == 0 || layer.pixels.len() < width.saturating_mul(height) {
        return image;
    }

    let scale = (size as f32 / width as f32).min(size as f32 / height as f32);
    let thumbnail_width = ((width as f32 * scale).round() as usize).clamp(1, size);
    let thumbnail_height = ((height as f32 * scale).round() as usize).clamp(1, size);
    let offset_x = (size - thumbnail_width) / 2;
    let offset_y = (size - thumbnail_height) / 2;

    for y in 0..thumbnail_height {
        let source_y = (y * height / thumbnail_height).min(height - 1);
        for x in 0..thumbnail_width {
            let source_x = (x * width / thumbnail_width).min(width - 1);
            let source = layer.pixels[source_y * width + source_x];
            let [red, green, blue, source_alpha] = source.to_srgba_unmultiplied();
            let alpha = source_alpha as f32 / 255.0 * layer.opacity.clamp(0.0, 1.0);
            let background = image[(offset_x + x, offset_y + y)];
            let [background_red, background_green, background_blue, _] =
                background.to_srgba_unmultiplied();
            let blend = |foreground: u8, background: u8| {
                (foreground as f32 * alpha + background as f32 * (1.0 - alpha)).round() as u8
            };
            image[(offset_x + x, offset_y + y)] = Color32::from_rgb(
                blend(red, background_red),
                blend(green, background_green),
                blend(blue, background_blue),
            );
        }
    }

    image
}

fn paint_reserved_lock(ui: &egui::Ui, rect: egui::Rect) {
    let color = ui.visuals().weak_text_color();
    let stroke = Stroke::new(1.2_f32, color);
    let body = egui::Rect::from_min_size(
        egui::pos2(rect.center().x - 5.0, rect.center().y),
        Vec2::new(10.0, 8.0),
    );
    ui.painter().rect_stroke(body, 1.0, stroke);
    let arch = [
        egui::pos2(body.left() + 2.0, body.top()),
        egui::pos2(body.left() + 2.0, body.top() - 3.0),
        egui::pos2(body.right() - 2.0, body.top() - 3.0),
        egui::pos2(body.right() - 2.0, body.top()),
    ];
    ui.painter().add(egui::Shape::line(arch.to_vec(), stroke));
}

#[cfg(test)]
mod tests {
    use super::layer_thumbnail;
    use crate::model::Layer;
    use egui::Color32;

    #[test]
    fn layer_thumbnail_fits_content_and_applies_opacity() {
        let mut layer = Layer::new("Test", 4, 2);
        layer
            .pixels
            .fill(Color32::from_rgba_unmultiplied(200, 30, 40, 128));
        layer.opacity = 0.5;

        let thumbnail = layer_thumbnail(&layer, 4, 2);

        assert_eq!(thumbnail.size, [48, 48]);
        assert_eq!(thumbnail[(24, 24)], Color32::from_rgb(224, 181, 184));
        assert_ne!(thumbnail[(24, 0)], thumbnail[(24, 24)]);
    }

    #[test]
    fn empty_layer_thumbnail_is_a_checkerboard() {
        let layer = Layer::new("Vuoto", 0, 0);

        let thumbnail = layer_thumbnail(&layer, 0, 0);

        assert_eq!(thumbnail.size, [48, 48]);
        assert_ne!(thumbnail[(0, 0)], thumbnail[(8, 0)]);
    }
}
