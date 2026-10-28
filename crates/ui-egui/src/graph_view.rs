//! The Graph tab: the plots R has drawn this session, newest shown first, with
//! the command that drew each, back and forward through them, and Save as PNG.
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

#[derive(Default)]
pub struct GraphView {
    /// The plot shown, counted from the oldest; `None` follows the newest.
    index: Option<usize>,
    /// How many plots there were when last looked at.
    seen: usize,
    textures: HashMap<PathBuf, egui::TextureHandle>,
    /// Why a file could not be shown or saved.
    problem: Option<String>,
}

/// The PNG at `path` as pixels for a texture.
fn load(path: &Path) -> Result<egui::ColorImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("could not read the plot: {e}"))?;
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|e| format!("could not read the plot: {e}"))?
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
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
    /// Whether plots have arrived since the last call; if so the newest is shown.
    pub fn note_new(&mut self, engine: &Engine) -> bool {
        let now = plots(engine).len();
        let new = now > self.seen;
        if new {
            self.index = None;
        }
        self.seen = now;
        new
    }

    #[cfg(test)]
    pub fn shown(&self) -> Option<usize> {
        self.index
    }

    pub fn show(&mut self, ui: &mut egui::Ui, pal: &Palette, engine: &Engine) {
        let all = plots(engine);
        let Some(last) = all.len().checked_sub(1) else {
            ui.label(
                RichText::new(
                    "No plots yet. A command that draws one, such as plot(1:10) or a dialog \
                     under Graphs, shows it here.",
                )
                .color(pal.dim),
            );
            return;
        };
        let at = self.index.unwrap_or(last).min(last);
        let Some(current) = all.get(at) else {
            return;
        };
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
                    self.save(&current.path);
                }
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
        }
        ui.add_space(4.0);
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
                    return;
                }
            }
        }
        if let Some(texture) = self.textures.get(path) {
            // the whole picture fits the pane, in both directions, never enlarged
            let room = ui.available_size();
            egui::ScrollArea::both()
                .id_salt("graph")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let size = texture.size_vec2();
                    let scale = fit_scale(size, room);
                    ui.image((texture.id(), size * scale));
                });
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
        img.put_pixel(1, 0, image::Rgba([0, 0, 255, 255]));
        assert!(img.save(&good).is_ok());
        let pixels = load(&good);
        assert_eq!(pixels.as_ref().map(|p| p.size), Ok([2, 1]));
        let bad = dir.join("bad.png");
        assert!(std::fs::write(&bad, b"not a png").is_ok());
        assert!(load(&bad).is_err());
        assert!(load(&dir.join("missing.png")).is_err());
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
}
