//! Stata mode (P2B.7): Stata typed in the Command window and written in the
//! Script Editor, translated to R by `statatr`, and run in R.
//!
//! The switch is per session, on the Command and Script panes alike. In
//! Stata mode a typed line goes through the session's [`Live`] translator,
//! which knows the lines before it, and the R it makes is what runs (with
//! the Stata as a comment above it and the notes below, so Results and the
//! session script show both). The Script Editor shows two panes: the Stata
//! on the left and its R on the right, line by line; the R of the line the
//! cursor is on is marked, and a click on an R block selects its Stata.
//! Every line with a known difference from Stata carries its note there.

use std::path::Path;

use eframe::egui::{self, RichText};
use rcraft_statatr::complete::{self, Completion, Context};
use rcraft_statatr::data_r::{translate, Translation};
use rcraft_statatr::live::{Live, HELPERS_BEGIN, HELPERS_END};
use rcraft_statatr::reader::read;

use crate::look::Palette;

/// What a script holds when Stata mode starts on an untouched one.
pub const SCRIPT_START: &str =
    "* Type Stata here.\n* Ctrl+Enter runs the selection, or the line the cursor is on.\n";

/// Most entries of a folder read for Tab.
const MAX_FILES: usize = 2_000;

/// One session's Stata mode.
#[derive(Default)]
pub struct StataMode {
    pub on: bool,
    /// The Stata run so far, for what each new line means.
    pub live: Live,
    /// A data frame to show once R lists it (after `use`), staying on Results.
    pub follow: Option<String>,
    /// The Stata lines typed, oldest first, and where Page Up has got to.
    typed: Vec<String>,
    at: Option<usize>,
    /// The script's translation, with the text it was made from.
    view: Option<(String, Translation)>,
}

impl StataMode {
    /// R was restarted: what the translator followed is gone with it.
    pub fn forget_r(&mut self) {
        self.live = Live::new();
    }

    /// Keeps a typed line for Page Up.
    pub fn typed(&mut self, line: &str) {
        if self.typed.last().map(String::as_str) != Some(line) {
            self.typed.push(line.to_string());
        }
        self.at = None;
    }

    /// An earlier typed line (Page Up).
    pub fn previous(&mut self) -> Option<String> {
        let i = match self.at {
            None => self.typed.len().checked_sub(1)?,
            Some(i) => i.checked_sub(1)?,
        };
        self.at = Some(i);
        self.typed.get(i).cloned()
    }

    /// A later one (Page Down); past the newest, an empty line.
    pub fn next(&mut self) -> Option<String> {
        let i = self.at? + 1;
        if i >= self.typed.len() {
            self.at = None;
            return Some(String::new());
        }
        self.at = Some(i);
        self.typed.get(i).cloned()
    }

    /// The script's translation, made again only when the script changed.
    pub fn translation(&mut self, script: &str) -> &Translation {
        if self.view.as_ref().is_none_or(|(s, _)| s != script) {
            self.view = Some((script.to_string(), translate(&read(script))));
        }
        let (_, t) = self
            .view
            .get_or_insert_with(|| (String::new(), Translation::default()));
        t
    }

    /// Tab on a Stata line: the line completed as far as the choices agree,
    /// and the choices when there are several. `variables` are those of the
    /// data in memory; file names come from `folder` (R's working folder).
    pub fn complete(
        &self,
        line: &str,
        variables: &[String],
        folder: Option<&Path>,
    ) -> Option<(String, Vec<String>)> {
        let files =
            |dir: &str| -> Vec<String> { folder.map(|f| entries(f, dir)).unwrap_or_default() };
        let ctx = Context {
            variables,
            locals: self.live.locals(),
            globals: self.live.globals(),
            labels: self.live.value_labels(),
            files: &files,
        };
        let c: Completion = complete::complete(line, &ctx)?;
        Some(complete::apply(line, &c))
    }
}

/// The entries of `dir` (as typed, relative to `folder`): files, and folders
/// with a `/`, hidden ones left out.
fn entries(folder: &Path, dir: &str) -> Vec<String> {
    let path = folder.join(dir);
    let Ok(read) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    read.filter_map(Result::ok)
        .take(MAX_FILES)
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            if name.starts_with('.') {
                return None;
            }
            let dir = e.file_type().ok()?.is_dir();
            Some(if dir { format!("{name}/") } else { name })
        })
        .collect()
}

/// The lines of code Results shows for a command: RCraft's Stata helpers
/// folded to one line (the session script keeps them whole).
pub fn shown_lines(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = code.trim_end().lines();
    while let Some(line) = lines.next() {
        if line.starts_with(HELPERS_BEGIN) {
            let mut n = 0usize;
            for l in lines.by_ref() {
                if l == HELPERS_END {
                    break;
                }
                n = n.saturating_add(1);
            }
            out.push(format!("{line} ({n} lines, in the session script)"));
            continue;
        }
        out.push(line.to_string());
    }
    out
}

/// What History shows for a command: its first line, past RCraft's Stata
/// helpers (so a Stata line shows as its Stata, a comment).
pub fn history_line(code: &str) -> &str {
    let mut lines = code.lines();
    while let Some(line) = lines.next() {
        if line.starts_with(HELPERS_BEGIN) {
            for l in lines.by_ref() {
                if l == HELPERS_END {
                    break;
                }
            }
            continue;
        }
        return line;
    }
    ""
}

/// The line (0-based) the character `at` is on.
pub fn line_of(text: &str, at: usize) -> usize {
    text.chars().take(at).filter(|c| *c == '\n').count()
}

/// The characters from the start of line `first` (1-based) to the end of the
/// line `lines` later, for selecting a command's Stata.
pub fn char_range(text: &str, first: usize, lines: usize) -> (usize, usize) {
    let mut start = 0usize;
    let mut line = 1usize;
    let mut end = None;
    let last = first.saturating_add(lines.max(1)).saturating_sub(1);
    for (i, c) in text.chars().enumerate() {
        if c == '\n' {
            if line == last {
                end = Some(i);
                break;
            }
            line += 1;
            if line == first {
                start = i + 1;
            }
        }
    }
    let end = end.unwrap_or_else(|| text.chars().count());
    (start.min(end), end)
}

/// The R pane: each Stata line's R under its Stata, its notes, or why it was
/// not translated. The line holding `cursor_line` (1-based) is marked; the
/// one clicked is returned.
pub fn translation_pane(
    ui: &mut egui::Ui,
    pal: &Palette,
    t: &Translation,
    cursor_line: usize,
) -> Option<usize> {
    let mut clicked = None;
    if !t.helpers.is_empty() {
        ui.label(
            RichText::new(format!(
                "(RCraft's helpers, {} of them, are defined first)",
                t.helpers.len()
            ))
            .italics()
            .small()
            .color(pal.dim),
        );
    }
    for (i, l) in t.lines.iter().enumerate() {
        let here = cursor_line >= l.span.line
            && cursor_line < l.span.line.saturating_add(l.span.lines.max(1));
        let fill = if here {
            pal.accent.gamma_multiply(0.18)
        } else {
            egui::Color32::TRANSPARENT
        };
        let frame = egui::Frame::none()
            .fill(fill)
            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
            .rounding(3.0);
        let response = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let stata = RichText::new(format!(
                    "{:>4}  {}",
                    l.span.line,
                    l.stata.replace('\n', " ")
                ))
                .monospace()
                .small()
                .color(pal.dim);
                ui.add(egui::Label::new(stata).wrap_mode(egui::TextWrapMode::Extend));
                match &l.result {
                    Ok(r) => {
                        for code in r.code.lines() {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(code).monospace().color(pal.command),
                                )
                                .wrap_mode(egui::TextWrapMode::Extend),
                            );
                        }
                        if r.code.is_empty() {
                            ui.label(
                                RichText::new("(nothing to do in R)")
                                    .italics()
                                    .small()
                                    .color(pal.dim),
                            );
                        }
                        for c in &r.caveats {
                            ui.label(
                                RichText::new(format!("note: {c}"))
                                    .small()
                                    .color(pal.warning),
                            );
                        }
                    }
                    Err(e) => {
                        ui.label(
                            RichText::new(format!("not translated: {}", e.reason))
                                .small()
                                .color(pal.error),
                        );
                    }
                }
            })
            .response
            .interact(egui::Sense::click());
        if response.clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_helpers_fold_to_one_line_in_results() {
        let code = format!(
            "{HELPERS_BEGIN} stata_fmt\nstata_fmt <- function(x) {{\n  x\n}}\n{HELPERS_END}\n# summarize y\nstata_summarize(d)\n"
        );
        assert_eq!(
            shown_lines(&code),
            vec![
                format!("{HELPERS_BEGIN} stata_fmt (3 lines, in the session script)"),
                "# summarize y".to_string(),
                "stata_summarize(d)".to_string(),
            ]
        );
        assert_eq!(shown_lines("x <- 1\n"), vec!["x <- 1".to_string()]);
    }

    #[test]
    fn a_commands_lines_are_found_and_selected() {
        let text = "use auto\nsummarize price ///\n  mpg\nregress price mpg";
        assert_eq!(line_of(text, 0), 0);
        assert_eq!(line_of(text, 10), 1);
        // line 2, two lines long: "summarize price ///\n  mpg"
        let (a, b) = char_range(text, 2, 2);
        let picked: String = text.chars().skip(a).take(b - a).collect();
        assert_eq!(picked, "summarize price ///\n  mpg");
        let (a, b) = char_range(text, 4, 1);
        let picked: String = text.chars().skip(a).take(b - a).collect();
        assert_eq!(picked, "regress price mpg");
        assert_eq!(char_range("", 3, 1), (0, 0));
    }

    #[test]
    fn page_up_recalls_the_stata_typed() {
        let mut s = StataMode::default();
        s.typed("use auto");
        s.typed("summarize price");
        s.typed("summarize price");
        assert_eq!(s.previous().as_deref(), Some("summarize price"));
        assert_eq!(s.previous().as_deref(), Some("use auto"));
        assert_eq!(s.previous(), None);
        assert_eq!(s.next().as_deref(), Some("summarize price"));
        assert_eq!(s.next().as_deref(), Some(""));
    }

    #[test]
    fn tab_completes_from_the_session() {
        let mut s = StataMode::default();
        s.live.step("use auto");
        s.live.step("local controls mpg");
        let vars: Vec<String> = ["price", "mpg", "make"]
            .iter()
            .map(|v| v.to_string())
            .collect();
        assert_eq!(
            s.complete("summarize pr", &vars, None),
            Some(("summarize price".to_string(), Vec::new()))
        );
        assert_eq!(
            s.complete("regress price `c", &vars, None),
            Some(("regress price `controls'".to_string(), Vec::new()))
        );
        let dir = std::env::temp_dir().join(format!("rcraft-stata-tab-{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("data"));
        let _ = std::fs::write(dir.join("auto.dta"), "");
        assert_eq!(
            s.complete("use au", &vars, Some(&dir)),
            Some(("use auto.dta".to_string(), Vec::new()))
        );
        assert_eq!(
            s.complete("use da", &vars, Some(&dir)),
            Some(("use data/".to_string(), Vec::new()))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
