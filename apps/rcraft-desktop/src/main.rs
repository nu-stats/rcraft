//! `rcraft-desktop`: the RCraft window (layer L7).

fn main() -> std::process::ExitCode {
    match rcraft_ui_egui::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rcraft-desktop: {e}");
            std::process::ExitCode::from(1)
        }
    }
}
