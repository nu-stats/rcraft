//! The Graph tab: the plots R has drawn this session, newest shown first, with
//! the command that drew each, back and forward through them, zoom, Copy (as a
//! picture, to the clipboard) and Save as PNG.
//! R writes each page as a PNG file (see the plot folder in `rhost::session`);
//! this reads the files and never draws a plot itself.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui::{self, RichText};
use rcraft_engine::{Engine, Item};

use crate::look::Palette;

/// One drawn page: the command that made it and the file it was written to.
pub struct Plot {
    pub command: u64,
    pub path: PathBuf,
}

/// Every plot in the Results, oldest first.
pub fn plots(engine: &Engine) -> Vec<Plot> {
    engine
        .results()
        .blocks()
        .iter()
        .flat_map(|b| {
            b.items.iter().filter_map(|i| match i {
                Item::Plot { path } => Some(Plot {
                    command: b.command,
                    path: path.clone(),
                }),
                _ => None,
            })
        })
        .collect()
}

/// How big the plot is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Zoom {
    /// The whole picture fits the pane, never enlarged.
    #[default]
    Fit,
    /// This many pixels on screen for each pixel of the picture.
    Scale(f32),
}

/// The zoom steps the buttons and Ctrl+wheel go through.
const STEPS: [f32; 12] = [
    0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0,
];
/// The smallest and largest zoom.
const SMALLEST: f32 = 0.25;
const LARGEST: f32 = 4.0;

impl Zoom {
    /// The scale for a picture of `size` in a pane with `room`.
    pub fn scale(self, size: egui::Vec2, room: egui::Vec2) -> f32 {
        match self {
            Zoom::Fit => fit_scale(size, room),
            Zoom::Scale(s) => s.clamp(SMALLEST, LARGEST),
        }
    }

    /// One step larger than `now` (the scale the picture is drawn at).
    pub fn larger(now: f32) -> Zoom {
        let next = STEPS.iter().copied().find(|&s| s > now + 1e-3);
        Zoom::Scale(next.unwrap_or(LARGEST))
    }

    /// One step smaller than `now`.
    pub fn smaller(now: f32) -> Zoom {
        let next = STEPS.iter().rev().copied().find(|&s| s < now - 1e-3);
        Zoom::Scale(next.unwrap_or(SMALLEST))
    }
}

#[derive(Default)]
pub struct GraphView {
    /// The plot shown, counted from the oldest; `None` follows the newest.
    index: Option<usize>,
    /// How many plots there were when last looked at.
    seen: usize,
    textures: HashMap<PathBuf, egui::TextureHandle>,
    zoom: Zoom,
    /// The scale the plot was last drawn at (0 before the first), for the
    /// zoom buttons, which come before the plot in the frame.
    drawn: f32,
    /// Why a file could not be shown, copied or saved.
    problem: Option<String>,
    /// The plot last copied to the clipboard, for a short note.
    copied: Option<PathBuf>,
}

/// The PNG at `path` as straight (not premultiplied) RGBA pixels.
fn rgba(path: &Path) -> Result<image::RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("could not read the plot: {e}"))?;
    Ok(
        image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .map_err(|e| format!("could not read the plot: {e}"))?
            .to_rgba8(),
    )
}

/// The PNG at `path` as pixels for a texture.
fn load(path: &Path) -> Result<egui::ColorImage, String> {
    let img = rgba(path)?;
    let size = [img.width() as usize, img.height() as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

/// The PNG at `path` as a picture for the clipboard.
fn clipboard_image(path: &Path) -> Result<arboard::ImageData<'static>, String> {
    let img = rgba(path)?;
    Ok(arboard::ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: img.into_raw().into(),
    })
}

/// Puts the plot at `path` on the system clipboard as a picture.
fn copy_picture(path: &Path) -> Result<(), String> {
    let image = clipboard_image(path)?;
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("could not reach the clipboard: {e}"))?;
    clipboard
        .set_image(image)
        .map_err(|e| format!("could not copy the plot: {e}"))
}

/// How much to scale a picture of `size` to fit in `room`: both directions, never
/// above 1, and never zero for a pane that has no room yet.
fn fit_scale(size: egui::Vec2, room: egui::Vec2) -> f32 {
    if size.x <= 0.0 || size.y <= 0.0 {
        return 1.0;
    }
    (room.x / size.x).min(room.y / size.y).clamp(0.05, 1.0)
}

impl GraphView {
    /// Whether plots have arrived since the last call; if so the newest is shown,
    /// fitted to the pane.
    pub fn note_new(&mut self, engine: &Engine) -> bool {
        let now = plots(engine).len();
        let new = now > self.seen;
        if new {
            self.index = None;
            self.zoom = Zoom::Fit;
        }
        self.seen = now;
        new
    }

    #[cfg(test)]
    pub fn shown(&self) -> Option<usize> {
        self.index
    }

    /// Draws the tab. Returns the file name of the plot shown when Export was
    /// pressed: R knows its plots by those names (`options("rcraft.plots")`).
    pub fn show(&mut self, ui: &mut egui::Ui, pal: &Palette, engine: &Engine) -> Option<String> {
        let mut export = None;
        let all = plots(engine);
        let Some(last) = all.len().checked_sub(1) else {
            ui.label(
                RichText::new(
                    "No plots yet. A command that draws one, such as plot(1:10) or a dialog \
                     under Graphs, shows it here.",
                )
                .color(pal.dim),
            );
            return None;
        };
        let at = self.index.unwrap_or(last).min(last);
        let current = all.get(at)?;
        let path = &current.path;
        if !self.textures.contains_key(path) {
            match load(path) {
                Ok(pixels) => {
                    let handle = ui.ctx().load_texture(
                        path.to_string_lossy(),
                        pixels,
                        egui::TextureOptions::LINEAR,
                    );
                    self.textures.insert(path.clone(), handle);
                }
                Err(why) => {
                    ui.label(RichText::new(why).color(pal.error));
                    return export;
                }
            }
        }
        let size = self
            .textures
            .get(path)
            .map_or(egui::Vec2::ZERO, |t| t.size_vec2());
        ui.horizontal(|ui| {
            if ui
                .add_enabled(at > 0, egui::Button::new("◀ Older"))
                .clicked()
            {
                self.index = Some(at - 1);
            }
            ui.label(RichText::new(format!("Plot {} of {}", at + 1, all.len())).strong());
            if ui
                .add_enabled(at < last, egui::Button::new("Newer ▶"))
                .clicked()
            {
                self.index = if at + 1 == last { None } else { Some(at + 1) };
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Save as PNG…").clicked() {
                    self.save(path);
                }
                if ui
                    .button("Export…")
                    .on_hover_text("PNG, SVG or PDF at a size and resolution you choose")
                    .clicked()
                {
                    export = path.file_name().map(|n| n.to_string_lossy().into_owned());
                }
                if ui
                    .button("Copy")
                    .on_hover_text("Copy the plot to the clipboard as a picture")
                    .clicked()
                {
                    self.problem = None;
                    self.copied = None;
                    match copy_picture(path) {
                        Ok(()) => self.copied = Some(path.clone()),
                        Err(why) => self.problem = Some(why),
                    }
                }
                ui.separator();
                self.zoom_buttons(ui);
            });
        });
        if let Some(entry) = engine.history().get(current.command) {
            let first = entry.code.trim_end().lines().next().unwrap_or("");
            ui.label(
                RichText::new(format!("> {first}"))
                    .monospace()
                    .small()
                    .color(pal.accent),
            );
        }
        if let Some(problem) = &self.problem {
            ui.label(RichText::new(problem).small().color(pal.error));
        } else if self.copied.as_ref() == Some(path) {
            ui.label(
                RichText::new("Copied to the clipboard as a picture.")
                    .small()
                    .color(pal.dim),
            );
        }
        ui.add_space(4.0);
        if let Some(texture) = self.textures.get(path) {
            let room = ui.available_size();
            let scale = self.zoom.scale(size, room);
            self.drawn = scale;
            let out = egui::ScrollArea::both()
                .id_salt("graph")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.image((texture.id(), size * scale));
                });
            // Ctrl and the wheel (or a pinch) over the plot zoom it
            let hovered = ui.rect_contains_pointer(out.inner_rect);
            let pinch = ui.input(|i| i.zoom_delta());
            if hovered && (pinch - 1.0).abs() > 1e-3 {
                self.zoom = if pinch > 1.0 {
                    Zoom::larger(scale)
                } else {
                    Zoom::smaller(scale)
                };
            }
        }
        export
    }

    /// Fit, smaller, the scale shown (press for 100%), larger.
    fn zoom_buttons(&mut self, ui: &mut egui::Ui) {
        // laid out right to left: the last added is leftmost
        let now = if self.drawn > 0.0 { self.drawn } else { 1.0 };
        if ui
            .button("+")
            .on_hover_text("Larger (Ctrl and the mouse wheel over the plot)")
            .clicked()
        {
            self.zoom = Zoom::larger(now);
        }
        if ui
            .button(format!("{:.0}%", now * 100.0))
            .on_hover_text("Show at its own size (100%)")
            .clicked()
        {
            self.zoom = Zoom::Scale(1.0);
        }
        if ui.button("−").on_hover_text("Smaller").clicked() {
            self.zoom = Zoom::smaller(now);
        }
        if ui
            .selectable_label(self.zoom == Zoom::Fit, "Fit")
            .on_hover_text("Fit the whole plot in the pane")
            .clicked()
        {
            self.zoom = Zoom::Fit;
        }
    }

    /// Copies the plot file where the user chooses.
    fn save(&mut self, from: &Path) {
        self.problem = None;
        let Some(to) = rfd::FileDialog::new()
            .set_file_name("plot.png")
            .add_filter("PNG picture", &["png"])
            .save_file()
        else {
            return;
        };
        if let Err(e) = std::fs::copy(from, &to) {
            self.problem = Some(format!("could not save the plot: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_png_file_loads_into_pixels_and_garbage_is_an_error() {
        let dir = std::env::temp_dir().join(format!("rcraft-graph-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let good = dir.join("p.png");
        // a 2 by 1 picture written with the image crate itself
        let mut img = image::RgbaImage::new(2, 1);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([0, 0, 255, 128]));
        assert!(img.save(&good).is_ok());
        let pixels = load(&good);
        assert_eq!(pixels.as_ref().map(|p| p.size), Ok([2, 1]));
        // the clipboard gets the same pixels, straight RGBA, half-clear kept as it is
        let clip = clipboard_image(&good);
        assert!(clip.is_ok());
        if let Ok(c) = clip {
            assert_eq!((c.width, c.height), (2, 1));
            assert_eq!(&c.bytes[..], &[255, 0, 0, 255, 0, 0, 255, 128]);
        }
        let bad = dir.join("bad.png");
        assert!(std::fs::write(&bad, b"not a png").is_ok());
        assert!(load(&bad).is_err());
        assert!(clipboard_image(&bad).is_err());
        assert!(load(&dir.join("missing.png")).is_err());
        assert!(copy_picture(&dir.join("missing.png")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod fit_tests {
    use super::*;

    #[test]
    fn a_picture_is_fitted_in_both_directions_and_never_enlarged() {
        let pic = egui::vec2(1000.0, 700.0);
        // wide room, short pane: the height decides
        assert!((fit_scale(pic, egui::vec2(2000.0, 350.0)) - 0.5).abs() < 1e-6);
        // narrow room, tall pane: the width decides
        assert!((fit_scale(pic, egui::vec2(500.0, 2000.0)) - 0.5).abs() < 1e-6);
        // plenty of room: shown at its own size
        assert_eq!(fit_scale(pic, egui::vec2(5000.0, 5000.0)), 1.0);
        // no room yet, or an empty picture: something sane, not zero or a panic
        assert_eq!(fit_scale(pic, egui::vec2(0.0, 0.0)), 0.05);
        assert_eq!(
            fit_scale(egui::vec2(0.0, 0.0), egui::vec2(100.0, 100.0)),
            1.0
        );
    }

    #[test]
    fn zoom_steps_up_and_down_and_stays_within_bounds() {
        let pic = egui::vec2(1000.0, 700.0);
        let room = egui::vec2(500.0, 2000.0);
        assert!((Zoom::Fit.scale(pic, room) - 0.5).abs() < 1e-6);
        // from a fitted 0.5: one step either way
        assert_eq!(Zoom::larger(0.5), Zoom::Scale(0.67));
        assert_eq!(Zoom::smaller(0.5), Zoom::Scale(0.33));
        // from between steps, to the next step that way
        assert_eq!(Zoom::larger(0.8), Zoom::Scale(0.9));
        assert_eq!(Zoom::smaller(0.8), Zoom::Scale(0.75));
        // at the ends it stays there
        assert_eq!(Zoom::larger(4.0), Zoom::Scale(4.0));
        assert_eq!(Zoom::smaller(0.25), Zoom::Scale(0.25));
        assert_eq!(Zoom::smaller(0.05), Zoom::Scale(0.25));
        // a scale set out of range is held to it
        assert_eq!(Zoom::Scale(50.0).scale(pic, room), 4.0);
        assert_eq!(Zoom::Scale(0.0).scale(pic, room), 0.25);
        assert_eq!(STEPS.first(), Some(&SMALLEST));
        assert_eq!(STEPS.last(), Some(&LARGEST));
        // a zoomed picture is not held to the pane: it can be larger
        assert_eq!(Zoom::Scale(2.0).scale(pic, room), 2.0);
    }
}
