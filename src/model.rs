//! # model.rs — Strutture dati dell'applicazione
//!
//! Questo file definisce i dati condivisi del documento e dell'applicazione.
//!
//! ## Contenuto
//! - [`Layer`]         — Un livello pittura (nome, visibilità, opacità, pixel)
//! - [`Tool`]          — Enumerazione degli strumenti disponibili (pennello, gomma, …)
//! - [`ClipboardData`] — Buffer degli appunti (pixel copiati/tagliati)
//! - [`HistoryState`]  — Snapshot dello stato per undo/redo
//! - [`PaintApp`]      — Stato del documento, strumenti e cronologia

use egui::{Color32, Pos2, Rect, Vec2};
use std::collections::VecDeque;

// ============================================================================
// 1. MODELLO DATI PER I LIVELLI
// ============================================================================

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub opacity: f32, // 0.0 ..= 1.0
    pub pixels: Vec<Color32>,
}

impl Layer {
    pub fn new(name: &str, width: usize, height: usize) -> Self {
        Self {
            name: name.to_string(),
            visible: true,
            opacity: 1.0,
            pixels: vec![Color32::TRANSPARENT; width * height],
        }
    }

    pub fn clear(&mut self, color: Color32) {
        self.pixels.fill(color);
    }
}

// ============================================================================
// 2. STRUMENTI DISPONIBILI
// ============================================================================

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Tool {
    Select,         // Selezione rettangolare (S)
    EllipseSelect,  // Selezione ellittica
    LassoSelect,    // Selezione libera
    Move,           // Sposta tela o selezione (M)
    Brush,          // Pennello a tratto continuo (B)
    Spray,          // Aerografo / spray
    Eraser,         // Gomma da cancellare (E)
    Pipette,        // Contagocce / Color picker (I)
    Bucket,         // Secchiello / Riempimento flood-fill (G)
    Line,           // Linea retta (L)
    ConnectedLines, // Linee collegate / polilinea
    Rectangle,      // Rettangolo (R)
    Circle,         // Cerchio / Ellisse (C)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionTool {
    Rectangular,
    Elliptical,
    FreeForm,
}

impl SelectionTool {
    pub fn from_tool(tool: Tool) -> Option<Self> {
        match tool {
            Tool::Select => Some(Self::Rectangular),
            Tool::EllipseSelect => Some(Self::Elliptical),
            Tool::LassoSelect => Some(Self::FreeForm),
            _ => None,
        }
    }

    fn create_region(self, start: Pos2, end: Pos2, points: Vec<Pos2>) -> Option<SelectionRegion> {
        let rect = Rect::from_two_pos(start, end);
        match self {
            Self::Rectangular if rect.width() > 2.0 && rect.height() > 2.0 => {
                Some(SelectionRegion::Rectangle(rect))
            }
            Self::Elliptical if rect.width() > 2.0 && rect.height() > 2.0 => {
                Some(SelectionRegion::Ellipse(rect))
            }
            Self::FreeForm => {
                let mut points = points;
                if points.last().is_none_or(|last| *last != end) {
                    points.push(end);
                }
                if points
                    .first()
                    .is_some_and(|first| points.last() != Some(first))
                {
                    points.push(start);
                }
                let region = SelectionRegion::Polygon(points);
                region.is_valid().then_some(region)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectionRegion {
    Rectangle(Rect),
    Ellipse(Rect),
    Polygon(Vec<Pos2>),
}

impl SelectionRegion {
    pub fn bounds(&self) -> Rect {
        match self {
            Self::Rectangle(rect) | Self::Ellipse(rect) => Rect::from_two_pos(rect.min, rect.max),
            Self::Polygon(points) => {
                if points.is_empty() {
                    return Rect::from_min_max(Pos2::ZERO, Pos2::ZERO);
                }
                let min = Pos2::new(
                    points
                        .iter()
                        .map(|point| point.x)
                        .fold(f32::INFINITY, f32::min),
                    points
                        .iter()
                        .map(|point| point.y)
                        .fold(f32::INFINITY, f32::min),
                );
                let max = Pos2::new(
                    points
                        .iter()
                        .map(|point| point.x)
                        .fold(f32::NEG_INFINITY, f32::max),
                    points
                        .iter()
                        .map(|point| point.y)
                        .fold(f32::NEG_INFINITY, f32::max),
                );
                Rect::from_min_max(min, max)
            }
        }
    }

    pub fn contains(&self, point: Pos2) -> bool {
        match self {
            Self::Rectangle(rect) => Rect::from_two_pos(rect.min, rect.max).contains(point),
            Self::Ellipse(rect) => {
                let rect = Rect::from_two_pos(rect.min, rect.max);
                if rect.width() <= 0.0 || rect.height() <= 0.0 {
                    return false;
                }
                let normalized_x = (point.x - rect.center().x) / (rect.width() / 2.0);
                let normalized_y = (point.y - rect.center().y) / (rect.height() / 2.0);
                normalized_x * normalized_x + normalized_y * normalized_y <= 1.0
            }
            Self::Polygon(points) => point_in_polygon(point, points),
        }
    }

    pub fn contains_pixel(&self, x: usize, y: usize) -> bool {
        let pixel = Rect::from_min_size(Pos2::new(x as f32, y as f32), Vec2::splat(1.0));
        match self {
            Self::Rectangle(_) => self.bounds().intersects(pixel),
            _ => self.contains(Pos2::new(x as f32 + 0.5, y as f32 + 0.5)),
        }
    }

    pub fn is_valid(&self) -> bool {
        let bounds = self.bounds();
        if bounds.width() < 1.0 || bounds.height() < 1.0 {
            return false;
        }
        match self {
            Self::Polygon(points) => {
                points.len() >= 4
                    && points.windows(3).any(|triple| {
                        let first = triple[0];
                        let second = triple[1];
                        let third = triple[2];
                        let cross = (second.x - first.x) * (third.y - first.y)
                            - (second.y - first.y) * (third.x - first.x);
                        cross.abs() > f32::EPSILON
                    })
            }
            _ => true,
        }
    }

    pub fn translated(&self, delta: Vec2) -> Self {
        match self {
            Self::Rectangle(rect) => Self::Rectangle(rect.translate(delta)),
            Self::Ellipse(rect) => Self::Ellipse(rect.translate(delta)),
            Self::Polygon(points) => {
                Self::Polygon(points.iter().map(|point| *point + delta).collect())
            }
        }
    }

    pub fn scaled(&self, scale_x: f32, scale_y: f32) -> Self {
        let scale_point = |point: Pos2| Pos2::new(point.x * scale_x, point.y * scale_y);
        match self {
            Self::Rectangle(rect) => Self::Rectangle(Rect::from_min_max(
                scale_point(rect.min),
                scale_point(rect.max),
            )),
            Self::Ellipse(rect) => Self::Ellipse(Rect::from_min_max(
                scale_point(rect.min),
                scale_point(rect.max),
            )),
            Self::Polygon(points) => {
                Self::Polygon(points.iter().map(|point| scale_point(*point)).collect())
            }
        }
    }

    pub fn rotated(
        &self,
        width: usize,
        height: usize,
        rotated_width: usize,
        rotated_height: usize,
        angle_degrees: u16,
    ) -> Self {
        let transform = |point: Pos2| match angle_degrees % 360 {
            0 => point,
            90 => Pos2::new(rotated_width as f32 - point.y, point.x),
            180 => Pos2::new(width as f32 - point.x, height as f32 - point.y),
            270 => Pos2::new(point.y, rotated_height as f32 - point.x),
            _ => {
                let radians = f64::from(angle_degrees).to_radians();
                let (sin, cos) = radians.sin_cos();
                let x = point.x as f64 - width as f64 / 2.0;
                let y = point.y as f64 - height as f64 / 2.0;
                Pos2::new(
                    (cos * x - sin * y + rotated_width as f64 / 2.0) as f32,
                    (sin * x + cos * y + rotated_height as f64 / 2.0) as f32,
                )
            }
        };
        let points = match self {
            Self::Rectangle(_) | Self::Ellipse(_) => {
                if matches!(self, Self::Ellipse(_)) && angle_degrees % 90 != 0 {
                    let rect = self.bounds();
                    (0..64)
                        .map(|step| {
                            let angle = step as f32 * (std::f32::consts::TAU / 64.0);
                            Pos2::new(
                                rect.center().x + angle.cos() * rect.width() / 2.0,
                                rect.center().y + angle.sin() * rect.height() / 2.0,
                            )
                        })
                        .collect()
                } else {
                    let rect = self.bounds();
                    vec![
                        rect.left_top(),
                        rect.right_top(),
                        rect.right_bottom(),
                        rect.left_bottom(),
                    ]
                }
            }
            Self::Polygon(points) => points.clone(),
        }
        .into_iter()
        .map(transform)
        .collect::<Vec<_>>();

        match self {
            Self::Rectangle(_) | Self::Ellipse(_) if angle_degrees % 90 == 0 => {
                let bounds = bounds_of_points(&points);
                if matches!(self, Self::Rectangle(_)) {
                    Self::Rectangle(bounds)
                } else {
                    Self::Ellipse(bounds)
                }
            }
            _ => Self::Polygon(points),
        }
    }
}

fn bounds_of_points(points: &[Pos2]) -> Rect {
    Rect::from_min_max(
        Pos2::new(
            points
                .iter()
                .map(|point| point.x)
                .fold(f32::INFINITY, f32::min),
            points
                .iter()
                .map(|point| point.y)
                .fold(f32::INFINITY, f32::min),
        ),
        Pos2::new(
            points
                .iter()
                .map(|point| point.x)
                .fold(f32::NEG_INFINITY, f32::max),
            points
                .iter()
                .map(|point| point.y)
                .fold(f32::NEG_INFINITY, f32::max),
        ),
    )
}

fn point_in_polygon(point: Pos2, points: &[Pos2]) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = points.len() - 1;
    for current in 0..points.len() {
        let a = points[current];
        let b = points[previous];
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

#[derive(Clone, Debug)]
pub struct SelectionGesture {
    pub tool: SelectionTool,
    pub start: Pos2,
    pub points: Vec<Pos2>,
}

#[derive(Clone, Debug, Default)]
pub struct SelectionToolManager {
    pub gesture: Option<SelectionGesture>,
}

impl SelectionToolManager {
    pub fn begin(&mut self, tool: SelectionTool, point: Pos2) {
        self.gesture = Some(SelectionGesture {
            tool,
            start: point,
            points: vec![point],
        });
    }

    pub fn update(&mut self, point: Pos2) {
        if let Some(gesture) = &mut self.gesture {
            if gesture.tool == SelectionTool::FreeForm
                && gesture
                    .points
                    .last()
                    .is_none_or(|last| last.distance(point) >= 1.0)
            {
                gesture.points.push(point);
            }
        }
    }

    pub fn preview(&self, point: Pos2) -> Option<SelectionRegion> {
        let gesture = self.gesture.as_ref()?;
        if gesture.tool == SelectionTool::FreeForm {
            let mut points = gesture.points.clone();
            if points.last().is_none_or(|last| *last != point) {
                points.push(point);
            }
            Some(SelectionRegion::Polygon(points))
        } else {
            let rect = Rect::from_two_pos(gesture.start, point);
            Some(match gesture.tool {
                SelectionTool::Rectangular => SelectionRegion::Rectangle(rect),
                SelectionTool::Elliptical => SelectionRegion::Ellipse(rect),
                SelectionTool::FreeForm => unreachable!(),
            })
        }
    }

    pub fn finish(&mut self, point: Pos2) -> Option<SelectionRegion> {
        let gesture = self.gesture.take()?;
        let mut points = gesture.points;
        if points.last().is_none_or(|last| *last != point) {
            points.push(point);
        }
        gesture.tool.create_region(gesture.start, point, points)
    }
}

// ============================================================================
// 3. STATO DELLA CRONOLOGIA E APPUNTI (CLIPBOARD)
// ============================================================================

#[derive(Clone, Debug)]
pub struct ClipboardData {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Color32>,
}

pub struct MovingSelection {
    pub origin: SelectionRegion,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub layer_idx: usize,
    pub pixels: Vec<Color32>,
    pub mask: Vec<bool>,
}

#[derive(Clone)]
pub struct HistoryState {
    pub width: usize,
    pub height: usize,
    pub layers: Vec<Layer>,
    pub active_layer_idx: usize,
    pub selection: Option<SelectionRegion>,
}

// ============================================================================
// 4. STATO DELL'APPLICAZIONE
// ============================================================================

pub struct PaintApp {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) layers: Vec<Layer>,
    pub(crate) active_layer_idx: usize,

    // Cronologia
    pub(crate) undo_stack: Vec<HistoryState>,
    pub(crate) redo_stack: Vec<HistoryState>,
    pub(crate) max_history: usize,

    // Selezione e Appunti (Clipboard)
    pub(crate) selection: Option<SelectionRegion>,
    pub(crate) clipboard: Option<ClipboardData>,
    pub(crate) moving_selection: Option<MovingSelection>,
    pub(crate) selection_tools: SelectionToolManager,
    pub(crate) document_revision: u64,
    pub(crate) spray_seed: u64,

    // Proprietà strumenti
    pub(crate) current_tool: Tool,
    pub(crate) brush_color: Color32,
    pub(crate) background_color: Color32,
    pub(crate) brush_size: f32,
    pub(crate) fill_shapes: bool,

    // Percorso del file corrente per salvataggio rapido
    pub(crate) current_filepath: Option<String>,

    // Coda per i pixel flood-fill (evita stack overflow)
    pub(crate) _flood_queue: VecDeque<(i32, i32)>,
}
