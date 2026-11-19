//! # ui — Interfaccia grafica
//!
//! Raggruppa i componenti egui dell'applicazione:
//! - [`canvas`] — area di disegno, input, zoom e anteprime
//! - [`menu`] — barra dei menu e relative azioni
//! - [`dialogs`] — dialoghi per le operazioni sui documenti
//! - [`panels`] — pannelli strumenti, livelli e tavolozza

pub mod canvas;
pub mod dialogs;
pub mod eframe_app;
pub mod menu;
pub mod panels;
pub mod selection_renderer;

pub use eframe_app::{Appearance, ColorDialogTarget, EframeApp, UiState};
