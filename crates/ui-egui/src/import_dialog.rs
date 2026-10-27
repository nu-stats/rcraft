//! File > Use my file: choose a delimited text file, see what it looks like,
//! change how it is read, and import it. The dialog shows the R code it will
//! run (AGENTS.md: the generated R is the product).

use std::path::Path;

use eframe::egui::{self, Key, RichText};
use rcraft_rcodegen::{parse_na_strings, CsvImport, Quote};
use rcraft_rdata::sniff::{sniff_file, Encoding, Sniffed};

use crate::look::Palette;

/// What the dialog asks the window to do.
pub enum DialogAction {
    /// Run this R code; show the data frame `name` afterwards.
    Import { code: String, name: String },
}

#[derive(Default)]
pub struct ImportDialog {
    pub open: bool,
    path: String,
    /// The path that was last looked at.
    looked_at: String,
    sniffed: Option<Sniffed>,
    spec: Option<CsvImport>,
    na_text: String,
    error: Option<String>,
}

const DELIMITERS: [(Option<char>, &str); 6] = [
    (Some(','), "Comma (,)"),
    (Some(';'), "Semicolon (;)"),
    (Some('\t'), "Tab"),
    (Some('|'), "Pipe (|)"),
    (Some(' '), "Space"),
    (None, "None (one column)"),
];

fn delimiter_label(d: Option<char>) -> &'static str {
    DELIMITERS
        .iter()
        .find(|(c, _)| *c == d)
        .map_or("Comma (,)", |(_, l)| l)
}

/// Words for the guess shown above the choices.
fn describe(s: &Sniffed) -> String {
    let how = match s.delimiter {
        Some(d) => format!(
            "{} separated",
            delimiter_label(Some(d)).split(' ').next().unwrap_or("")
        ),
        None => "a single column".to_string(),
    };
    format!(
        "Looks like: {how}, {}, {} columns, {}.",
        if s.header {
            "first row is names"
        } else {
            "no names row"
        },
        s.columns,
        s.encoding.label()
    )
}

impl ImportDialog {
    /// Opens the dialog on `path` and looks at it.
    pub fn open_with(&mut self, path: &Path) {
        self.open = true;
        self.path = path.to_string_lossy().into_owned();
        self.look();
    }

    /// Opens the dialog with nothing chosen yet.
    pub fn open_empty(&mut self) {
        self.open = true;
    }

    /// Looks at the file in the path box and sets the choices from it.
    fn look(&mut self) {
        self.looked_at = self.path.clone();
        self.sniffed = None;
        self.spec = None;
        self.error = None;
        let path = self.path.trim();
        if path.is_empty() {
            return;
        }
        match sniff_file(Path::new(path)) {
            Ok(s) => {
                let spec = CsvImport::from_sniffed(path, &s);
                self.na_text = spec.na_strings.join(", ");
                self.spec = Some(spec);
                self.sniffed = Some(s);
            }
            Err(e) => {
                let mut why = e.to_string();
                if matches!(e, rcraft_rdata::sniff::SniffError::NotText(_)) {
                    why.push_str(
                        ". Opening Excel, Stata, SPSS, SAS, R data and Parquet files is planned \
                         and needs packages; for now RCraft opens delimited text (.csv, .tsv, .txt).",
                    );
                }
                self.error = Some(why);
            }
        }
    }

    /// Draws the dialog if it is open. `existing` are the data frames already
    /// in R, so the dialog can warn before one is replaced.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        existing: &[String],
    ) -> Option<DialogAction> {
        if !self.open {
            return None;
        }
        let mut open = true;
        let mut action = None;
        let mut close = false;
        egui::Window::new("Use my file")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(660.0)
            .show(ctx, |ui| {
                self.body(ui, pal, existing, &mut action, &mut close);
            });
        if action.is_some() || close {
            open = false;
        }
        self.open = open;
        action
    }

    fn body(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        existing: &[String],
        action: &mut Option<DialogAction>,
        close: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.label("File:");
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.path)
                    .hint_text("Type or paste a file path, or drop a file on the window")
                    .desired_width(ui.available_width() - 110.0),
            );
            let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            let look = ui.button("Look at file").clicked();
            if look || ((enter || edit.lost_focus()) && self.path != self.looked_at) {
                self.look();
            }
        });
        if let Some(why) = &self.error {
            ui.add_space(4.0);
            ui.label(RichText::new(why).color(pal.error));
        }
        let (Some(sniffed), Some(spec)) = (&self.sniffed, &mut self.spec) else {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    *close = true;
                }
            });
            return;
        };

        ui.add_space(4.0);
        ui.label(RichText::new(describe(sniffed)).color(pal.dim));
        ui.add_space(2.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt("import-preview")
                .max_height(150.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for (i, line) in sniffed.preview.iter().enumerate() {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{:>2}  {line}", i + 1))
                                    .monospace()
                                    .small(),
                            )
                            .wrap_mode(egui::TextWrapMode::Extend),
                        );
                    }
                    if sniffed.more {
                        ui.label(RichText::new("… the file goes on").small().color(pal.dim));
                    }
                });
        });
        ui.add_space(6.0);

        egui::Grid::new("import-choices")
            .num_columns(4)
            .spacing([14.0, 6.0])
            .show(ui, |ui| {
                ui.label("Delimiter");
                egui::ComboBox::from_id_salt("import-delim")
                    .selected_text(delimiter_label(spec.delimiter))
                    .show_ui(ui, |ui| {
                        for (value, label) in DELIMITERS {
                            ui.selectable_value(&mut spec.delimiter, value, label);
                        }
                    });
                ui.label("Decimal mark");
                egui::ComboBox::from_id_salt("import-decimal")
                    .selected_text(if spec.decimal == ',' { "Comma (,)" } else { "Point (.)" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut spec.decimal, '.', "Point (.)");
                        ui.selectable_value(&mut spec.decimal, ',', "Comma (,)");
                    });
                ui.end_row();

                ui.label("Quotes");
                egui::ComboBox::from_id_salt("import-quote")
                    .selected_text(spec.quote.label())
                    .show_ui(ui, |ui| {
                        for q in Quote::ALL {
                            ui.selectable_value(&mut spec.quote, q, q.label());
                        }
                    });
                ui.label("Text encoding");
                egui::ComboBox::from_id_salt("import-encoding")
                    .selected_text(spec.encoding.label())
                    .show_ui(ui, |ui| {
                        for e in Encoding::ALL {
                            ui.selectable_value(&mut spec.encoding, e, e.label());
                        }
                    });
                ui.end_row();

                ui.label("Skip lines at the top");
                ui.add(egui::DragValue::new(&mut spec.skip).range(0..=100_000));
                ui.label("Missing values");
                if ui
                    .add(egui::TextEdit::singleline(&mut self.na_text).desired_width(150.0))
                    .on_hover_text("Text to read as missing, separated by commas. Leave an item empty for empty fields.")
                    .changed()
                {
                    spec.na_strings = parse_na_strings(&self.na_text);
                }
                ui.end_row();
            });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut spec.header, "First row holds the variable names");
            ui.checkbox(&mut spec.strip_white, "Trim spaces");
            ui.checkbox(&mut spec.strings_as_factors, "Text as factors");
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Name for the data:");
            ui.add(egui::TextEdit::singleline(&mut spec.name).desired_width(180.0));
            if existing.contains(&spec.name) {
                ui.label(
                    RichText::new("replaces the data frame with this name")
                        .small()
                        .color(pal.warning),
                );
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new("R code that will run").small().color(pal.dim));
        let code = spec.code();
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.add(
                egui::Label::new(RichText::new(&code).monospace().small())
                    .wrap_mode(egui::TextWrapMode::Wrap),
            );
        });
        ui.add_space(8.0);
        let problem = spec.problem();
        if let Some(why) = problem {
            ui.label(RichText::new(why).color(pal.error));
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(problem.is_none(), egui::Button::new("Import"))
                .clicked()
            {
                *action = Some(DialogAction::Import {
                    code: code.clone(),
                    name: spec.name.clone(),
                });
            }
            if ui.button("Cancel").clicked() {
                *close = true;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_csv(text: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rcraft-dialog-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("people.csv");
        let _ = std::fs::write(&path, text);
        path
    }

    #[test]
    fn opening_a_file_fills_in_the_choices() {
        let path = temp_csv("id;score\n1;2,5\n");
        let mut d = ImportDialog::default();
        d.open_with(&path);
        assert!(d.open);
        let spec = d.spec.as_ref();
        assert_eq!(spec.map(|s| s.delimiter), Some(Some(';')));
        assert_eq!(spec.map(|s| s.decimal), Some(','));
        assert_eq!(spec.map(|s| s.name.as_str()), Some("people"));
        assert_eq!(d.na_text, ", NA");
        assert!(d.error.is_none());
    }

    #[test]
    fn a_bad_path_or_a_binary_file_gives_a_reason_and_no_choices() {
        let mut d = ImportDialog::default();
        d.open_with(Path::new("definitely/not/here.csv"));
        assert!(d.spec.is_none());
        assert!(d
            .error
            .as_deref()
            .is_some_and(|e| e.contains("could not read")));

        let dir = std::env::temp_dir().join(format!("rcraft-dialog-bin-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("book.xlsx");
        let _ = std::fs::write(&path, b"PK\x03\x04 binary");
        d.open_with(&path);
        assert!(d.spec.is_none());
        assert!(d.error.as_deref().is_some_and(|e| e.contains("Excel")));
    }

    #[test]
    fn delimiters_have_words() {
        assert_eq!(delimiter_label(Some('\t')), "Tab");
        assert_eq!(delimiter_label(None), "None (one column)");
        assert_eq!(delimiter_label(Some('#')), "Comma (,)");
    }
}
