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

use egui::{Color32, Rect};
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
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) layers: Vec<Layer>,
    pub(crate) active_layer_idx: usize,

    // Cronologia
    pub(crate) undo_stack: Vec<HistoryState>,
    pub(crate) redo_stack: Vec<HistoryState>,
    pub(crate) max_history: usize,

    // Selezione e Appunti (Clipboard)
    pub(crate) selection: Option<Rect>,
    pub(crate) clipboard: Option<ClipboardData>,
    pub(crate) moving_selection: Option<MovingSelection>,
    pub(crate) document_revision: u64,

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
