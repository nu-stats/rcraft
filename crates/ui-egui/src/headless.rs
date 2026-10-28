//! Draws the real window to a PNG file with no window and no graphics
//! driver: the same `App` runs frame by frame, and the software rasterizer
//! paints what egui produces.
//!
//! Used to look at the interface on machines where no window can open, and
//! for pictures in the documentation. With a demo file, each line in it is
//! run as a typed command first, one at a time.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::app::App;
use crate::look::Accent;
use crate::softraster::{encode_png, Canvas, Textures, MAX_SIDE};

/// Frames to wait after the last command, so the window has settled.
const SETTLE_FRAMES: u32 = 15;
/// Give up after this long (R failing to start, a command that never ends).
const GIVE_UP: Duration = Duration::from_secs(90);
/// Most demo lines read.
const MAX_DEMO_LINES: usize = 200;

#[derive(Clone, Debug)]
pub struct HeadlessOptions {
    pub output: PathBuf,
    pub demo: Option<PathBuf>,
    pub dark: bool,
    /// Show the Data tab instead of Results.
    pub show_data: bool,
    /// Show the Objects list instead of Variables.
    pub show_objects: bool,
    /// List only variables of this type (`Factors`, `Numbers`, ...).
    pub filter: Option<String>,
    /// After the demo, open this Data menu dialog (`"Sort rows"`) ...
    pub dialog: Option<String>,
    /// ... with these values set, as (widget id, value).
    pub set: Vec<(String, String)>,
    /// Open "Use my file" on this file.
    pub import: Option<PathBuf>,
    /// The recent files "Use my file" offers, newest first. A picture never
    /// reads or changes the user's own list.
    pub recent: Vec<String>,
    /// Open the Packages window on this tab (`find` or `installed`) ...
    pub packages: Option<String>,
    /// ... with this search typed in and this package selected ...
    pub packages_query: String,
    pub packages_select: Option<String>,
    /// ... and the install list for it showing, waiting for a yes ...
    pub packages_confirm: bool,
    /// ... or a made-up install failure (`compiler`, `newer-r`, `network`) ...
    pub packages_failure: Option<String>,
    /// ... over the package list read from this downloaded `packages.rds` (a
    /// small made-up list when none). A picture never reaches the network or
    /// reads the user's own cache.
    pub packages_from: Option<PathBuf>,
    pub accent: Accent,
    /// Width and height in points.
    pub size: [u32; 2],
    pub pixels_per_point: f32,
}

impl HeadlessOptions {
    pub fn new(output: PathBuf) -> HeadlessOptions {
        HeadlessOptions {
            output,
            demo: None,
            dark: false,
            show_data: false,
            show_objects: false,
            filter: None,
            dialog: None,
            set: Vec::new(),
            import: None,
            recent: Vec::new(),
            packages: None,
            packages_query: String::new(),
            packages_select: None,
            packages_confirm: false,
            packages_failure: None,
            packages_from: None,
            accent: Accent::default(),
            size: [1240, 780],
            pixels_per_point: 1.0,
        }
    }
}

/// Runs the window offscreen and writes the picture.
pub fn run_headless(options: &HeadlessOptions) -> Result<(), String> {
    let ppp = options.pixels_per_point.clamp(0.5, 4.0);
    let side = |points: u32| ((points as f32 * ppp).round() as usize).clamp(1, MAX_SIDE);
    let (width, height) = (side(options.size[0]), side(options.size[1]));
    let demo = read_demo(options.demo.as_deref())?;

    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(ppp);
    let mut app = App::new(&ctx);
    let mut recent = rcraft_rhost::recent::Recent::at(None);
    for path in options.recent.iter().rev() {
        recent.add(path);
    }
    app.use_recent(recent);
    app.set_theme(
        &ctx,
        if options.dark {
            egui::ThemePreference::Dark
        } else {
            egui::ThemePreference::Light
        },
    );
    app.set_accent(&ctx, options.accent);
    if options.show_data {
        app.show_data(&ctx);
    }
    if options.show_objects {
        app.show_objects(&ctx);
    }
    if let Some(name) = &options.filter {
        app.filter_variables(&ctx, name)?;
    }
    if let Some(path) = &options.import {
        app.open_import(path);
    }
    if let Some(tab) = &options.packages {
        app.show_packages(
            tab,
            &options.packages_query,
            options.packages_select.as_deref(),
            options.packages_confirm,
            options.packages_from.as_deref(),
        )?;
        if let Some(kind) = &options.packages_failure {
            app.show_packages_failure(kind)?;
        }
    }

    let screen = Rect::from_min_size(
        Pos2::ZERO,
        egui::vec2(width as f32 / ppp, height as f32 / ppp),
    );
    let mut textures = Textures::default();
    let started = Instant::now();
    let (mut next, mut settled) = (0usize, 0u32);
    let mut dialog_opened = false;
    let mut shapes: Vec<egui::epaint::ClippedShape>;
    loop {
        let raw = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(started.elapsed().as_secs_f64()),
            focused: true,
            ..Default::default()
        };
        let output = ctx.run(raw, |ctx| app.ui(ctx));
        textures.apply(&output.textures_delta);
        shapes = output.shapes;

        if app.is_settled() {
            match demo.get(next) {
                Some(code) if app.is_ready() => {
                    app.submit_typed(code);
                    next += 1;
                    settled = 0;
                }
                // after the demo, once the data frame has come over
                None if !dialog_opened && settled >= SETTLE_FRAMES / 2 => {
                    if let Some(entry) = &options.dialog {
                        app.open_dialog(&ctx, entry, &options.set)?;
                        settled = 0;
                    }
                    dialog_opened = true;
                }
                _ => settled += 1,
            }
        }
        if settled >= SETTLE_FRAMES {
            break;
        }
        if started.elapsed() > GIVE_UP {
            return Err("the window did not settle within 90 seconds".into());
        }
        std::thread::sleep(Duration::from_millis(30));
    }

    let background = if options.dark {
        Color32::from_rgb(26, 32, 39)
    } else {
        Color32::from_rgb(244, 246, 248)
    };
    let mut canvas = Canvas::new(width, height, background);
    let primitives = ctx.tessellate(shapes, ppp);
    canvas.draw(&primitives, &textures, ppp);
    let png = encode_png(canvas.width, canvas.height, &canvas.pixels);
    std::fs::write(&options.output, png)
        .map_err(|e| format!("could not write {}: {e}", options.output.display()))
}

fn read_demo(path: Option<&Path>) -> Result<Vec<String>, String> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(MAX_DEMO_LINES)
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_demo_file_is_an_error_not_a_crash() {
        assert!(read_demo(Some(Path::new("definitely/not/here.txt"))).is_err());
        assert_eq!(read_demo(None), Ok(Vec::new()));
    }

    #[test]
    fn defaults_are_the_window_size_and_teal() {
        let o = HeadlessOptions::new(PathBuf::from("x.png"));
        assert_eq!(o.size, [1240, 780]);
        assert_eq!(o.accent, Accent::Teal);
        assert!(!o.dark);
    }
}
