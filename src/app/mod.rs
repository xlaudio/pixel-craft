//! Application state and document operations.
//!
//! The UI adapter owns presentation state, notifications, and rendering
//! resources. This module contains the document and its editing behavior.

pub mod commands;
mod document;
mod file_io;
mod history;
mod tools;

pub use commands::{Command, CommandEngine};

use egui::Color32;
use std::collections::VecDeque;

use crate::model::{Layer, PaintApp, SelectionRegion, Tool};

impl PaintApp {
    /// Execute an application command and return user-facing notices.
    pub fn execute_command(&mut self, command: Command) -> Vec<String> {
        CommandEngine.execute(self, command)
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn active_layer_index(&self) -> usize {
        self.active_layer_idx
    }

    pub fn selection(&self) -> Option<egui::Rect> {
        self.selection.as_ref().map(SelectionRegion::bounds)
    }

    pub fn selection_region(&self) -> Option<&SelectionRegion> {
        self.selection.as_ref()
    }

    pub fn selection_contains(&self, point: egui::Pos2) -> bool {
        self.selection
            .as_ref()
            .is_some_and(|selection| selection.contains(point))
    }

    pub fn clipboard(&self) -> Option<&crate::model::ClipboardData> {
        self.clipboard.as_ref()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn current_tool(&self) -> Tool {
        self.current_tool
    }

    pub fn brush_color(&self) -> Color32 {
        self.brush_color
    }

    pub fn background_color(&self) -> Color32 {
        self.background_color
    }

    pub fn brush_size(&self) -> f32 {
        self.brush_size
    }

    pub fn fill_shapes(&self) -> bool {
        self.fill_shapes
    }

    pub fn current_filepath(&self) -> Option<&str> {
        self.current_filepath.as_deref()
    }

    pub fn document_revision(&self) -> u64 {
        self.document_revision
    }

    pub fn new_headless() -> Self {
        let width = 1024;
        let height = 768;

        let mut background = Layer::new("Sfondo", width, height);
        background.pixels.fill(Color32::WHITE);

        Self {
            width,
            height,
            layers: vec![background, Layer::new("Livello 1", width, height)],
            active_layer_idx: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 25,
            selection: None,
            clipboard: None,
            moving_selection: None,
            selection_tools: Default::default(),
            document_revision: 0,
            spray_seed: 0x9E37_79B9_7F4A_7C15,
            current_tool: Tool::Brush,
            brush_color: Color32::from_rgb(200, 50, 50),
            background_color: Color32::WHITE,
            brush_size: 10.0,
            fill_shapes: false,
            current_filepath: None,
            _flood_queue: VecDeque::new(),
        }
    }

    pub(crate) fn mark_document_changed(&mut self) {
        self.document_revision = self.document_revision.wrapping_add(1);
    }
}
