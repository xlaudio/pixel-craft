//! # history — Cronologia delle modifiche
//!
//! Gestisce snapshot dello stato del documento e operazioni undo/redo.
//! Le modifiche grafiche chiamano `push_undo_snapshot` prima di intervenire
//! sui livelli, così l'utente può ripristinare lo stato precedente.

use crate::model::{HistoryState, PaintApp};

impl PaintApp {
    pub fn push_undo_snapshot(&mut self) {
        let pixels = self.width * self.height;
        let limit = if pixels > 6_000_000 {
            4
        } else if pixels > 2_000_000 {
            8
        } else {
            self.max_history
        };

        self.undo_stack.push(HistoryState {
            width: self.width,
            height: self.height,
            layers: self.layers.clone(),
            active_layer_idx: self.active_layer_idx,
            selection: self.selection,
        });
        while self.undo_stack.len() > limit {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(HistoryState {
                width: self.width,
                height: self.height,
                layers: self.layers.clone(),
                active_layer_idx: self.active_layer_idx,
                selection: self.selection,
            });
            self.width = prev.width;
            self.height = prev.height;
            self.layers = prev.layers;
            self.active_layer_idx = prev
                .active_layer_idx
                .min(self.layers.len().saturating_sub(1));
            self.selection = prev.selection;
            self.dirty_canvas = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(HistoryState {
                width: self.width,
                height: self.height,
                layers: self.layers.clone(),
                active_layer_idx: self.active_layer_idx,
                selection: self.selection,
            });
            self.width = next.width;
            self.height = next.height;
            self.layers = next.layers;
            self.active_layer_idx = next
                .active_layer_idx
                .min(self.layers.len().saturating_sub(1));
            self.selection = next.selection;
            self.dirty_canvas = true;
        }
    }
}
