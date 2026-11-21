//! Application commands shared by the GUI and future non-GUI adapters.
//!
//! Adapters translate their input into [`Command`] values and execute them
//! against the same application state through [`CommandEngine`].

use egui::{Color32, Pos2, Rect, Vec2};

use crate::model::{PaintApp, SelectionTool, Tool};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasVector {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasRect {
    pub min: CanvasPoint,
    pub max: CanvasPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbaColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl From<Pos2> for CanvasPoint {
    fn from(point: Pos2) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<CanvasPoint> for Pos2 {
    fn from(point: CanvasPoint) -> Self {
        Self::new(point.x, point.y)
    }
}

impl From<Vec2> for CanvasVector {
    fn from(vector: Vec2) -> Self {
        Self {
            x: vector.x,
            y: vector.y,
        }
    }
}

impl From<Color32> for RgbaColor {
    fn from(color: Color32) -> Self {
        let [red, green, blue, alpha] = color.to_srgba_unmultiplied();
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl From<Rect> for CanvasRect {
    fn from(rect: Rect) -> Self {
        Self {
            min: rect.min.into(),
            max: rect.max.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Undo,
    Redo,
    PushUndoSnapshot,
    NewCanvas {
        width: usize,
        height: usize,
    },
    ResizeCanvas {
        width: usize,
        height: usize,
    },
    SelectAll,
    SetSelection(Option<CanvasRect>),
    BeginSelection(CanvasPoint),
    UpdateSelection(CanvasPoint),
    FinishSelection(CanvasPoint),
    Deselect,
    CopySelection,
    CutSelection,
    DeleteSelection,
    Paste,
    PasteFromSystemClipboard,
    PasteInNewWindow,
    CropToSelection,
    FlipHorizontal,
    FlipVertical,
    RotateClockwise,
    RotateCounterClockwise,
    RotateImage {
        angle_degrees: u16,
        clockwise: bool,
    },
    InvertColors,
    ClearActiveLayer,
    CreateLayer,
    DuplicateActiveLayer,
    MergeLayersDown(usize),
    DeleteLayer(usize),
    MoveLayerUp(usize),
    MoveLayerDown(usize),
    SetActiveLayer(usize),
    SetLayerVisible {
        index: usize,
        visible: bool,
    },
    SetLayerOpacity {
        index: usize,
        opacity: f32,
    },
    SetTool(Tool),
    SetBrushSize(f32),
    SetFillShapes(bool),
    DrawPoint(CanvasPoint),
    SprayPoint(CanvasPoint),
    DrawLine {
        from: CanvasPoint,
        to: CanvasPoint,
    },
    DrawConnectedLines(Vec<CanvasPoint>),
    FloodFill {
        point: CanvasPoint,
        color: RgbaColor,
    },
    PickColor(CanvasPoint),
    BakeShape {
        from: CanvasPoint,
        to: CanvasPoint,
    },
    BeginSelectionMove(CanvasPoint),
    FinishSelectionMove(CanvasVector),
    SetForegroundColor(RgbaColor),
    SetBackgroundColor(RgbaColor),
    SavePng(String),
    SaveJpeg(String),
    SaveBmp(String),
    SaveWebP(String),
    SaveOpenRaster(String),
    LoadPng(String),
    LoadOpenRaster(String),
    SavePdf(String),
    CopySelectionToFile(String),
    PasteFromFile(String),
}

/// Dispatches user intent to the existing document, history, tool and file
/// operations. The engine is deliberately independent of egui widgets.
#[derive(Default)]
pub struct CommandEngine;

impl CommandEngine {
    pub fn execute(&mut self, app: &mut PaintApp, command: Command) -> Vec<String> {
        let mut notices = Vec::new();
        match command {
            Command::Undo => app.undo(),
            Command::Redo => app.redo(),
            Command::PushUndoSnapshot => app.push_undo_snapshot(),
            Command::NewCanvas { width, height } => app.new_canvas(width, height, &mut notices),
            Command::ResizeCanvas { width, height } => {
                app.resize_canvas(width, height, &mut notices)
            }
            Command::SelectAll => app.select_all(),
            Command::SetSelection(selection) => {
                let selection = selection.map(|rect| {
                    crate::model::SelectionRegion::Rectangle(Rect::from_min_max(
                        rect.min.into(),
                        rect.max.into(),
                    ))
                });
                if app.selection != selection {
                    app.push_undo_snapshot();
                    app.selection = selection;
                    app.mark_document_changed();
                }
            }
            Command::BeginSelection(point) => {
                if let Some(tool) = SelectionTool::from_tool(app.current_tool) {
                    app.begin_selection_gesture(tool, point.into());
                }
            }
            Command::UpdateSelection(point) => app.update_selection_gesture(point.into()),
            Command::FinishSelection(point) => app.finish_selection_gesture(point.into()),
            Command::Deselect => app.deselect(),
            Command::CopySelection => app.copy_selection(&mut notices),
            Command::CutSelection => app.cut_selection(&mut notices),
            Command::DeleteSelection => app.delete_selection(),
            Command::Paste => app.paste(&mut notices),
            Command::PasteFromSystemClipboard => app.paste_from_system_clipboard(&mut notices),
            Command::PasteInNewWindow => app.paste_in_new_window(&mut notices),
            Command::CropToSelection => app.crop_to_selection(),
            Command::FlipHorizontal => app.flip_horizontal(),
            Command::FlipVertical => app.flip_vertical(),
            Command::RotateClockwise => app.rotate_clockwise(),
            Command::RotateCounterClockwise => app.rotate_counter_clockwise(),
            Command::RotateImage {
                angle_degrees,
                clockwise,
            } => app.rotate_image(angle_degrees, clockwise, &mut notices),
            Command::InvertColors => app.invert_colors(),
            Command::ClearActiveLayer => {
                if app.active_layer_idx < app.layers.len() {
                    app.push_undo_snapshot();
                    app.layers[app.active_layer_idx].clear(Color32::TRANSPARENT);
                    app.mark_document_changed();
                }
            }
            Command::CreateLayer => {
                app.push_undo_snapshot();
                let name = format!("Livello {}", app.layers.len());
                app.layers
                    .push(crate::model::Layer::new(&name, app.width, app.height));
                app.active_layer_idx = app.layers.len() - 1;
                app.mark_document_changed();
            }
            Command::DuplicateActiveLayer => {
                if let Some(active) = app.layers.get(app.active_layer_idx).cloned() {
                    app.push_undo_snapshot();
                    let mut duplicate = active;
                    duplicate.name = format!("{} (Copia)", duplicate.name);
                    app.layers.insert(app.active_layer_idx + 1, duplicate);
                    app.active_layer_idx += 1;
                    app.mark_document_changed();
                }
            }
            Command::MergeLayersDown(index) => app.merge_layers_down(index, &mut notices),
            Command::DeleteLayer(index) => {
                if app.layers.len() > 1 && index < app.layers.len() {
                    app.push_undo_snapshot();
                    app.layers.remove(index);
                    if app.active_layer_idx >= app.layers.len() {
                        app.active_layer_idx = app.layers.len() - 1;
                    }
                    app.mark_document_changed();
                }
            }
            Command::MoveLayerUp(index) => {
                if index + 1 < app.layers.len() {
                    app.push_undo_snapshot();
                    app.layers.swap(index, index + 1);
                    app.active_layer_idx = index + 1;
                    app.mark_document_changed();
                }
            }
            Command::MoveLayerDown(index) => {
                if index > 0 && index < app.layers.len() {
                    app.push_undo_snapshot();
                    app.layers.swap(index, index - 1);
                    app.active_layer_idx = index - 1;
                    app.mark_document_changed();
                }
            }
            Command::SetActiveLayer(index) => {
                if index < app.layers.len() {
                    app.active_layer_idx = index;
                }
            }
            Command::SetLayerVisible { index, visible } => {
                if let Some(layer) = app.layers.get_mut(index) {
                    layer.visible = visible;
                    app.mark_document_changed();
                }
            }
            Command::SetLayerOpacity { index, opacity } => {
                if let Some(layer) = app.layers.get_mut(index) {
                    layer.opacity = opacity.clamp(0.0, 1.0);
                    app.mark_document_changed();
                }
            }
            Command::SetTool(tool) => {
                app.current_tool = tool;
                app.selection_tools.gesture = None;
            }
            Command::SetBrushSize(size) => app.brush_size = size.clamp(1.0, 80.0),
            Command::SetFillShapes(fill) => app.fill_shapes = fill,
            Command::SetForegroundColor(color) => {
                app.brush_color =
                    Color32::from_rgba_unmultiplied(color.red, color.green, color.blue, color.alpha)
            }
            Command::SetBackgroundColor(color) => {
                app.background_color =
                    Color32::from_rgba_unmultiplied(color.red, color.green, color.blue, color.alpha)
            }
            Command::DrawPoint(point) => app.draw_point_on_active(point.x, point.y),
            Command::SprayPoint(point) => app.spray_point_on_active(point.x, point.y),
            Command::DrawLine { from, to } => app.draw_continuous_line(from.into(), to.into()),
            Command::DrawConnectedLines(points) => {
                if points.len() >= 2 {
                    app.push_undo_snapshot();
                    for pair in points.windows(2) {
                        app.draw_continuous_line(pair[0].into(), pair[1].into());
                    }
                }
            }
            Command::FloodFill { point, color } => app.flood_fill(
                point.x as i32,
                point.y as i32,
                Color32::from_rgba_unmultiplied(color.red, color.green, color.blue, color.alpha),
            ),
            Command::PickColor(point) => app.pick_color(point.x as i32, point.y as i32),
            Command::BakeShape { from, to } => app.bake_shape(from.into(), to.into()),
            Command::BeginSelectionMove(position) => {
                app.begin_selection_move(position.into());
            }
            Command::FinishSelectionMove(delta) => {
                app.finish_selection_move(Vec2::new(delta.x, delta.y))
            }
            Command::SavePng(path) => app.save_to_png(&path, &mut notices),
            Command::SaveJpeg(path) => app.save_to_jpeg(&path, &mut notices),
            Command::SaveBmp(path) => app.save_to_bmp(&path, &mut notices),
            Command::SaveWebP(path) => app.save_to_webp(&path, &mut notices),
            Command::SaveOpenRaster(path) => app.save_to_openraster(&path, &mut notices),
            Command::LoadPng(path) => app.load_from_png(&path, &mut notices),
            Command::LoadOpenRaster(path) => app.load_from_openraster(&path, &mut notices),
            Command::SavePdf(path) => app.save_to_pdf(&path, &mut notices),
            Command::CopySelectionToFile(path) => app.copy_to_file(&path, &mut notices),
            Command::PasteFromFile(path) => app.paste_from_file(&path, &mut notices),
        }
        notices
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, CommandEngine};
    use crate::model::{PaintApp, Tool};
    use egui::Color32;

    #[test]
    fn headless_state_accepts_commands_without_an_egui_window() {
        let mut app = PaintApp::new_headless();
        let mut engine = CommandEngine;

        engine.execute(&mut app, Command::SetTool(Tool::Eraser));
        engine.execute(&mut app, Command::CreateLayer);
        engine.execute(&mut app, Command::SetBrushSize(20.0));

        assert_eq!(app.current_tool, Tool::Eraser);
        assert_eq!(app.layers.len(), 3);
        assert_eq!(app.active_layer_idx, 2);
        assert_eq!(app.brush_size, 20.0);
    }

    #[test]
    fn invalid_canvas_command_returns_a_notice_without_changing_revision() {
        let mut app = PaintApp::new_headless();
        let notices = CommandEngine.execute(
            &mut app,
            Command::NewCanvas {
                width: 0,
                height: 100,
            },
        );

        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("Dimensioni immagine non valide"));
        assert_eq!(app.document_revision, 0);
    }

    #[test]
    fn raster_edits_and_undo_advance_the_document_revision() {
        let mut app = PaintApp::new_headless();
        let mut engine = CommandEngine;
        engine.execute(&mut app, Command::PushUndoSnapshot);
        engine.execute(
            &mut app,
            Command::DrawPoint(super::CanvasPoint { x: 8.0, y: 8.0 }),
        );
        let drawn_revision = app.document_revision;

        engine.execute(&mut app, Command::Undo);

        assert!(drawn_revision > 0);
        assert!(app.document_revision > drawn_revision);
    }

    #[test]
    fn rgba_command_colors_preserve_unmultiplied_alpha_channels() {
        let color = Color32::from_rgba_unmultiplied(128, 64, 32, 128);
        let command_color = super::RgbaColor::from(color);
        let restored = Color32::from_rgba_unmultiplied(
            command_color.red,
            command_color.green,
            command_color.blue,
            command_color.alpha,
        );

        assert_eq!(restored, color);
    }

    #[test]
    fn spray_command_adds_droplets_to_the_active_layer() {
        let mut app = PaintApp::new_headless();
        app.width = 128;
        app.height = 128;
        app.layers[1].pixels = vec![Color32::TRANSPARENT; 128 * 128];
        app.brush_size = 40.0;

        CommandEngine.execute(
            &mut app,
            Command::SprayPoint(super::CanvasPoint { x: 64.0, y: 64.0 }),
        );

        let sprayed_pixels: Vec<_> = app.layers[1]
            .pixels
            .iter()
            .enumerate()
            .filter(|(_, pixel)| **pixel == app.brush_color)
            .map(|(index, _)| (index % app.width, index / app.width))
            .collect();
        assert!(sprayed_pixels.len() > 10);
        assert!(sprayed_pixels.iter().any(|(x, y)| {
            let dx = *x as isize - 64;
            let dy = *y as isize - 64;
            dx * dx + dy * dy > 10 * 10
        }));
    }

    #[test]
    fn connected_lines_draw_a_polyline_and_undo_as_one_action() {
        let mut app = PaintApp::new_headless();
        app.width = 16;
        app.height = 16;
        app.layers[1].pixels = vec![Color32::TRANSPARENT; 16 * 16];
        let mut engine = CommandEngine;

        engine.execute(
            &mut app,
            Command::DrawConnectedLines(vec![
                super::CanvasPoint { x: 2.0, y: 2.0 },
                super::CanvasPoint { x: 10.0, y: 2.0 },
                super::CanvasPoint { x: 10.0, y: 10.0 },
            ]),
        );

        assert_eq!(app.layers[1].pixels[2 * 16 + 6], app.brush_color);
        assert_eq!(app.layers[1].pixels[6 * 16 + 10], app.brush_color);
        engine.execute(&mut app, Command::Undo);
        assert!(app.layers[1]
            .pixels
            .iter()
            .all(|pixel| *pixel == Color32::TRANSPARENT));
    }

    #[test]
    fn selection_commands_create_ellipse_and_lasso_regions_and_history_restores_them() {
        let mut app = PaintApp::new_headless();
        let mut engine = CommandEngine;
        engine.execute(&mut app, Command::SetTool(Tool::Select));
        engine.execute(
            &mut app,
            Command::BeginSelection(super::CanvasPoint { x: 1.0, y: 1.0 }),
        );
        engine.execute(
            &mut app,
            Command::FinishSelection(super::CanvasPoint { x: 10.0, y: 10.0 }),
        );
        assert!(matches!(
            app.selection,
            Some(crate::model::SelectionRegion::Rectangle(_))
        ));

        engine.execute(&mut app, Command::SetTool(Tool::EllipseSelect));
        engine.execute(
            &mut app,
            Command::BeginSelection(super::CanvasPoint { x: 2.0, y: 3.0 }),
        );
        engine.execute(
            &mut app,
            Command::FinishSelection(super::CanvasPoint { x: 12.0, y: 13.0 }),
        );
        assert!(matches!(
            app.selection,
            Some(crate::model::SelectionRegion::Ellipse(_))
        ));

        engine.execute(&mut app, Command::Undo);
        assert!(matches!(
            app.selection,
            Some(crate::model::SelectionRegion::Rectangle(_))
        ));
        engine.execute(&mut app, Command::Redo);
        assert!(matches!(
            app.selection,
            Some(crate::model::SelectionRegion::Ellipse(_))
        ));

        engine.execute(&mut app, Command::SetTool(Tool::LassoSelect));
        engine.execute(
            &mut app,
            Command::BeginSelection(super::CanvasPoint { x: 20.0, y: 20.0 }),
        );
        engine.execute(
            &mut app,
            Command::UpdateSelection(super::CanvasPoint { x: 30.0, y: 20.0 }),
        );
        engine.execute(
            &mut app,
            Command::UpdateSelection(super::CanvasPoint { x: 30.0, y: 30.0 }),
        );
        engine.execute(
            &mut app,
            Command::FinishSelection(super::CanvasPoint { x: 20.0, y: 30.0 }),
        );
        let Some(crate::model::SelectionRegion::Polygon(points)) = app.selection.as_ref() else {
            panic!("lasso should produce a polygon selection");
        };
        assert_eq!(points.len(), 5);
        assert_eq!(points.first(), points.last());
        assert!(app.selection_contains(egui::pos2(25.0, 25.0)));
        assert!(!app.selection_contains(egui::pos2(35.0, 25.0)));
        engine.execute(&mut app, Command::Undo);
        assert!(matches!(
            app.selection,
            Some(crate::model::SelectionRegion::Ellipse(_))
        ));
        engine.execute(&mut app, Command::Redo);
        assert!(matches!(
            app.selection,
            Some(crate::model::SelectionRegion::Polygon(_))
        ));
    }
}
