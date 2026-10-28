//! Draws any catalog dialog from its [`Spec`]: one row per widget, the R it
//! will run, and Run. The dialog never writes R itself; the spec does.

use eframe::egui::{self, RichText};
use rcraft_engine::Engine;
use rcraft_rcodegen::dialog::{Context, Frame, Spec, Style, TextRule, Values, Widget};
use rcraft_rdata::Table;

use crate::look::Palette;
use crate::packages::PackageCheck;

/// What Run asks for.
pub struct RunRequest {
    /// The menu path, for the history and log (`"Data > Sort rows"`).
    pub menu: String,
    pub code: String,
    /// The data frame the code writes.
    pub result: String,
    /// Whether that is the data frame the dialog worked on.
    pub replaces: bool,
    /// The data frame (or object) the dialog worked on.
    pub dataset: String,
}

/// What the dialog asks the window to do.
pub enum SpecAction {
    Run(RunRequest),
    /// Run this R code to install the packages the code style needs, as a menu
    /// command; the dialog stays open and checks again when R is free.
    Install {
        code: String,
    },
}

struct Open {
    spec: &'static Spec,
    ctx: Context,
    values: Values,
    style: Style,
}

#[derive(Default)]
pub struct SpecDialog {
    open: Option<Open>,
    packages: PackageCheck,
}

/// The columns of `table` as a dialog sees them.
pub fn context(dataset: &str, table: &Table, frames: Vec<Frame>) -> Context {
    Context {
        dataset: dataset.to_string(),
        frames,
        columns: columns_of(table),
    }
}

/// The columns of `table` as a dialog sees them.
pub fn columns_of(table: &Table) -> Vec<rcraft_rcodegen::dialog::ColumnInfo> {
    table
        .columns
        .iter()
        .map(|c| rcraft_rcodegen::dialog::ColumnInfo {
            name: c.name.clone(),
            kind: rcraft_rcodegen::dialog::VarKind::of(&c.data),
        })
        .collect()
}

impl SpecDialog {
    pub fn open(&mut self, spec: &'static Spec, ctx: Context) {
        let values = spec.defaults(&ctx);
        let style = spec.styles.first().copied().unwrap_or_default();
        self.packages.reset();
        self.open = Some(Open {
            spec,
            ctx,
            values,
            style,
        });
    }

    #[cfg(test)]
    fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The data frame the open dialog works on.
    pub fn dataset(&self) -> Option<&str> {
        self.open.as_ref().map(|o| o.ctx.dataset.as_str())
    }

    /// The other data frames the open dialog has chosen, so the window can bring
    /// their columns over.
    pub fn chosen_frames(&self) -> Vec<String> {
        let Some(open) = &self.open else {
            return Vec::new();
        };
        open.spec
            .widgets
            .iter()
            .filter(|w| matches!(w, Widget::Frame { .. }))
            .map(|w| open.values.text(w.id()).to_string())
            .filter(|n| !n.is_empty())
            .collect()
    }

    /// Takes in the data frame's columns again (a command may have changed
    /// them while the dialog was open); choices stay as they are.
    pub fn refresh(&mut self, ctx: Context) {
        if let Some(open) = &mut self.open {
            if open.ctx.dataset == ctx.dataset {
                open.ctx = ctx;
            }
        }
    }

    /// Sets one widget's text, as if typed or chosen. For a multi-variable
    /// widget the text is the column names separated by commas.
    pub fn set_text(&mut self, id: &str, value: &str) {
        if let Some(open) = &mut self.open {
            let many = open.spec.widgets.iter().any(|w| {
                matches!(w, Widget::Variables { id: wid, .. }
                        | Widget::FrameVariables { id: wid, .. } if *wid == id)
            });
            if many {
                let names = value
                    .split(',')
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(str::to_string)
                    .collect();
                open.values.set_list(id, names);
            } else {
                open.values.set_text(id, value);
            }
        }
    }

    /// The code Run would hand over now, or why there is none yet.
    #[cfg(test)]
    pub(crate) fn code(&self) -> Option<Result<String, Vec<rcraft_rcodegen::dialog::Problem>>> {
        self.open
            .as_ref()
            .map(|o| o.spec.generate(&o.ctx, &o.values, o.style))
    }

    #[cfg(test)]
    fn values_mut(&mut self) -> Option<&mut Values> {
        self.open.as_mut().map(|o| &mut o.values)
    }

    /// Draws the dialog if it is open. With an `engine`, R is asked whether the
    /// packages the chosen code style needs are installed, and Run waits for it.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        engine: Option<&mut Engine>,
    ) -> Option<SpecAction> {
        let SpecDialog { open, packages } = self;
        let open = open.as_mut()?;
        let checked = engine.is_some();
        if let Some(engine) = engine {
            packages.service(engine, &open.spec.packages(open.style));
        }
        if packages.is_waiting() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        let mut keep = true;
        let mut run = None;
        let mut install = None;
        let mut cancel = false;
        egui::Window::new(open.spec.entry)
            .open(&mut keep)
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .default_pos(egui::pos2(280.0, 60.0))
            .show(ctx, |ui| {
                ui.label(RichText::new(open.spec.about).color(pal.dim));
                let subject = if rcraft_catalog::data::is_object_menu(open.spec.menu) {
                    "Object"
                } else {
                    "Data frame"
                };
                ui.label(
                    RichText::new(format!("{subject}: {}", open.ctx.dataset))
                        .small()
                        .color(pal.dim),
                );
                ui.add_space(6.0);
                let problems = open.spec.problems(&open.ctx, &open.values);
                egui::Grid::new("spec-widgets")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        for w in open.spec.widgets {
                            widget_row(ui, pal, open, w, &problems);
                        }
                    });
                if open.spec.styles.len() > 1 {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Code style");
                        egui::ComboBox::from_id_salt("spec-style")
                            .selected_text(open.style.label())
                            .show_ui(ui, |ui| {
                                for s in open.spec.styles {
                                    ui.selectable_value(&mut open.style, *s, s.label());
                                }
                            });
                        let packages = open.spec.packages(open.style);
                        if !packages.is_empty() {
                            ui.label(
                                RichText::new(format!("needs {}", packages.join(", ")))
                                    .small()
                                    .color(pal.dim),
                            );
                        }
                    });
                }
                let needed = open.spec.packages(open.style);
                if open.spec.styles.len() <= 1 && !needed.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!("needs the R package {}", needed.join(", ")))
                            .small()
                            .color(pal.dim),
                    );
                }
                if checked {
                    if let Some(code) = packages.panel(ui, pal, &needed) {
                        install = Some(code);
                    }
                }
                let can_run = !checked || packages.ready(&needed);
                ui.add_space(6.0);
                ui.label(RichText::new("R that will run").strong().color(pal.dim));
                let code = open.spec.generate(&open.ctx, &open.values, open.style);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match &code {
                        Ok(code) => {
                            ui.label(RichText::new(code).monospace());
                        }
                        Err(_) => {
                            ui.label(
                                RichText::new("Shown here once every choice is made.")
                                    .color(pal.dim),
                            );
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(code.is_ok() && can_run, egui::Button::new("Run"))
                        .clicked()
                    {
                        if let Ok(code) = &code {
                            let result = open.values.text("result").trim().to_string();
                            let result = if result.is_empty() {
                                open.ctx.dataset.clone()
                            } else {
                                result
                            };
                            run = Some(RunRequest {
                                menu: open.spec.menu.to_string(),
                                code: code.clone(),
                                replaces: result == open.ctx.dataset,
                                result,
                                dataset: open.ctx.dataset.clone(),
                            });
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if !keep || cancel || run.is_some() {
            self.open = None;
        }
        run.map(SpecAction::Run)
            .or_else(|| install.map(|code| SpecAction::Install { code }))
    }
}

/// The file ending a save dialog is set to write, from its format or separator
/// choice (or, for the two without one, the entry itself).
fn suggested_extension(spec: &Spec, values: &Values) -> &'static str {
    match (values.text("format"), values.text("sep")) {
        ("rdata", _) => "RData",
        ("dta", _) => "dta",
        ("sav", _) => "sav",
        ("zsav", _) => "zsav",
        ("xpt", _) => "xpt",
        ("rds", _) => "rds",
        ("png", _) => "png",
        ("svg", _) => "svg",
        ("pdf", _) => "pdf",
        ("parquet", _) => "parquet",
        ("feather", _) => "feather",
        (_, "tab") => "tsv",
        (_, "comma" | "semicolon") => "csv",
        _ if spec.entry == "Excel workbook" => "xlsx",
        _ => "RData",
    }
}

/// Asks the system's save dialog where to write, and puts the path in the box.
/// The system dialog has already asked about replacing a file that is there,
/// so the dialog's own Replace box is ticked for it.
fn browse_save(open: &mut Open, id: &str) {
    let ext = suggested_extension(open.spec, &open.values);
    let mut chooser = rfd::FileDialog::new()
        .set_title("Save as")
        .set_file_name(format!("{}.{ext}", open.ctx.dataset))
        .add_filter(format!(".{ext} file"), &[ext])
        .add_filter("All files", &["*"]);
    let typed = open.values.text(id).trim().to_string();
    if let Some(dir) = std::path::Path::new(&typed).parent().filter(|d| d.is_dir()) {
        chooser = chooser.set_directory(dir);
    }
    if let Some(path) = chooser.save_file() {
        if path.exists() && open.spec.widgets.iter().any(|w| w.id() == "replace") {
            open.values.set_flag("replace", true);
        }
        open.values
            .set_text(id, path.to_string_lossy().into_owned());
    }
}

fn widget_row(
    ui: &mut egui::Ui,
    pal: &Palette,
    open: &mut Open,
    w: &Widget,
    problems: &[rcraft_rcodegen::dialog::Problem],
) {
    let (label, hint) = match *w {
        Widget::Variable { label, hint, .. }
        | Widget::Variables { label, hint, .. }
        | Widget::FrameVariables { label, hint, .. }
        | Widget::Text { label, hint, .. }
        | Widget::Choice { label, hint, .. }
        | Widget::Frame { label, hint, .. } => (label, hint),
        Widget::Check { hint, .. } => ("", hint),
    };
    ui.label(label);
    ui.vertical(|ui| {
        match *w {
            Widget::Variable { id, required, .. } => {
                let current = open.values.text(id).to_string();
                let shown = if current.is_empty() {
                    if required {
                        "(choose)".to_string()
                    } else {
                        "(none)".to_string()
                    }
                } else {
                    current.clone()
                };
                let choices: Vec<(String, &'static str)> = open
                    .spec
                    .choices(w, &open.ctx)
                    .into_iter()
                    .map(|c| (c.name.clone(), c.kind.label()))
                    .collect();
                egui::ComboBox::from_id_salt(("spec-var", id))
                    .selected_text(RichText::new(shown).monospace())
                    .width(240.0)
                    .show_ui(ui, |ui| {
                        if !required && ui.selectable_label(current.is_empty(), "(none)").clicked()
                        {
                            open.values.set_text(id, "");
                        }
                        for (name, kind) in &choices {
                            let text = RichText::new(format!("{name}   {kind}")).monospace();
                            if ui.selectable_label(&current == name, text).clicked() {
                                open.values.set_text(id, name.clone());
                            }
                        }
                    });
            }
            Widget::Variables { id, min, .. } => {
                let choices: Vec<(String, &'static str)> = open
                    .spec
                    .choices(w, &open.ctx)
                    .into_iter()
                    .map(|c| (c.name.clone(), c.kind.label()))
                    .collect();
                let chosen = open.values.list_mut(id);
                egui::ScrollArea::vertical()
                    .id_salt(("spec-vars", id))
                    .max_height(130.0)
                    .show(ui, |ui| {
                        for (name, kind) in &choices {
                            let mut on = chosen.contains(name);
                            let text = RichText::new(format!("{name}   {kind}")).monospace();
                            if ui.checkbox(&mut on, text).changed() {
                                if on {
                                    chosen.push(name.clone());
                                } else {
                                    chosen.retain(|n| n != name);
                                }
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.small_button("Select all").clicked() {
                        *chosen = choices.iter().map(|(n, _)| n.clone()).collect();
                    }
                    if ui.small_button("Clear").clicked() {
                        chosen.clear();
                    }
                    let note = if min == 0 {
                        format!("{} chosen (none: all of them)", chosen.len())
                    } else {
                        format!("{} chosen", chosen.len())
                    };
                    ui.label(RichText::new(note).small().color(pal.dim));
                });
            }
            Widget::FrameVariables { id, .. } => {
                let choices: Vec<(String, &'static str)> = open
                    .spec
                    .frame_choices(w, &open.ctx, &open.values)
                    .into_iter()
                    .map(|c| (c.name.clone(), c.kind.label()))
                    .collect();
                if choices.is_empty() {
                    ui.label(RichText::new("Choose the other data frame first.").color(pal.dim));
                } else {
                    let chosen = open.values.list_mut(id);
                    egui::ScrollArea::vertical()
                        .id_salt(("spec-frame-vars", id))
                        .max_height(130.0)
                        .show(ui, |ui| {
                            for (name, kind) in &choices {
                                let mut on = chosen.contains(name);
                                let text = RichText::new(format!("{name}   {kind}")).monospace();
                                if ui.checkbox(&mut on, text).changed() {
                                    if on {
                                        chosen.push(name.clone());
                                    } else {
                                        chosen.retain(|n| n != name);
                                    }
                                }
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui.small_button("Clear").clicked() {
                            chosen.clear();
                        }
                        ui.label(
                            RichText::new(format!("{} chosen", chosen.len()))
                                .small()
                                .color(pal.dim),
                        );
                    });
                }
            }
            Widget::Text {
                id,
                rule: TextRule::SavePath,
                ..
            } => {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(open.values.text_mut(id)).desired_width(260.0),
                    );
                    if ui.button("Browse…").clicked() {
                        browse_save(open, id);
                    }
                });
            }
            Widget::Text { id, .. } => {
                ui.add(egui::TextEdit::singleline(open.values.text_mut(id)).desired_width(320.0));
            }
            Widget::Choice { id, options, .. } => {
                let current = open.values.text(id).to_string();
                let shown = options
                    .iter()
                    .find(|o| o.value == current)
                    .map_or("", |o| o.label);
                egui::ComboBox::from_id_salt(("spec-choice", id))
                    .selected_text(shown)
                    .width(240.0)
                    .show_ui(ui, |ui| {
                        for o in options {
                            if ui.selectable_label(o.value == current, o.label).clicked() {
                                open.values.set_text(id, o.value);
                            }
                        }
                    });
            }
            Widget::Check { id, label, .. } => {
                ui.checkbox(open.values.flag_mut(id), label);
            }
            Widget::Frame { id, .. } => {
                let current = open.values.text(id).to_string();
                if open.ctx.frames.is_empty() {
                    ui.label(
                        RichText::new("There is no other data frame yet; open or make one first.")
                            .color(pal.warning),
                    );
                } else {
                    let shown = if current.is_empty() {
                        "(choose)"
                    } else {
                        current.as_str()
                    };
                    egui::ComboBox::from_id_salt(("spec-frame", id))
                        .selected_text(shown)
                        .width(240.0)
                        .show_ui(ui, |ui| {
                            for f in &open.ctx.frames {
                                if ui.selectable_label(f.name == current, &f.name).clicked() {
                                    open.values.set_text(id, &f.name);
                                }
                            }
                        });
                }
            }
        }
        if !hint.is_empty() {
            ui.label(RichText::new(hint).small().color(pal.dim));
        }
        for p in problems.iter().filter(|p| p.widget == w.id()) {
            ui.label(RichText::new(&p.message).small().color(pal.warning));
        }
    });
    ui.end_row();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_catalog::data::SORT;

    #[test]
    fn browse_suggests_the_ending_of_the_chosen_format() {
        use rcraft_catalog::save::{
            SAVE_EXCEL, SAVE_R, SAVE_STATISTICS, SAVE_TEXT, SAVE_WORKSPACE,
        };
        let with = |spec: &Spec, set: &[(&str, &str)]| {
            let mut v = spec.defaults(&Context {
                dataset: "d".into(),
                frames: Vec::new(),
                columns: Vec::new(),
            });
            for (id, t) in set {
                v.set_text(id, *t);
            }
            suggested_extension(spec, &v)
        };
        assert_eq!(with(&SAVE_TEXT, &[]), "csv");
        assert_eq!(with(&SAVE_TEXT, &[("sep", "tab")]), "tsv");
        assert_eq!(with(&SAVE_EXCEL, &[]), "xlsx");
        assert_eq!(with(&SAVE_STATISTICS, &[("format", "zsav")]), "zsav");
        assert_eq!(with(&SAVE_R, &[("format", "rdata")]), "RData");
        assert_eq!(with(&SAVE_WORKSPACE, &[]), "RData");
    }
    use rcraft_rcodegen::dialog::{ColumnInfo, VarKind};

    fn ctx() -> Context {
        Context {
            dataset: "d".into(),
            frames: Vec::new(),
            columns: vec![ColumnInfo {
                name: "x".into(),
                kind: VarKind::Number,
            }],
        }
    }

    /// Draws one frame of the dialog, pressing nothing.
    fn frame(dialog: &mut SpecDialog) -> Option<RunRequest> {
        let ctx = egui::Context::default();
        let pal = Palette::new(crate::look::Accent::default(), false);
        let mut out = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            out = match dialog.show(ctx, &pal, None) {
                Some(SpecAction::Run(run)) => Some(run),
                _ => None,
            };
        });
        out
    }

    #[test]
    fn a_spec_dialog_draws_and_stays_open_until_run_or_cancel() {
        let mut d = SpecDialog::default();
        assert!(frame(&mut d).is_none());
        d.open(&SORT, ctx());
        assert!(d.is_open());
        assert_eq!(d.dataset(), Some("d"));
        assert!(frame(&mut d).is_none());
        assert!(d.is_open(), "drawing alone does not close it");
        if let Some(v) = d.values_mut() {
            v.set_text("by1", "x");
        }
        assert!(frame(&mut d).is_none());
        assert!(d.is_open());
    }

    #[test]
    fn refresh_only_takes_columns_of_the_same_data_frame() {
        let mut d = SpecDialog::default();
        d.open(&SORT, ctx());
        let mut other = ctx();
        other.dataset = "e".into();
        other.columns.clear();
        d.refresh(other);
        assert_eq!(d.open.as_ref().map(|o| o.ctx.columns.len()), Some(1));
        let mut same = ctx();
        same.columns.clear();
        d.refresh(same);
        assert_eq!(d.open.as_ref().map(|o| o.ctx.columns.len()), Some(0));
    }
}
