//! The desktop window (layout B, "Workbench"; see `design/layouts.html`).
//!
//! Variables over History on the left, the Script above Results in the
//! middle, the Command line below, and a status bar. The interface stays
//! thin (AGENTS.md section 3): it reads the engine and sends it commands.
//!
//! Two ways to show it: a real window ([`run`]), and a picture drawn on the
//! CPU with no window and no graphics driver ([`run_headless`]).

mod app;
mod headless;
pub mod look;
mod softraster;

pub use headless::{run_headless, HeadlessOptions};
pub use look::Accent;

fn run_with(renderer: eframe::Renderer) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("RCraft")
            .with_inner_size([1240.0, 780.0])
            .with_min_inner_size([760.0, 480.0]),
        renderer,
        ..Default::default()
    };
    eframe::run_native(
        "RCraft",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(&cc.egui_ctx)))),
    )
    .map_err(|e| e.to_string())
}

/// Opens the window and runs until it is closed.
///
/// Drawing uses `wgpu` (Direct3D, Vulkan or Metal), which works on machines
/// where OpenGL does not; if it cannot start, OpenGL is tried.
pub fn run() -> Result<(), String> {
    match run_with(eframe::Renderer::Wgpu) {
        Ok(()) => Ok(()),
        Err(first) => run_with(eframe::Renderer::Glow)
            .map_err(|second| format!("could not open the window: {first}; then {second}")),
    }
}
