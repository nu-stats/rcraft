//! `rcraft-desktop`: the RCraft window (layer L7).
//!
//! `rcraft-desktop` opens the window. `rcraft-desktop --picture FILE.png`
//! draws it to a picture instead, with no window and no graphics driver:
//!
//! ```text
//! --picture FILE   write the window to FILE (a PNG)
//! --demo FILE      first run each line of FILE as a typed command
//! --dark           dark mode (default light)
//! --accent NAME    Blue, Indigo, Teal, Plum, Forest, Amber or Rose
//! --size WxH       size in points (default 1240x780)
//! --scale N        pixels per point (default 1)
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use rcraft_ui_egui::{Accent, HeadlessOptions};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = if args.is_empty() {
        rcraft_ui_egui::run()
    } else {
        parse(&args).and_then(|o| rcraft_ui_egui::run_headless(&o))
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rcraft-desktop: {e}");
            ExitCode::from(1)
        }
    }
}

fn parse(args: &[String]) -> Result<HeadlessOptions, String> {
    let mut picture: Option<PathBuf> = None;
    let mut options = HeadlessOptions::new(PathBuf::new());
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--picture" => picture = Some(PathBuf::from(value("--picture")?)),
            "--demo" => options.demo = Some(PathBuf::from(value("--demo")?)),
            "--dark" => options.dark = true,
            "--accent" => {
                let name = value("--accent")?;
                options.accent = Accent::ALL
                    .into_iter()
                    .find(|a| a.name().eq_ignore_ascii_case(name))
                    .ok_or_else(|| format!("unknown accent {name}"))?;
            }
            "--size" => {
                let text = value("--size")?;
                let (w, h) = text.split_once(['x', 'X']).ok_or("--size is WxH")?;
                options.size = [
                    w.parse().map_err(|_| "--size is WxH")?,
                    h.parse().map_err(|_| "--size is WxH")?,
                ];
            }
            "--scale" => {
                options.pixels_per_point = value("--scale")?
                    .parse()
                    .map_err(|_| "--scale is a number".to_string())?;
            }
            other => return Err(format!("unknown option {other}; see the top of main.rs")),
        }
    }
    options.output = picture.ok_or("--picture FILE is needed")?;
    Ok(options)
}
