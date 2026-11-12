//! # canvas.rs — Area di disegno centrale
//!
//! Contiene la funzione [`show_canvas`] che gestisce tutto il pannello
//! centrale della finestra: rendering, input mouse e overlay grafici.
//!
//! ## Responsabilità
//! 1. **Pan** — trascinamento con tasto centrale/destro, Spazio+Sinistro o tool Sposta
//! 2. **Zoom** — scroll della rotellina, centrato sul puntatore
//! 3. **Scrollbar** — barre orizzontale e verticale quando la tela supera l'area visibile
//! 4. **Texture** — aggiornamento GPU tramite `app.update_texture()`
//!    (ricalcolata solo quando `dirty_canvas = true`)
//! 5. **Input mouse** — ciclo drag_started → dragged → drag_stopped
//!    per tutti gli strumenti (Select, Brush, Eraser, Pipette, Bucket,
//!    Move, Line, Rectangle, Circle)
//! 6. **Live preview** — anteprima vettoriale durante il trascinamento
//!    per selezione, linea, rettangolo ed ellisse
//! 7. **Overlay selezione** — doppio bordo azzurro Breeze + 8 maniglie
//! 8. **Toast** — notifica flottante temporanea in basso al centro

use eframe::egui;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::app::Command;
use crate::model::{PaintApp, Tool};

pub fn show_canvas(app: &mut PaintApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        const SCROLLBAR_SIZE: f32 = 14.0;
        let panel_rect = ui.available_rect_before_wrap();
        let canvas_size = Vec2::new(app.width as f32 * app.zoom, app.height as f32 * app.zoom);
        let mut show_horizontal = canvas_size.x > panel_rect.width();
        let mut show_vertical = canvas_size.y > panel_rect.height();
        for _ in 0..2 {
            let viewport_width =
                panel_rect.width() - if show_vertical { SCROLLBAR_SIZE } else { 0.0 };
            let viewport_height =
                panel_rect.height() - if show_horizontal { SCROLLBAR_SIZE } else { 0.0 };
            show_horizontal |= canvas_size.x > viewport_width;
            show_vertical |= canvas_size.y > viewport_height;
        }
        let viewport_size = Vec2::new(
            panel_rect.width() - if show_vertical { SCROLLBAR_SIZE } else { 0.0 },
            panel_rect.height() - if show_horizontal { SCROLLBAR_SIZE } else { 0.0 },
        );
        let (response, painter) = ui.allocate_painter(viewport_size, Sense::click_and_drag());
        let workspace_color = if ui.visuals().dark_mode {
            Color32::from_gray(48)
        } else {
            Color32::from_gray(224)
        };
        painter.rect_filled(response.rect, 0.0, workspace_color);

        let mut max_pan = Vec2::new(
            ((canvas_size.x - viewport_size.x) / 2.0).max(0.0),
            ((canvas_size.y - viewport_size.y) / 2.0).max(0.0),
        );
        let mut pan_x = app.pan.x;
        let mut pan_y = app.pan.y;
        if show_horizontal {
            let bar_rect = Rect::from_min_size(
                Pos2::new(panel_rect.left(), panel_rect.bottom() - SCROLLBAR_SIZE),
                Vec2::new(panel_rect.width(), SCROLLBAR_SIZE),
            );
            paint_scrollbar(
                ui,
                bar_rect,
                canvas_size.x,
                viewport_size.x,
                max_pan.x,
                &mut pan_x,
                false,
            );
        }
        if show_vertical {
            let bar_rect = Rect::from_min_size(
                Pos2::new(panel_rect.right() - SCROLLBAR_SIZE, panel_rect.top()),
                Vec2::new(SCROLLBAR_SIZE, panel_rect.height()),
            );
            paint_scrollbar(
                ui,
                bar_rect,
                canvas_size.y,
                viewport_size.y,
                max_pan.y,
                &mut pan_y,
                true,
            );
        }
        let scrollbar_pan = Vec2::new(pan_x, pan_y);
        if app.pan != scrollbar_pan {
            app.execute_command(Command::SetPan(scrollbar_pan.into()));
        }
        let clamped_pan = Vec2::new(
            app.pan.x.clamp(-max_pan.x, max_pan.x),
            app.pan.y.clamp(-max_pan.y, max_pan.y),
        );
        if app.pan != clamped_pan {
            app.execute_command(Command::SetPan(clamped_pan.into()));
        }

        // 1. Gestione Pan con drag tasto centrale/destro o Spazio + Sinistro
        let space_pressed = ctx.input(|i| i.key_down(egui::Key::Space));
        let move_selection_hit = response
            .interact_pointer_pos()
            .and_then(|pos| {
                let canvas_center = response.rect.center() + app.pan;
                let canvas_rect = Rect::from_center_size(canvas_center, canvas_size);
                app.selection.filter(|selection| {
                    canvas_rect.contains(pos)
                        && selection.contains(Pos2::new(
                            (pos.x - canvas_rect.left()) / app.zoom,
                            (pos.y - canvas_rect.top()) / app.zoom,
                        ))
                })
            })
            .is_some();
        let is_panning = response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
            || (space_pressed && response.dragged_by(egui::PointerButton::Primary))
            || (app.current_tool == Tool::Move
                && response.dragged_by(egui::PointerButton::Primary)
                && app.moving_selection.is_none()
                && !move_selection_hit);

        if is_panning {
            app.execute_command(Command::PanBy(response.drag_delta().into()));
        }

        // 2. La rotellina zooma a passi del 20%; lo scroll orizzontale sposta la vista.
        let scroll_delta = ctx.input(|i| i.raw_scroll_delta);
        if response.hovered() && (scroll_delta.x != 0.0 || scroll_delta.y != 0.0) {
            if scroll_delta.y != 0.0 {
                let old_zoom = app.zoom;
                let zoom_step = if scroll_delta.y > 0.0 { 0.2 } else { -0.2 };
                let new_zoom = (old_zoom + zoom_step).clamp(0.05, 30.0);

                if let Some(hover_pos) = response.hover_pos() {
                    let old_canvas_center = response.rect.center() + app.pan;
                    let to_pointer = hover_pos - old_canvas_center;
                    let scale_ratio = new_zoom / old_zoom;
                    app.execute_command(Command::PanBy((to_pointer * (1.0 - scale_ratio)).into()));
                }
                app.execute_command(Command::SetZoom(new_zoom));
            }
            app.execute_command(Command::PanBy(Vec2::new(scroll_delta.x, 0.0).into()));
        }
        max_pan = Vec2::new(
            ((app.width as f32 * app.zoom - viewport_size.x) / 2.0).max(0.0),
            ((app.height as f32 * app.zoom - viewport_size.y) / 2.0).max(0.0),
        );
        let clamped_pan = Vec2::new(
            app.pan.x.clamp(-max_pan.x, max_pan.x),
            app.pan.y.clamp(-max_pan.y, max_pan.y),
        );
        if app.pan != clamped_pan {
            app.execute_command(Command::SetPan(clamped_pan.into()));
        }

        // 3. Calcolo del rettangolo dello schermo occupato dalla tela
        let canvas_center = response.rect.center() + app.pan;
        let canvas_screen_size = canvas_size;
        let canvas_screen_rect = Rect::from_center_size(canvas_center, canvas_screen_size);

        // 4. Aggiornamento texture
        app.update_texture(ui);

        let texture_id = if let Some(tex) = &app.canvas_texture {
            tex.id()
        } else {
            return;
        };

        // Disegna ombra/bordo esterno tela
        painter.rect_filled(
            canvas_screen_rect.expand(1.0),
            0.0,
            Color32::from_black_alpha(90),
        );

        // Disegna l'immagine della tela
        painter.image(
            texture_id,
            canvas_screen_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );

        // Contorno tela
        painter.rect_stroke(
            canvas_screen_rect,
            0.0,
            Stroke::new(0.5_f32, Color32::from_gray(120)),
        );

        let canvas_w = app.width as f32;
        let canvas_h = app.height as f32;

        let screen_to_canvas = move |screen_pos: Pos2| -> Pos2 {
            let norm_x = (screen_pos.x - canvas_screen_rect.min.x) / canvas_screen_rect.width();
            let norm_y = (screen_pos.y - canvas_screen_rect.min.y) / canvas_screen_rect.height();
            Pos2::new(norm_x * canvas_w, norm_y * canvas_h)
        };

        let canvas_to_screen = move |canvas_pos: Pos2| -> Pos2 {
            Pos2::new(
                canvas_screen_rect.min.x + (canvas_pos.x / canvas_w) * canvas_screen_rect.width(),
                canvas_screen_rect.min.y + (canvas_pos.y / canvas_h) * canvas_screen_rect.height(),
            )
        };

        // 5. Interazione di Disegno
        if !is_panning {
            let pointer_pos_opt = response.interact_pointer_pos();

            let primary_started = response.drag_started_by(egui::PointerButton::Primary)
                || response.clicked_by(egui::PointerButton::Primary);
            let bucket_background_clicked = app.current_tool == Tool::Bucket
                && response.clicked_by(egui::PointerButton::Secondary);

            if primary_started || bucket_background_clicked {
                if let Some(pointer_pos) = pointer_pos_opt {
                    if canvas_screen_rect.contains(pointer_pos) {
                        let c_pos = screen_to_canvas(pointer_pos);

                        match app.current_tool {
                            Tool::Select => {
                                app.execute_command(Command::BeginDrawing {
                                    drag_start: Some(c_pos.into()),
                                    current: Some(c_pos.into()),
                                    last: None,
                                });
                            }
                            Tool::Move => {
                                if response.drag_started_by(egui::PointerButton::Primary) && {
                                    app.execute_command(Command::BeginSelectionMove(c_pos.into()));
                                    app.moving_selection.is_some()
                                } {
                                    app.execute_command(Command::BeginDrawing {
                                        drag_start: Some(c_pos.into()),
                                        current: Some(c_pos.into()),
                                        last: None,
                                    });
                                }
                            }
                            Tool::Pipette => {
                                app.execute_command(Command::PickColor(c_pos.into()));
                            }
                            Tool::Bucket => {
                                app.execute_command(Command::PushUndoSnapshot);
                                let fill_color =
                                    if response.clicked_by(egui::PointerButton::Secondary) {
                                        app.background_color
                                    } else {
                                        app.brush_color
                                    };
                                app.execute_command(Command::FloodFill {
                                    point: c_pos.into(),
                                    color: fill_color.into(),
                                });
                            }
                            Tool::Brush | Tool::Eraser => {
                                app.execute_command(Command::PushUndoSnapshot);
                                app.execute_command(Command::BeginDrawing {
                                    drag_start: None,
                                    current: None,
                                    last: Some(c_pos.into()),
                                });
                                app.execute_command(Command::DrawPoint(c_pos.into()));
                            }
                            Tool::Line | Tool::Rectangle | Tool::Circle => {
                                app.execute_command(Command::PushUndoSnapshot);
                                app.execute_command(Command::BeginDrawing {
                                    drag_start: Some(c_pos.into()),
                                    current: Some(c_pos.into()),
                                    last: None,
                                });
                            }
                        }
                    }
                }
            } else if response.dragged_by(egui::PointerButton::Primary) && app.is_drawing {
                if let Some(pointer_pos) = pointer_pos_opt {
                    let c_pos = screen_to_canvas(pointer_pos);

                    match app.current_tool {
                        Tool::Brush | Tool::Eraser => {
                            if let Some(last_pos) = app.last_canvas_pos {
                                app.execute_command(Command::DrawLine {
                                    from: last_pos.into(),
                                    to: c_pos.into(),
                                });
                            } else {
                                app.execute_command(Command::DrawPoint(c_pos.into()));
                            }
                            app.execute_command(Command::SetLastCanvasPosition(c_pos.into()));
                        }
                        Tool::Move => {
                            app.execute_command(Command::SetCurrentCanvasPosition(c_pos.into()));
                            if let (Some(moving), Some(start)) =
                                (&app.moving_selection, app.drag_start_canvas)
                            {
                                let delta = c_pos - start;
                                let max_x = app.width as f32 - moving.width as f32;
                                let max_y = app.height as f32 - moving.height as f32;
                                let delta = Vec2::new(
                                    (moving.x as f32 + delta.x).clamp(0.0, max_x) - moving.x as f32,
                                    (moving.y as f32 + delta.y).clamp(0.0, max_y) - moving.y as f32,
                                );
                                app.execute_command(Command::SetSelection(Some(
                                    moving.origin.translate(delta).into(),
                                )));
                            }
                        }
                        Tool::Select | Tool::Line | Tool::Rectangle | Tool::Circle => {
                            app.execute_command(Command::SetCurrentCanvasPosition(c_pos.into()));
                        }
                        _ => {}
                    }
                }
            } else if (response.drag_stopped() || (!response.dragged() && app.is_drawing))
                && app.is_drawing
            {
                match (
                    app.current_tool,
                    app.drag_start_canvas,
                    app.current_canvas_pos,
                ) {
                    (Tool::Select, Some(p0), Some(p1)) => {
                        let min_x = p0.x.min(p1.x).clamp(0.0, app.width as f32);
                        let max_x = p0.x.max(p1.x).clamp(0.0, app.width as f32);
                        let min_y = p0.y.min(p1.y).clamp(0.0, app.height as f32);
                        let max_y = p0.y.max(p1.y).clamp(0.0, app.height as f32);
                        if (max_x - min_x) > 2.0 && (max_y - min_y) > 2.0 {
                            app.execute_command(Command::SetSelection(Some(
                                Rect::from_min_max(
                                    Pos2::new(min_x, min_y),
                                    Pos2::new(max_x, max_y),
                                )
                                .into(),
                            )));
                        } else {
                            app.execute_command(Command::SetSelection(None));
                        }
                    }
                    (Tool::Move, Some(start), Some(end)) => {
                        app.execute_command(Command::FinishSelectionMove((end - start).into()));
                    }
                    (Tool::Line | Tool::Rectangle | Tool::Circle, Some(p0), Some(p1)) => {
                        app.execute_command(Command::BakeShape {
                            from: p0.into(),
                            to: p1.into(),
                        });
                    }
                    _ => {}
                }
                app.execute_command(Command::EndDrawing);
            }
        }

        // 6. Live Preview forme geometriche / selezione
        if app.is_drawing {
            if let (Some(c0), Some(c1)) = (app.drag_start_canvas, app.current_canvas_pos) {
                let s0 = canvas_to_screen(c0);
                let s1 = canvas_to_screen(c1);
                let preview_stroke_width = (app.brush_size * app.zoom).max(1.0);
                let preview_color = app.brush_color;
                let preview_fill_color = app.background_color;

                match app.current_tool {
                    Tool::Select => {
                        let rect = Rect::from_two_pos(s0, s1);
                        painter.rect_stroke(
                            rect,
                            0.0_f32,
                            Stroke::new(1.5_f32, Color32::from_rgb(61, 174, 233)),
                        );
                        painter.rect_stroke(
                            rect.expand(1.0_f32),
                            0.0_f32,
                            Stroke::new(1.0_f32, Color32::WHITE),
                        );
                    }
                    Tool::Line => {
                        painter.line_segment(
                            [s0, s1],
                            Stroke::new(preview_stroke_width, preview_color),
                        );
                    }
                    Tool::Rectangle => {
                        let rect = Rect::from_two_pos(s0, s1);
                        if app.fill_shapes {
                            painter.rect_filled(rect, 0.0, preview_fill_color);
                            painter.rect_stroke(
                                rect,
                                0.0,
                                Stroke::new(preview_stroke_width, preview_color),
                            );
                        } else {
                            painter.rect_stroke(
                                rect,
                                0.0,
                                Stroke::new(preview_stroke_width, preview_color),
                            );
                        }
                    }
                    Tool::Circle => {
                        let rect = Rect::from_two_pos(s0, s1);
                        let rx = rect.width() / 2.0;
                        let ry = rect.height() / 2.0;
                        let center = rect.center();

                        let points: Vec<Pos2> = (0..36)
                            .map(|i| {
                                let angle = (i as f32) * (std::f32::consts::TAU / 36.0);
                                Pos2::new(center.x + angle.cos() * rx, center.y + angle.sin() * ry)
                            })
                            .collect();

                        if app.fill_shapes {
                            painter.add(egui::Shape::convex_polygon(
                                points.clone(),
                                preview_fill_color,
                                Stroke::NONE,
                            ));
                            painter.add(egui::Shape::closed_line(
                                points,
                                Stroke::new(preview_stroke_width, preview_color),
                            ));
                        } else {
                            painter.add(egui::Shape::closed_line(
                                points,
                                Stroke::new(preview_stroke_width, preview_color),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        // 7. Overlay selezione attiva
        if let Some(sel) = app.selection {
            let p0 = canvas_to_screen(sel.min);
            let p1 = canvas_to_screen(sel.max);
            let sel_rect = Rect::from_two_pos(p0, p1);

            painter.rect_stroke(
                sel_rect,
                0.0_f32,
                Stroke::new(1.5_f32, Color32::from_black_alpha(220)),
            );
            painter.rect_stroke(
                sel_rect.expand(1.0_f32),
                0.0_f32,
                Stroke::new(1.2_f32, Color32::from_rgb(61, 174, 233)),
            );

            let handle_positions = [
                sel_rect.left_top(),
                sel_rect.right_top(),
                sel_rect.left_bottom(),
                sel_rect.right_bottom(),
                sel_rect.center_top(),
                sel_rect.center_bottom(),
                sel_rect.left_center(),
                sel_rect.right_center(),
            ];
            for pos in handle_positions {
                let handle_rect = Rect::from_center_size(pos, Vec2::splat(6.0));
                painter.rect_filled(handle_rect, 0.0, Color32::WHITE);
                painter.rect_stroke(handle_rect, 0.0, Stroke::new(1.0_f32, Color32::BLACK));
            }
        }

        // 8. Notifica Toast
        if let Some((msg, time)) = &app.status_message {
            if time.elapsed().as_secs_f32() < 3.0 {
                egui::Area::new(egui::Id::new("notice_toast"))
                    .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -30.0))
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style())
                            .fill(Color32::from_rgb(35, 38, 41))
                            .stroke(Stroke::new(1.5_f32, Color32::from_rgb(61, 174, 233)))
                            .rounding(0.0)
                            .inner_margin(egui::Margin::symmetric(16.0, 8.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("ℹ  {}", msg))
                                        .color(Color32::WHITE)
                                        .strong(),
                                );
                            });
                    });
                ctx.request_repaint();
            }
        }
    });
}

fn paint_scrollbar(
    ui: &mut egui::Ui,
    bar_rect: Rect,
    content_length: f32,
    viewport_length: f32,
    max_pan: f32,
    pan: &mut f32,
    vertical: bool,
) {
    let id = if vertical {
        egui::Id::new("canvas_vertical_scrollbar")
    } else {
        egui::Id::new("canvas_horizontal_scrollbar")
    };
    let response = ui.interact(bar_rect, id, Sense::click_and_drag());
    let track_length = if vertical {
        bar_rect.height()
    } else {
        bar_rect.width()
    };
    let thumb_length = (track_length * viewport_length / content_length)
        .clamp(24.0_f32.min(track_length), track_length);
    let thumb_travel = track_length - thumb_length;
    let fraction = if max_pan > 0.0 {
        ((max_pan - *pan) / (2.0 * max_pan)).clamp(0.0, 1.0)
    } else {
        0.5
    };
    let thumb_start = if vertical {
        bar_rect.top() + thumb_travel * fraction
    } else {
        bar_rect.left() + thumb_travel * fraction
    };
    let thumb_rect = if vertical {
        Rect::from_min_size(
            Pos2::new(bar_rect.center().x - 3.0, thumb_start),
            Vec2::new(6.0, thumb_length),
        )
    } else {
        Rect::from_min_size(
            Pos2::new(thumb_start, bar_rect.center().y - 3.0),
            Vec2::new(thumb_length, 6.0),
        )
    };

    let visuals = ui.style().interact(&response);
    let track_rect = if vertical {
        Rect::from_center_size(bar_rect.center(), Vec2::new(2.0, track_length))
    } else {
        Rect::from_center_size(bar_rect.center(), Vec2::new(track_length, 2.0))
    };
    ui.painter().rect_filled(track_rect, 0.0, visuals.bg_fill);
    ui.painter()
        .rect_filled(thumb_rect, 0.0, visuals.fg_stroke.color);

    if response.dragged() && thumb_travel > 0.0 {
        let delta = if vertical {
            response.drag_delta().y
        } else {
            response.drag_delta().x
        };
        *pan = (*pan - delta * (2.0 * max_pan / thumb_travel)).clamp(-max_pan, max_pan);
    } else if response.clicked() && thumb_travel > 0.0 {
        if let Some(pointer) = response.interact_pointer_pos() {
            let pointer_position = if vertical { pointer.y } else { pointer.x };
            let bar_start = if vertical {
                bar_rect.top()
            } else {
                bar_rect.left()
            };
            let fraction = ((pointer_position - bar_start - thumb_length / 2.0) / thumb_travel)
                .clamp(0.0, 1.0);
            *pan = max_pan - fraction * 2.0 * max_pan;
        }
    }
}
