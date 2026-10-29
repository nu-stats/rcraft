//! File > Use my file: choose a data file, see what it looks like, change how
//! it is read, and import it. Delimited text, Excel, Stata, SPSS, SAS, Parquet,
//! Feather and R data files are handled. The dialog shows the R code it will run (AGENTS.md:
//! the generated R is the product) and R's own preview of the first rows.

use std::path::Path;

use eframe::egui::{self, Key, RichText};
use rcraft_engine::{Engine, QueryId};
use rcraft_rcodegen::{
    package_check_code, parse_na_strings, sheets_code, ArrowFormat, ArrowImport, CsvImport,
    ExcelImport, FileKind, HavenImport, HavenKind, Import, Quote, RObjectImport, DATA_EXTENSIONS,
};
use rcraft_rdata::sniff::{sniff_file, Encoding, SniffError, Sniffed};

use crate::look::Palette;
use crate::packages::{install_code, library_dir};

/// Rows shown in R's preview.
const PREVIEW_ROWS: usize = 10;

/// What the dialog asks the window to do.
pub enum DialogAction {
    /// Run this R code; show the data frame `name` afterwards, if known.
    /// `file` is the file it reads, for the list of recent files.
    Import {
        code: String,
        name: Option<String>,
        file: String,
    },
    /// Run this R code to install a package, as a menu command.
    Install { code: String },
    /// Show the help page of this dialog.
    Help,
}

/// What is known about an answer from R.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum Reply<T> {
    #[default]
    NotAsked,
    Waiting(QueryId),
    Got(T),
}

#[derive(Default)]
pub struct ImportDialog {
    pub open: bool,
    path: String,
    /// The path that was last looked at.
    looked_at: String,
    kind: Option<FileKind>,
    import: Option<Import>,
    /// Delimited text only: what the start of the file looks like.
    sniffed: Option<Sniffed>,
    na_text: String,
    error: Option<String>,
    /// Whether the package the format needs can be loaded.
    package: Reply<bool>,
    /// Excel: the sheet names, or why they could not be read.
    sheets: Reply<Result<Vec<String>, String>>,
    /// R's printed preview, and the code it was made for.
    preview: Reply<(bool, String)>,
    previewed_code: String,
    /// Ask for the preview as soon as nothing stands in the way.
    want_preview: bool,
    /// A package install was started from here; look again when R is free.
    installing: bool,
    install_seen_busy: bool,
    /// What the buttons at the foot of the window need, set while drawing.
    footer: Option<Footer>,
    /// Files opened lately, newest first, and whether each is still there.
    recent: Vec<(String, bool)>,
}

struct Footer {
    code: String,
    name: Option<String>,
    file: String,
    problem: Option<&'static str>,
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

/// Words for the guess shown above a text file's choices.
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

    /// Sets the list of recent files the dialog offers (newest first), and
    /// looks which of them are still there.
    pub fn set_recent(&mut self, paths: &[String]) {
        self.recent = paths
            .iter()
            .map(|p| (p.clone(), Path::new(p).is_file()))
            .collect();
    }

    /// Looks at the file in the path box and sets the choices from it.
    fn look(&mut self) {
        self.looked_at = self.path.clone();
        self.kind = None;
        self.import = None;
        self.sniffed = None;
        self.error = None;
        self.package = Reply::NotAsked;
        self.sheets = Reply::NotAsked;
        self.preview = Reply::NotAsked;
        self.previewed_code.clear();
        self.want_preview = true;
        let path = self.path.trim().to_string();
        if path.is_empty() {
            return;
        }
        if !Path::new(&path).is_file() {
            self.error = Some("could not read the file: it was not found".to_string());
            return;
        }
        let kind = FileKind::from_path(&path);
        self.kind = Some(kind);
        self.import = match kind {
            FileKind::Delimited => self.look_at_text(&path),
            FileKind::Unknown => {
                // maybe text with an unusual ending
                let text = self.look_at_text(&path);
                if text.is_none() {
                    self.error
                        .get_or_insert_with(|| "RCraft does not know this kind of file.".into());
                }
                text
            }
            FileKind::Excel => Some(Import::Excel(ExcelImport::new(&path))),
            FileKind::Stata => Some(Import::Haven(HavenImport::new(&path, HavenKind::Stata))),
            FileKind::Spss => Some(Import::Haven(HavenImport::new(&path, HavenKind::Spss))),
            FileKind::SpssPortable => Some(Import::Haven(HavenImport::new(
                &path,
                HavenKind::SpssPortable,
            ))),
            FileKind::Sas => Some(Import::Haven(HavenImport::new(&path, HavenKind::Sas))),
            FileKind::SasTransport => Some(Import::Haven(HavenImport::new(
                &path,
                HavenKind::SasTransport,
            ))),
            FileKind::Rds => Some(Import::RObject(RObjectImport::new(&path, false))),
            FileKind::RData => Some(Import::RObject(RObjectImport::new(&path, true))),
            FileKind::Parquet => Some(Import::Arrow(ArrowImport::new(&path, ArrowFormat::Parquet))),
            FileKind::Feather => Some(Import::Arrow(ArrowImport::new(&path, ArrowFormat::Feather))),
        };
    }

    fn look_at_text(&mut self, path: &str) -> Option<Import> {
        match sniff_file(Path::new(path)) {
            Ok(s) => {
                let spec = CsvImport::from_sniffed(path, &s);
                self.na_text = spec.na_strings.join(", ");
                self.sniffed = Some(s);
                Some(Import::Csv(spec))
            }
            Err(e) => {
                let mut why = e.to_string();
                if matches!(e, SniffError::NotText(_)) {
                    why.push_str(
                        ". RCraft opens delimited text (.csv, .tsv, .txt), Excel, Stata, \
                         SPSS, SAS, Parquet, Feather and R data files.",
                    );
                }
                self.error = Some(why);
                None
            }
        }
    }

    /// Asks R for what the dialog needs and takes in its answers: whether the
    /// package is there, the sheet names, and the preview. Call it every frame
    /// while the dialog is open.
    pub fn service(&mut self, engine: &mut Engine) {
        if self.installing {
            if engine.is_busy() {
                self.install_seen_busy = true;
            } else if self.install_seen_busy {
                self.installing = false;
                self.install_seen_busy = false;
                self.package = Reply::NotAsked;
            }
        }
        let Some(import) = &self.import else {
            return;
        };
        // 1. the package, if the format needs one
        let needed = import.package();
        match (&self.package, needed) {
            (Reply::NotAsked, Some(pkg)) => {
                self.package = Reply::Waiting(engine.query(&package_check_code(pkg)));
            }
            (Reply::NotAsked, None) => self.package = Reply::Got(true),
            (Reply::Waiting(id), _) => {
                if let Some(r) = engine.query_result(*id) {
                    self.package = Reply::Got(r.ok && r.text.trim() == "yes");
                }
            }
            _ => {}
        }
        if self.package != Reply::Got(true) {
            return;
        }
        // 2. Excel: the sheets
        if let Import::Excel(x) = import {
            match &self.sheets {
                Reply::NotAsked => {
                    self.sheets = Reply::Waiting(engine.query(&sheets_code(&x.path)));
                }
                Reply::Waiting(id) => {
                    if let Some(r) = engine.query_result(*id) {
                        let text = r.text.clone();
                        self.sheets = Reply::Got(if r.ok && !text.starts_with("ERROR") {
                            Ok(text.lines().map(str::to_string).collect())
                        } else {
                            Err(text.trim_start_matches("ERROR").trim().to_string())
                        });
                    }
                }
                Reply::Got(_) => {}
            }
        }
        // 3. R's preview of the first rows
        match &self.preview {
            Reply::NotAsked if self.want_preview => {
                let code = import.preview_code(PREVIEW_ROWS);
                self.previewed_code.clone_from(&code);
                self.preview = Reply::Waiting(engine.query(&code));
                self.want_preview = false;
            }
            Reply::Waiting(id) => {
                if let Some(r) = engine.query_result(*id) {
                    self.preview = Reply::Got((r.ok, r.text.trim_end().to_string()));
                }
            }
            _ => {}
        }
    }

    /// Whether R is still answering one of the dialog's questions.
    pub fn is_waiting(&self) -> bool {
        matches!(self.package, Reply::Waiting(_))
            || matches!(self.sheets, Reply::Waiting(_))
            || matches!(self.preview, Reply::Waiting(_))
            || (self.want_preview && self.import.is_some() && self.package != Reply::Got(false))
    }

    /// Draws the dialog if it is open. `existing` are the data frames already
    /// in R, so the dialog can warn before one is replaced.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        existing: &[String],
        engine: Option<&mut Engine>,
    ) -> Option<DialogAction> {
        if !self.open {
            return None;
        }
        if let Some(engine) = engine {
            self.service(engine);
        }
        if self.is_waiting() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        let mut open = true;
        let mut action = None;
        let mut close = false;
        egui::Window::new("Use my file")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(700.0)
            .default_pos(egui::pos2(250.0, 36.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("import-scroll")
                    .auto_shrink([false, true])
                    .max_height((ctx.screen_rect().height() - 190.0).max(160.0))
                    .show(ui, |ui| {
                        self.body(ui, pal, existing, &mut action);
                    });
                ui.separator();
                self.footer_row(ui, pal, &mut action, &mut close);
            });
        if matches!(action, Some(DialogAction::Install { .. })) {
            self.installing = true;
            self.install_seen_busy = false;
            self.package = Reply::NotAsked;
        } else if action.is_some() || close {
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
    ) {
        ui.horizontal(|ui| {
            ui.label("File:");
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.path)
                    .hint_text("Type or paste a file path, or drop a file on the window")
                    .desired_width(ui.available_width() - 200.0),
            );
            let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            if ui.button("Browse…").clicked() {
                self.browse();
            }
            let look = ui.button("Look at file").clicked();
            if look || ((enter || edit.lost_focus()) && self.path != self.looked_at) {
                self.look();
            }
        });
        self.recent_list(ui, pal);
        if let Some(why) = &self.error {
            ui.add_space(4.0);
            ui.label(RichText::new(why).color(pal.error));
        }
        self.footer = None;
        let (Some(kind), Some(mut import)) = (self.kind, self.import.take()) else {
            return;
        };

        ui.add_space(4.0);
        ui.label(RichText::new(kind.label()).strong().color(pal.dim));
        if let Some(sniffed) = &self.sniffed {
            ui.label(RichText::new(describe(sniffed)).color(pal.dim));
            self.raw_lines(ui, pal);
        }
        ui.add_space(4.0);

        // a package the format needs
        let package_ready = self.package_panel(ui, pal, &import, action);

        if package_ready {
            match &mut import {
                Import::Csv(spec) => self.csv_options(ui, spec),
                Import::Excel(spec) => self.excel_options(ui, spec),
                Import::Haven(spec) => haven_options(ui, spec),
                Import::Arrow(_) => {
                    ui.label(
                        RichText::new(
                            "Parquet and Feather files keep the type of every column, so \
                             the file is read as it is.",
                        )
                        .color(pal.dim),
                    );
                }
                Import::RObject(spec) => {
                    if spec.workspace {
                        ui.label(
                            RichText::new(
                                "The objects in this file are loaded under the names they were saved with.",
                            )
                            .color(pal.dim),
                        );
                    }
                }
            }
            ui.add_space(6.0);
            let name_edit = import.name_mut().map(|n| {
                ui.horizontal(|ui| {
                    ui.label("Name for the data:");
                    ui.add(egui::TextEdit::singleline(n).desired_width(180.0));
                    n.clone()
                })
                .inner
            });
            if let Some(name) = name_edit {
                if existing.contains(&name) {
                    ui.label(
                        RichText::new("This replaces the data frame with this name.")
                            .small()
                            .color(pal.warning),
                    );
                }
            }
            self.preview_panel(ui, pal, &import);
            ui.add_space(6.0);
            ui.label(RichText::new("R code that will run").small().color(pal.dim));
            let code = import.code();
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(&code).monospace().small())
                        .wrap_mode(egui::TextWrapMode::Wrap),
                );
            });
            self.footer = Some(Footer {
                code,
                name: import.result_name().map(str::to_string),
                file: self.path.trim().to_string(),
                problem: import.problem(),
            });
        }
        self.import = Some(import);
    }

    /// The files opened lately; a click chooses one. Open while nothing is chosen.
    fn recent_list(&mut self, ui: &mut egui::Ui, pal: &Palette) {
        if self.recent.is_empty() {
            return;
        }
        let mut chosen: Option<String> = None;
        egui::CollapsingHeader::new(RichText::new("Recent files").small().color(pal.dim))
            .id_salt("import-recent")
            .default_open(self.path.trim().is_empty())
            .show(ui, |ui| {
                for (path, exists) in &self.recent {
                    let text = RichText::new(path).monospace().small();
                    let text = if *exists { text } else { text.color(pal.dim) };
                    let hint = if *exists {
                        "Choose this file"
                    } else {
                        "This file is not there any more"
                    };
                    if ui
                        .selectable_label(false, text)
                        .on_hover_text(hint)
                        .clicked()
                    {
                        chosen = Some(path.clone());
                    }
                }
            });
        if let Some(path) = chosen {
            self.choose(path);
        }
    }

    /// Asks the system's file chooser for a data file, and looks at the one
    /// picked. The window waits while the chooser is open.
    fn browse(&mut self) {
        let mut chooser = rfd::FileDialog::new()
            .set_title("Choose a data file")
            .add_filter("Data files", DATA_EXTENSIONS)
            .add_filter("All files", &["*"]);
        if let Some(dir) = start_folder(&self.path, &self.recent) {
            chooser = chooser.set_directory(dir);
        }
        if let Some(path) = chooser.pick_file() {
            self.choose(path.to_string_lossy().into_owned());
        }
    }

    /// Takes `path` as the file and looks at it.
    fn choose(&mut self, path: String) {
        self.path = path;
        self.look();
    }

    /// The buttons at the foot of the window, kept in view while the rest scrolls.
    fn footer_row(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        action: &mut Option<DialogAction>,
        close: &mut bool,
    ) {
        if let Some(footer) = &self.footer {
            if let Some(why) = footer.problem {
                ui.label(RichText::new(why).color(pal.error));
            }
        }
        ui.horizontal(|ui| {
            if let Some(footer) = &self.footer {
                if ui
                    .add_enabled(footer.problem.is_none(), egui::Button::new("Import"))
                    .clicked()
                {
                    *action = Some(DialogAction::Import {
                        code: footer.code.clone(),
                        name: footer.name.clone(),
                        file: footer.file.clone(),
                    });
                }
            }
            if ui.button("Help").clicked() {
                *action = Some(DialogAction::Help);
            }
            if ui.button("Cancel").clicked() {
                *close = true;
            }
        });
    }

    /// The first lines of a text file, as they are in the file.
    fn raw_lines(&self, ui: &mut egui::Ui, pal: &Palette) {
        let Some(sniffed) = &self.sniffed else {
            return;
        };
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt("import-raw")
                .max_height(130.0)
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
    }

    /// Tells the user when the format needs a package that is not installed,
    /// and offers to install it. Returns true when the import can go ahead.
    fn package_panel(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        import: &Import,
        action: &mut Option<DialogAction>,
    ) -> bool {
        let Some(pkg) = import.package() else {
            return true;
        };
        match &self.package {
            Reply::Got(true) => true,
            Reply::Got(false) => {
                ui.label(
                    RichText::new(format!(
                        "This kind of file needs the R package \"{pkg}\", which is not installed."
                    ))
                    .color(pal.warning),
                );
                let library = library_dir();
                let code = install_code(&[pkg], library.as_deref());
                ui.label(
                    RichText::new(
                        "RCraft can install it for you. It is downloaded from CRAN (with the \
                         packages it needs) into RCraft's own library, not into any other app's R:",
                    )
                    .small()
                    .color(pal.dim),
                );
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.add(
                        egui::Label::new(RichText::new(&code).monospace().small())
                            .wrap_mode(egui::TextWrapMode::Wrap),
                    );
                });
                ui.horizontal(|ui| {
                    if ui.button(format!("Install {pkg}")).clicked() {
                        *action = Some(DialogAction::Install { code: code.clone() });
                    }
                    if ui.button("Check again").clicked() {
                        self.package = Reply::NotAsked;
                    }
                });
                false
            }
            _ => {
                ui.label(RichText::new(format!("Checking for {pkg}…")).color(pal.dim));
                false
            }
        }
    }

    fn csv_options(&mut self, ui: &mut egui::Ui, spec: &mut CsvImport) {
        ui.add_space(2.0);
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
                    .selected_text(if spec.decimal == ',' {
                        "Comma (,)"
                    } else {
                        "Point (.)"
                    })
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
                    .on_hover_text(
                        "Text to read as missing, separated by commas. \
                         Leave an item empty for empty fields.",
                    )
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
    }

    fn excel_options(&mut self, ui: &mut egui::Ui, spec: &mut ExcelImport) {
        ui.add_space(2.0);
        egui::Grid::new("import-excel")
            .num_columns(4)
            .spacing([14.0, 6.0])
            .show(ui, |ui| {
                ui.label("Sheet");
                match &self.sheets {
                    Reply::Got(Ok(names)) => {
                        let shown = if spec.sheet.is_empty() {
                            names
                                .first()
                                .map_or("(first sheet)".to_string(), |n| format!("{n} (first)"))
                        } else {
                            spec.sheet.clone()
                        };
                        egui::ComboBox::from_id_salt("import-sheet")
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                for (i, name) in names.iter().enumerate() {
                                    let chosen = if i == 0 {
                                        spec.sheet.is_empty() || spec.sheet == *name
                                    } else {
                                        spec.sheet == *name
                                    };
                                    if ui.selectable_label(chosen, name).clicked() {
                                        spec.sheet =
                                            if i == 0 { String::new() } else { name.clone() };
                                    }
                                }
                            });
                    }
                    Reply::Got(Err(why)) => {
                        ui.label(RichText::new(why).small());
                    }
                    _ => {
                        ui.label("Reading the sheet names…");
                    }
                }
                ui.label("Cell range");
                ui.add(
                    egui::TextEdit::singleline(&mut spec.range)
                        .hint_text("whole sheet, or B2:F40")
                        .desired_width(130.0),
                );
                ui.end_row();

                ui.label("Skip rows at the top");
                ui.add_enabled(
                    spec.range.trim().is_empty(),
                    egui::DragValue::new(&mut spec.skip).range(0..=100_000),
                )
                .on_disabled_hover_text("A cell range replaces this.");
                ui.label("Missing values");
                let mut na = spec.na_strings.join(", ");
                if ui
                    .add(egui::TextEdit::singleline(&mut na).desired_width(130.0))
                    .changed()
                {
                    spec.na_strings = parse_na_strings(&na);
                }
                ui.end_row();

                ui.label("Types from the first");
                ui.add(
                    egui::DragValue::new(&mut spec.guess_max)
                        .range(1..=1_000_000)
                        .suffix(" rows"),
                );
                ui.end_row();
            });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut spec.header, "First row holds the variable names");
            ui.checkbox(&mut spec.trim_ws, "Trim spaces");
        });
    }

    /// R's own printed first rows, with a button to refresh them.
    fn preview_panel(&mut self, ui: &mut egui::Ui, pal: &Palette, import: &Import) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("How R reads it").small().color(pal.dim));
            let stale = matches!(self.preview, Reply::Got(_))
                && self.previewed_code != import.preview_code(PREVIEW_ROWS);
            if ui
                .add_enabled(
                    !matches!(self.preview, Reply::Waiting(_)) && import.problem().is_none(),
                    egui::Button::new("Preview"),
                )
                .clicked()
            {
                self.preview = Reply::NotAsked;
                self.want_preview = true;
            }
            if stale {
                ui.label(
                    RichText::new("(choices changed: press Preview)")
                        .small()
                        .color(pal.warning),
                );
            }
        });
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt("import-preview")
                .max_height(170.0)
                .auto_shrink([false, true])
                .show(ui, |ui| match &self.preview {
                    Reply::Got((true, text)) => {
                        ui.add(
                            egui::Label::new(RichText::new(text).monospace().small())
                                .wrap_mode(egui::TextWrapMode::Extend),
                        );
                    }
                    Reply::Got((false, why)) => {
                        ui.add(
                            egui::Label::new(RichText::new(why).small().color(pal.error))
                                .wrap_mode(egui::TextWrapMode::Wrap),
                        );
                    }
                    Reply::Waiting(_) => {
                        ui.label(RichText::new("Asking R…").small().color(pal.dim));
                    }
                    Reply::NotAsked => {
                        ui.label(RichText::new("Press Preview.").small().color(pal.dim));
                    }
                });
        });
    }
}

fn haven_options(ui: &mut egui::Ui, spec: &mut HavenImport) {
    ui.add_space(2.0);
    ui.checkbox(
        &mut spec.labels_as_factors,
        "Turn labelled variables into factors",
    )
    .on_hover_text(
        "Off keeps the value labels as attributes (shown in the Variables pane); \
         on makes a factor with the labels as levels.",
    );
    if spec.kind.has_encoding() {
        ui.horizontal(|ui| {
            ui.label("Text encoding");
            ui.add(
                egui::TextEdit::singleline(&mut spec.encoding)
                    .hint_text("automatic, or e.g. latin1")
                    .desired_width(160.0),
            );
        });
    }
    if spec.kind.has_user_na() {
        ui.checkbox(
            &mut spec.user_na,
            "Keep user-defined missing values as values",
        );
    }
}

/// The folder the file chooser opens in: where the file in the box is, else
/// where the newest recent file that is still there is.
fn start_folder(path: &str, recent: &[(String, bool)]) -> Option<std::path::PathBuf> {
    let folder_of = |p: &str| {
        Path::new(p.trim())
            .parent()
            .filter(|d| d.is_dir())
            .map(Path::to_path_buf)
    };
    folder_of(path).or_else(|| {
        recent
            .iter()
            .filter(|(_, there)| *there)
            .find_map(|(p, _)| folder_of(p))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rcraft-dialog-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(name);
        let _ = std::fs::write(&path, bytes);
        path
    }

    #[test]
    fn a_text_file_fills_in_the_choices() {
        let path = temp("people.csv", b"id;score\n1;2,5\n");
        let mut d = ImportDialog::default();
        d.open_with(&path);
        assert!(d.open && d.error.is_none());
        let spec = match &d.import {
            Some(Import::Csv(spec)) => Some(spec.clone()),
            _ => None,
        };
        assert!(spec.is_some(), "not a csv import: {:?}", d.import);
        let Some(spec) = spec else { return };
        assert_eq!((spec.delimiter, spec.decimal), (Some(';'), ','));
        assert_eq!(spec.name, "people");
        assert_eq!(d.na_text, ", NA");
    }

    #[test]
    fn each_kind_of_file_gets_its_import() {
        let cases = [
            ("a.xlsx", "readxl"),
            ("a.dta", "haven"),
            ("a.sav", "haven"),
            ("a.sas7bdat", "haven"),
            ("a.xpt", "haven"),
            ("a.parquet", "arrow"),
            ("a.feather", "arrow"),
        ];
        for (file, package) in cases {
            let path = temp(file, b"binary \0 stuff");
            let mut d = ImportDialog::default();
            d.open_with(&path);
            assert!(d.error.is_none(), "{file}: {:?}", d.error);
            assert_eq!(
                d.import.as_ref().and_then(Import::package),
                Some(package),
                "{file}"
            );
        }
        let rds = temp("o.rds", b"x\0");
        let mut d = ImportDialog::default();
        d.open_with(&rds);
        assert!(matches!(d.import, Some(Import::RObject(_))));
    }

    #[test]
    fn unsupported_and_bad_files_say_why() {
        let mut d = ImportDialog::default();
        d.open_with(Path::new("definitely/not/here.csv"));
        assert!(d.import.is_none());
        assert!(d.error.as_deref().is_some_and(|e| e.contains("not found")));

        let weird = temp("blob.bin", b"\0\x01\x02 binary");
        d.open_with(&weird);
        assert!(d.import.is_none() && d.error.is_some());
    }

    #[test]
    fn recent_files_are_offered_and_choosing_one_looks_at_it() {
        let there = temp("recent.csv", b"a,b\n1,2\n")
            .to_string_lossy()
            .into_owned();
        let mut d = ImportDialog::default();
        d.set_recent(&[there.clone(), "gone/file.csv".into()]);
        let exist: Vec<bool> = d.recent.iter().map(|(_, e)| *e).collect();
        assert_eq!(exist, [true, false]);

        d.choose(there.clone());
        assert_eq!(d.path, there);
        assert!(matches!(d.import, Some(Import::Csv(_))), "{:?}", d.error);
        d.choose("gone/file.csv".into());
        assert!(d.import.is_none() && d.error.as_deref().is_some_and(|e| e.contains("not found")));

        // the dialog draws with the list, open and closed, without a click
        d.open = true;
        let ctx = egui::Context::default();
        let pal = Palette::new(crate::look::Accent::default(), false);
        for path in ["", "gone/file.csv"] {
            d.path = path.to_string();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                assert!(d.show(ctx, &pal, &[], None).is_none());
            });
        }
        assert!(d.open);
    }

    #[test]
    fn the_chooser_opens_where_the_file_or_a_recent_one_is() {
        let file = temp("start.csv", b"a\n1\n");
        let dir = file.parent().map(Path::to_path_buf);
        let file = file.to_string_lossy().into_owned();
        assert_eq!(start_folder(&file, &[]), dir);
        // an empty or missing path falls back to the newest recent file that exists
        let recent = [("gone/dir/x.csv".to_string(), false), (file.clone(), true)];
        assert_eq!(start_folder("", &recent), dir);
        assert_eq!(start_folder("nowhere/at/all.csv", &recent), dir);
        assert_eq!(start_folder("", &[]), None);
        assert_eq!(start_folder("", &[("gone/x.csv".into(), false)]), None);
    }

    #[test]
    fn delimiters_have_words() {
        assert_eq!(delimiter_label(Some('\t')), "Tab");
        assert_eq!(delimiter_label(None), "None (one column)");
        assert_eq!(delimiter_label(Some('#')), "Comma (,)");
    }

    fn engine() -> Option<Engine> {
        use rcraft_rhost::{find_installs, Env, SessionConfig};
        let install = find_installs(&Env::from_process())
            .into_iter()
            .find(|i| i.is_supported())?;
        Engine::start(SessionConfig::new(install)).ok()
    }

    /// Runs the dialog's service loop until `done` or a minute has passed.
    fn serve_until(
        d: &mut ImportDialog,
        e: &mut Engine,
        done: impl Fn(&ImportDialog) -> bool,
    ) -> bool {
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_secs(60) {
            e.poll(std::time::Duration::from_millis(30));
            d.service(e);
            if done(d) {
                return true;
            }
        }
        false
    }

    #[test]
    fn the_preview_comes_from_r_for_text_and_for_stata_files() {
        let Some(mut e) = engine() else { return };
        let csv = temp("prev.csv", b"id,name\n1,Ann\n2,Bo\n");
        let mut d = ImportDialog::default();
        d.open_with(&csv);
        assert!(serve_until(&mut d, &mut e, |d| matches!(
            d.preview,
            Reply::Got(_)
        )));
        let Reply::Got((ok, text)) = &d.preview else {
            return;
        };
        assert!(
            *ok && text.contains("2 columns; first 2 rows") && text.contains("Ann"),
            "{text}"
        );
        assert!(!d.is_waiting());

        // a Stata file, written by haven when it is installed
        let dta = temp("prev.dta", b"");
        let code = format!(
            "haven::write_dta(data.frame(id = 1:3, x = c(1.5, NA, 3)), \"{}\")",
            dta.to_string_lossy().replace('\\', "/")
        );
        let id = e.query(&code);
        let started = std::time::Instant::now();
        while e.query_result(id).is_none() && started.elapsed().as_secs() < 60 {
            e.poll(std::time::Duration::from_millis(30));
        }
        if !e.query_result(id).is_some_and(|r| r.ok) {
            eprintln!("skipped: haven is not installed");
            return;
        }
        d.open_with(&dta);
        assert!(serve_until(&mut d, &mut e, |d| matches!(
            d.preview,
            Reply::Got(_)
        )));
        assert_eq!(d.package, Reply::Got(true));
        let Reply::Got((ok, text)) = &d.preview else {
            return;
        };
        assert!(*ok && text.contains("2 columns; first 3 rows"), "{text}");
    }

    #[test]
    fn a_missing_package_stops_the_import_until_it_is_installed() {
        let Some(mut e) = engine() else { return };
        let path = temp("pkg.xlsx", b"PK\x03\x04 not a workbook");
        let mut d = ImportDialog::default();
        d.open_with(&path);
        // pretend the package check found nothing, as on a machine without readxl
        d.package = Reply::Got(false);
        d.service(&mut e);
        assert_eq!(d.package, Reply::Got(false), "no question is asked again");
        assert!(
            !matches!(d.preview, Reply::Waiting(_)),
            "no preview without the package"
        );

        // an install in progress ends with a fresh check
        d.installing = true;
        d.install_seen_busy = false;
        let r = e.submit(rcraft_engine::Source::Typed, "Sys.sleep(0.3)");
        assert!(r.is_ok());
        d.service(&mut e);
        assert!(d.installing && d.install_seen_busy);
        assert!(serve_until(&mut d, &mut e, |d| !d.installing));
        assert!(matches!(d.package, Reply::Waiting(_) | Reply::Got(_)));
    }
}
