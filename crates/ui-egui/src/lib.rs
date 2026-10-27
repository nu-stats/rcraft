//! The desktop window (layout B, "Workbench"; see `design/layouts.html`).
//!
//! Variables over History on the left, the Script above Results in the
//! middle, the Command line below, and a status bar. The interface stays
//! thin (AGENTS.md section 3): it reads the engine and sends it commands.
//!
//! Two ways to show it: a real window ([`run`]), and a picture drawn on the
//! CPU with no window and no graphics driver ([`run_headless`]).

mod app;
mod data_view;
mod headless;
pub mod look;
mod softraster;
mod softwindow;

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
/// where OpenGL does not; if it cannot start, OpenGL is tried, and if that
/// cannot either, the window is drawn on the CPU (see `softwindow.rs`).
pub fn run() -> Result<(), String> {
    match run_with(eframe::Renderer::Wgpu) {
        Ok(()) => Ok(()),
        Err(first) => match run_with(eframe::Renderer::Glow) {
            Ok(()) => Ok(()),
            Err(second) => {
                eprintln!(
                    "rcraft-desktop: no graphics driver ({first}; {second}); drawing the window in software"
                );
                softwindow::run()
            }
        },
    }
}

/// Opens the window drawn on the CPU, whatever the graphics driver is doing.
pub fn run_software() -> Result<(), String> {
    softwindow::run()
}
