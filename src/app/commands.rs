//! Application commands shared by the GUI and future non-GUI adapters.
//!
//! Adapters translate their input into [`Command`] values and execute them
//! against the same application state through [`CommandEngine`].

use egui::{Color32, Pos2, Rect, Vec2};

use crate::model::{Appearance, PaintApp, Tool};

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
    SetAppearance(Appearance),
    SetZoom(f32),
    ZoomBy(f32),
    ResetView,
    SetPan(CanvasVector),
    PanBy(CanvasVector),
    BeginDrawing {
        drag_start: Option<CanvasPoint>,
        current: Option<CanvasPoint>,
        last: Option<CanvasPoint>,
    },
    SetCurrentCanvasPosition(CanvasPoint),
    SetLastCanvasPosition(CanvasPoint),
    EndDrawing,
    DrawPoint(CanvasPoint),
    DrawLine {
        from: CanvasPoint,
        to: CanvasPoint,
    },
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
    LoadPng(String),
    SavePdf(String),
    CopySelectionToFile(String),
    PasteFromFile(String),
}

/// Dispatches user intent to the existing document, history, tool and file
/// operations. The engine is deliberately independent of egui widgets.
#[derive(Default)]
pub struct CommandEngine;

impl CommandEngine {
    pub fn execute(&mut self, app: &mut PaintApp, command: Command) {
        match command {
            Command::Undo => app.undo(),
            Command::Redo => app.redo(),
            Command::PushUndoSnapshot => app.push_undo_snapshot(),
            Command::NewCanvas { width, height } => app.new_canvas(width, height),
            Command::ResizeCanvas { width, height } => app.resize_canvas(width, height),
            Command::SelectAll => app.select_all(),
            Command::SetSelection(selection) => {
                app.selection =
                    selection.map(|rect| Rect::from_min_max(rect.min.into(), rect.max.into()));
            }
            Command::Deselect => app.deselect(),
            Command::CopySelection => app.copy_selection(),
            Command::CutSelection => app.cut_selection(),
            Command::DeleteSelection => app.delete_selection(),
            Command::Paste => app.paste(),
            Command::PasteFromSystemClipboard => app.paste_from_system_clipboard(),
            Command::PasteInNewWindow => app.paste_in_new_window(),
            Command::CropToSelection => app.crop_to_selection(),
            Command::FlipHorizontal => app.flip_horizontal(),
            Command::FlipVertical => app.flip_vertical(),
            Command::InvertColors => app.invert_colors(),
            Command::ClearActiveLayer => {
                if app.active_layer_idx < app.layers.len() {
                    app.push_undo_snapshot();
                    app.layers[app.active_layer_idx].clear(Color32::TRANSPARENT);
                    app.dirty_canvas = true;
                }
            }
            Command::CreateLayer => {
                app.push_undo_snapshot();
                let name = format!("Livello {}", app.layers.len());
                app.layers
                    .push(crate::model::Layer::new(&name, app.width, app.height));
                app.active_layer_idx = app.layers.len() - 1;
                app.dirty_canvas = true;
            }
            Command::DuplicateActiveLayer => {
                if let Some(active) = app.layers.get(app.active_layer_idx).cloned() {
                    app.push_undo_snapshot();
                    let mut duplicate = active;
                    duplicate.name = format!("{} (Copia)", duplicate.name);
                    app.layers.insert(app.active_layer_idx + 1, duplicate);
                    app.active_layer_idx += 1;
                    app.dirty_canvas = true;
                }
            }
            Command::MergeLayersDown(index) => app.merge_layers_down(index),
            Command::DeleteLayer(index) => {
                if app.layers.len() > 1 && index < app.layers.len() {
                    app.push_undo_snapshot();
                    app.layers.remove(index);
                    if app.active_layer_idx >= app.layers.len() {
                        app.active_layer_idx = app.layers.len() - 1;
                    }
                    app.dirty_canvas = true;
                }
            }
            Command::MoveLayerUp(index) => {
                if index + 1 < app.layers.len() {
                    app.push_undo_snapshot();
                    app.layers.swap(index, index + 1);
                    app.active_layer_idx = index + 1;
                    app.dirty_canvas = true;
                }
            }
            Command::MoveLayerDown(index) => {
                if index > 0 && index < app.layers.len() {
                    app.push_undo_snapshot();
                    app.layers.swap(index, index - 1);
                    app.active_layer_idx = index - 1;
                    app.dirty_canvas = true;
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
                    app.dirty_canvas = true;
                }
            }
            Command::SetLayerOpacity { index, opacity } => {
                if let Some(layer) = app.layers.get_mut(index) {
                    layer.opacity = opacity.clamp(0.0, 1.0);
                    app.dirty_canvas = true;
                }
            }
            Command::SetTool(tool) => app.current_tool = tool,
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
            Command::SetAppearance(appearance) => app.appearance = appearance,
            Command::SetZoom(zoom) => app.zoom = zoom.clamp(0.05, 30.0),
            Command::ZoomBy(factor) => app.zoom = (app.zoom * factor).clamp(0.05, 30.0),
            Command::ResetView => {
                app.zoom = 1.0;
                app.pan = Vec2::ZERO;
            }
            Command::SetPan(pan) => app.pan = Vec2::new(pan.x, pan.y),
            Command::PanBy(delta) => app.pan += Vec2::new(delta.x, delta.y),
            Command::BeginDrawing {
                drag_start,
                current,
                last,
            } => {
                app.is_drawing = true;
                app.drag_start_canvas = drag_start.map(Into::into);
                app.current_canvas_pos = current.map(Into::into);
                app.last_canvas_pos = last.map(Into::into);
            }
            Command::SetCurrentCanvasPosition(position) => {
                app.current_canvas_pos = Some(position.into());
            }
            Command::SetLastCanvasPosition(position) => {
                app.last_canvas_pos = Some(position.into());
            }
            Command::EndDrawing => {
                app.is_drawing = false;
                app.drag_start_canvas = None;
                app.last_canvas_pos = None;
                app.current_canvas_pos = None;
            }
            Command::DrawPoint(point) => app.draw_point_on_active(point.x, point.y),
            Command::DrawLine { from, to } => app.draw_continuous_line(from.into(), to.into()),
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
            Command::SavePng(path) => app.save_to_png(&path),
            Command::LoadPng(path) => app.load_from_png(&path),
            Command::SavePdf(path) => app.save_to_pdf(&path),
            Command::CopySelectionToFile(path) => app.copy_to_file(&path),
            Command::PasteFromFile(path) => app.paste_from_file(&path),
        }
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

        assert_eq!(app.current_tool, Tool::Eraser);
        assert_eq!(app.layers.len(), 3);
        assert_eq!(app.active_layer_idx, 2);
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
}
