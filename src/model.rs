//! # model.rs — Strutture dati dell'applicazione
//!
//! Questo file definisce **solo tipi e struct**, senza logica applicativa.
//! È il "contratto" condiviso da tutti gli altri moduli: ogni modulo
//! importa da qui le definizioni di cui ha bisogno.
//!
//! ## Contenuto
//! - [`Layer`]         — Un livello pittura (nome, visibilità, opacità, pixel)
//! - [`Tool`]          — Enumerazione degli strumenti disponibili (pennello, gomma, …)
//! - [`ClipboardData`] — Buffer degli appunti (pixel copiati/tagliati)
//! - [`HistoryState`]  — Snapshot dello stato per undo/redo
//! - [`PaintApp`]      — Struttura principale con tutto lo stato dell'applicazione

use egui::{Color32, Pos2, Rect, TextureHandle, Vec2};
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
    Select,    // Selezione rettangolare (S)
    Move,      // Sposta tela o selezione (M)
    Brush,     // Pennello a tratto continuo (B)
    Eraser,    // Gomma da cancellare (E)
    Pipette,   // Contagocce / Color picker (I)
    Bucket,    // Secchiello / Riempimento flood-fill (G)
    Line,      // Linea retta (L)
    Rectangle, // Rettangolo (R)
    Circle,    // Cerchio / Ellisse (C)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Appearance {
    /// Follow the platform theme when the windowing backend can detect it.
    System,
    /// Always use the dark color scheme.
    Dark,
    /// Always use the light color scheme.
    Light,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorDialogTarget {
    Foreground,
    Background,
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
    pub origin: Rect,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub layer_idx: usize,
    pub pixels: Vec<Color32>,
}

#[derive(Clone)]
pub struct HistoryState {
    pub width: usize,
    pub height: usize,
    pub layers: Vec<Layer>,
    pub active_layer_idx: usize,
    pub selection: Option<Rect>,
}

// ============================================================================
// 4. STATO DELL'APPLICAZIONE
// ============================================================================

pub struct PaintApp {
    pub width: usize,
    pub height: usize,
    pub layers: Vec<Layer>,
    pub active_layer_idx: usize,

    // Cronologia
    pub undo_stack: Vec<HistoryState>,
    pub redo_stack: Vec<HistoryState>,
    pub max_history: usize,

    // Selezione e Appunti (Clipboard)
    pub selection: Option<Rect>,
    pub clipboard: Option<ClipboardData>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub dirty_canvas: bool,

    // Proprietà strumenti
    pub current_tool: Tool,
    pub brush_color: Color32,
    pub background_color: Color32,
    pub brush_size: f32,
    pub fill_shapes: bool,
    pub appearance: Appearance,
    pub applied_dark_theme: bool,

    // Vista (Zoom e Pan)
    pub zoom: f32,
    pub pan: Vec2,

    // Stato interazione disegno
    pub canvas_texture: Option<TextureHandle>,
    pub is_drawing: bool,
    pub drag_start_canvas: Option<Pos2>,
    pub last_canvas_pos: Option<Pos2>,
    pub current_canvas_pos: Option<Pos2>,
    pub moving_selection: Option<MovingSelection>,

    // Tavolozza rapida
    pub palette: Vec<Color32>,

    // Percorso del file corrente per salvataggio rapido
    pub current_filepath: Option<String>,

    // Dialoghi per le dimensioni del documento
    pub show_new_image_dialog: bool,
    pub new_image_width: usize,
    pub new_image_height: usize,
    pub show_resize_image_dialog: bool,
    pub resize_image_width: usize,
    pub resize_image_height: usize,
    pub lock_resize_aspect_ratio: bool,
    pub show_print_dialog: bool,
    pub print_output_path: String,
    pub color_dialog_target: Option<ColorDialogTarget>,
    pub color_dialog_color: Color32,

    // Coda per i pixel flood-fill (evita stack overflow)
    pub _flood_queue: VecDeque<(i32, i32)>,
}
