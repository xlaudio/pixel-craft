//! # main.rs — Native application entry point
//!
//! The reusable application library and eframe integration live in `lib.rs`
//! and `ui::eframe_app`; this file only launches the native window.

use eframe::egui;

use pixel_craft::ui::EframeApp;

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        follow_system_theme: true,
        default_theme: eframe::Theme::Light,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 750.0])
            .with_title("PixelCraft"),
        ..Default::default()
    };

    eframe::run_native(
        "PixelCraft",
        options,
        Box::new(|cc| Box::new(EframeApp::new(cc))),
    )
}
