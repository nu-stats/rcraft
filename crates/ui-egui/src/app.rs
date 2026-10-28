//! The window: panes, menus and keys.

use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

use eframe::egui::{self, Align, Key, Layout, Modifiers, RichText, TextStyle};
use rcraft_engine::{Engine, History, Item, Outcome, PackageIndexService, Recall, Source};
use rcraft_rcodegen::dialog::Spec;
use rcraft_rdata::display::LabelView;
use rcraft_rhost::recent::Recent;
use rcraft_rhost::{find_installs, Env, InterruptOutcome, SessionConfig, Stream, MIN_VERSION};

use crate::data_view::{self, ObjectClick, TypeFilter, VariableClick};
use crate::graph_view::GraphView;
use crate::import_dialog::{DialogAction, ImportDialog};
use crate::look::{self, Accent, Palette};
use crate::package_window::{PackagesWindow, Tab as PackagesTab};
use crate::spec_dialog::{self, SpecAction, SpecDialog};

/// What the middle pane shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
    Results,
    Data,
    Graph,
}

/// What the left pane's top half lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SideTab {
    Variables,
    Objects,
}

const COMMAND_ID: &str = "command-line";
/// History lines drawn (the newest); the rest stay in the engine.
const HISTORY_SHOWN: usize = 1000;

const SCRIPT_START: &str =
    "# Type R code here.\n# Ctrl+Enter runs the selection, or the line the cursor is on.\n";

enum EngineState {
    Starting(Receiver<Result<Engine, String>>),
    Ready(Box<Engine>),
    Failed(String),
}

/// What a click or key asks for; collected while drawing, done afterwards,
/// so drawing never needs the engine mutably.
enum Action {
    Submit(Source, String),
    SetCommand(String),
    /// Add text (a variable name) to the Command line.
    Insert(String),
    SelectDataset(Option<String>),
    /// Show this data frame as soon as R lists it (after an import).
    ShowWhenListed(String),
    OpenImport,
    /// Open the Packages window on this tab.
    OpenPackages(PackagesTab),
    /// Open a catalog dialog on the data frame shown in Variables.
    OpenSpec(&'static Spec),
    /// Open one of the Objects pane's dialogs on this object.
    OpenObjectSpec(&'static Spec, String),
    /// Open Export plot on this plot (its file name; empty: the newest).
    OpenPlotExport(String),
    SetView(View),
    SetSideTab(SideTab),
    SetTypeFilter(TypeFilter),
    Interrupt,
    Restart,
    ClearResults,
    CopyLog,
    CopyScript,
    SetAccent(Accent),
    SetTheme(egui::ThemePreference),
    Quit,
}

pub struct App {
    engine: EngineState,
    command: String,
    recall: Recall,
    script: String,
    accent: Accent,
    theme: egui::ThemePreference,
    focus_command: bool,
    needs_restart: bool,
    message: Option<String>,
    view: View,
    /// The Graph tab: plots R has drawn, and which one is shown.
    graph: GraphView,
    /// The data frame shown in Variables and Data.
    dataset: Option<String>,
    /// Other data frames brought over only because a dialog chose them.
    dialog_watch: Vec<String>,
    /// Sideways scroll of the Data grid, handed to its header.
    grid_x: f32,
    /// Whether the Data grid shows value labels, codes or both.
    label_view: LabelView,
    side_tab: SideTab,
    type_filter: TypeFilter,
    import: ImportDialog,
    /// The open catalog dialog (Data menu), if any.
    spec_dialog: SpecDialog,
    /// A data frame to switch to once R lists it.
    show_when_listed: Option<String>,
    /// Files imported lately, kept between runs.
    recent: Recent,
    /// The CRAN package list, cached on this computer.
    package_index: PackageIndexService,
    packages_window: PackagesWindow,
}

impl App {
    pub fn new(ctx: &egui::Context) -> App {
        look::apply_static(ctx);
        let accent = Accent::default();
        look::apply_colors(ctx, accent);
        let theme = egui::ThemePreference::System;
        ctx.set_theme(theme);
        let recent = Recent::load(&Env::from_process());
        let mut import = ImportDialog::default();
        import.set_recent(recent.paths());
        App {
            engine: EngineState::Starting(start_engine()),
            command: String::new(),
            recall: Recall::new(),
            script: SCRIPT_START.to_string(),
            accent,
            theme,
            focus_command: true,
            needs_restart: false,
            message: None,
            view: View::Results,
            graph: GraphView::default(),
            dataset: None,
            dialog_watch: Vec::new(),
            grid_x: 0.0,
            label_view: LabelView::default(),
            side_tab: SideTab::Variables,
            type_filter: TypeFilter::default(),
            import,
            spec_dialog: SpecDialog::default(),
            show_when_listed: None,
            recent,
            package_index: PackageIndexService::for_machine(&Env::from_process()),
            packages_window: PackagesWindow::default(),
        }
    }

    /// Uses another list of recent files (pictures and tests keep their own,
    /// so they neither read nor change the user's).
    pub fn use_recent(&mut self, recent: Recent) {
        self.import.set_recent(recent.paths());
        self.recent = recent;
    }

    fn engine(&self) -> Option<&Engine> {
        match &self.engine {
            EngineState::Ready(e) => Some(&**e),
            _ => None,
        }
    }

    /// What a dialog working on the data frame `name` sees: its columns as last
    /// brought over, and the workspace's other data frames (with columns once
    /// a dialog has chosen one and it has arrived).
    fn dialog_context(&self, name: &str) -> Option<rcraft_rcodegen::dialog::Context> {
        let engine = self.engine()?;
        let table = engine.table(name)?;
        let frames = engine
            .workspace()
            .iter()
            .filter(|w| w.is_data_frame() && w.name != name)
            .map(|w| rcraft_rcodegen::dialog::Frame {
                name: w.name.clone(),
                columns: engine
                    .table(&w.name)
                    .map(spec_dialog::columns_of)
                    .unwrap_or_default(),
            })
            .collect();
        Some(spec_dialog::context(name, table, frames))
    }

    /// Brings over the other data frames an open dialog has chosen (to know
    /// their variables), and lets go of those it no longer needs.
    fn watch_chosen_frames(&mut self) {
        let chosen = self.spec_dialog.chosen_frames();
        let shown = self.dataset.clone();
        let EngineState::Ready(engine) = &mut self.engine else {
            return;
        };
        for name in &chosen {
            if Some(name) != shown.as_ref() && !self.dialog_watch.contains(name) {
                engine.watch(name);
                self.dialog_watch.push(name.clone());
            }
        }
        self.dialog_watch.retain(|name| {
            let keep = chosen.contains(name);
            if !keep && Some(name) != shown.as_ref() {
                engine.unwatch(name);
            }
            keep
        });
    }

    /// Takes in what R has produced and finishes starting up.
    fn poll(&mut self, ctx: &egui::Context) {
        if let EngineState::Starting(rx) = &self.engine {
            match rx.try_recv() {
                Ok(Ok(engine)) => self.engine = EngineState::Ready(Box::new(engine)),
                Ok(Err(why)) => self.engine = EngineState::Failed(why),
                Err(TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
                Err(TryRecvError::Disconnected) => {
                    self.engine = EngineState::Failed("R did not start".into());
                }
            }
        }
        if let EngineState::Ready(engine) = &mut self.engine {
            let polled = engine.poll(Duration::ZERO);
            if polled.changed {
                ctx.request_repaint();
            }
            if engine.is_working() {
                ctx.request_repaint_after(Duration::from_millis(33));
            }
        }
        self.follow_datasets();
    }

    /// Keeps the chosen data frame valid: picks the first one when none is
    /// chosen, and lets go of one that has gone from R.
    fn follow_datasets(&mut self) {
        let EngineState::Ready(engine) = &mut self.engine else {
            return;
        };
        if let Some(name) = &self.dataset {
            let gone = !engine.is_working()
                && !engine
                    .workspace()
                    .iter()
                    .any(|w| &w.name == name && w.is_data_frame());
            if gone {
                engine.unwatch(name);
                self.dataset = None;
            }
        }
        if let Some(wanted) = self.show_when_listed.clone() {
            let listed = engine
                .workspace()
                .iter()
                .any(|w| w.name == wanted && w.is_data_frame());
            if listed {
                if let Some(old) = self.dataset.take() {
                    engine.unwatch(&old);
                }
                engine.watch(&wanted);
                self.dataset = Some(wanted);
                self.view = View::Data;
                self.show_when_listed = None;
            } else if !engine.is_working() {
                // the import failed: the error is in Results
                self.show_when_listed = None;
                self.view = View::Results;
            }
        }
        if self.dataset.is_none() {
            if let Some(first) = engine.workspace().iter().find(|w| w.is_data_frame()) {
                let name = first.name.clone();
                engine.watch(&name);
                self.dataset = Some(name);
            }
        }
    }

    fn perform(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::Submit(source, code) => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    match engine.submit(source, &code) {
                        Ok(_) => self.message = None,
                        Err(e) => self.message = Some(e.to_string()),
                    }
                }
            }
            Action::SetCommand(text) => {
                self.command = text;
                self.recall.reset();
                self.focus_command = true;
                cursor_to_end(ctx, egui::Id::new(COMMAND_ID), &self.command);
            }
            Action::Insert(text) => {
                let mut command = std::mem::take(&mut self.command);
                if !command.is_empty() && !command.ends_with([' ', '(', ',']) {
                    command.push(' ');
                }
                command.push_str(&text);
                self.perform(ctx, Action::SetCommand(command));
            }
            Action::SelectDataset(choice) => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    if let Some(old) = &self.dataset {
                        engine.unwatch(old);
                    }
                    if let Some(new) = &choice {
                        engine.watch(new);
                    }
                }
                self.dataset = choice;
            }
            Action::SetView(view) => self.view = view,
            Action::SetSideTab(tab) => self.side_tab = tab,
            Action::SetTypeFilter(filter) => self.type_filter = filter,
            Action::OpenPackages(tab) => self.packages_window.open_on(tab),
            Action::OpenImport => {
                self.import.set_recent(self.recent.paths());
                self.import.open_empty();
            }
            Action::OpenSpec(spec) => {
                let table = self
                    .dataset
                    .as_deref()
                    .and_then(|name| self.dialog_context(name));
                match table {
                    Some(dialog_ctx) => self.spec_dialog.open(spec, dialog_ctx),
                    None => {
                        self.message = Some(format!("{}: open a data frame first.", spec.entry));
                    }
                }
            }
            Action::OpenObjectSpec(spec, name) => {
                // an object has no columns to choose from
                self.spec_dialog.open(
                    spec,
                    rcraft_rcodegen::dialog::Context {
                        dataset: name,
                        frames: Vec::new(),
                        columns: Vec::new(),
                    },
                );
            }
            Action::OpenPlotExport(plot) => {
                // a plot needs no data frame, so the dialog has no columns
                self.spec_dialog.open(
                    &rcraft_catalog::plot_export::EXPORT_PLOT,
                    rcraft_rcodegen::dialog::Context {
                        dataset: "(plot)".into(),
                        frames: Vec::new(),
                        columns: Vec::new(),
                    },
                );
                self.spec_dialog.set_text("plot", &plot);
            }
            Action::ShowWhenListed(name) => self.show_when_listed = Some(name),
            Action::Interrupt => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    if engine.interrupt() == InterruptOutcome::SessionStopped {
                        self.needs_restart = true;
                    }
                }
            }
            Action::Restart => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    match engine.restart() {
                        Ok(()) => {
                            self.needs_restart = false;
                            self.message = None;
                        }
                        Err(e) => self.message = Some(e.to_string()),
                    }
                }
            }
            Action::ClearResults => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    engine.clear_results();
                }
            }
            Action::CopyLog => {
                if let Some(engine) = self.engine() {
                    ctx.copy_text(engine.log_text().to_string());
                }
            }
            Action::CopyScript => {
                if let Some(engine) = self.engine() {
                    ctx.copy_text(engine.script_text());
                }
            }
            Action::SetAccent(accent) => {
                self.accent = accent;
                look::apply_colors(ctx, accent);
            }
            Action::SetTheme(theme) => {
                self.theme = theme;
                ctx.set_theme(theme);
            }
            Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl App {
    /// Draws one frame and carries out what was asked. The window and the
    /// offscreen picture (`headless.rs`) both call this.
    pub fn ui(&mut self, ctx: &egui::Context) {
        self.poll(ctx);
        let dark = ctx.style().visuals.dark_mode;
        let pal = Palette::new(self.accent, dark);
        let mut actions: Vec<Action> = Vec::new();
        let busy = self.engine().is_some_and(Engine::is_busy);

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            self.menu_bar(ui, &pal, &mut actions);
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| self.status_bar(ui, &pal));
        egui::TopBottomPanel::bottom("command")
            .resizable(true)
            .default_height(64.0)
            .min_height(54.0)
            .show(ctx, |ui| {
                self.command_pane(ui, &pal, busy, &mut actions);
            });
        egui::SidePanel::left("left")
            .resizable(true)
            .default_width(240.0)
            .width_range(160.0..=480.0)
            .show(ctx, |ui| {
                self.side_panel(ui, &pal, &mut actions);
            });
        egui::TopBottomPanel::top("script")
            .resizable(true)
            .default_height(190.0)
            .min_height(80.0)
            .show(ctx, |ui| {
                self.script_pane(ui, &pal, busy, &mut actions);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.results_pane(ui, &pal, &mut actions);
        });

        // a file dropped on the window opens the dialog on it
        let dropped = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone()));
        if let Some(path) = dropped {
            self.import.set_recent(self.recent.paths());
            self.import.open_with(&path);
        }
        let frames: Vec<String> = self
            .engine()
            .map(|e| {
                e.workspace()
                    .iter()
                    .filter(|w| w.is_data_frame())
                    .map(|w| w.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let engine = match &mut self.engine {
            EngineState::Ready(e) => Some(&mut **e),
            _ => None,
        };
        match self.import.show(ctx, &pal, &frames, engine) {
            Some(DialogAction::Import { code, name, file }) => {
                self.recent.add(&file);
                self.import.set_recent(self.recent.paths());
                actions.push(Action::Submit(
                    Source::Menu("File > Use my file".into()),
                    code,
                ));
                if let Some(name) = name {
                    actions.push(Action::ShowWhenListed(name));
                }
            }
            Some(DialogAction::Install { code }) => {
                // the install runs as an ordinary command, so its progress is
                // in Results and it is in the log; the dialog stays open and
                // checks for the package again when R is free
                actions.push(Action::Submit(
                    Source::Menu("Packages > Install".into()),
                    code,
                ));
            }
            None => {}
        }

        // the package list: what its worker has reported, then the window
        if self.package_index.poll() {
            ctx.request_repaint();
        }
        if self.package_index.is_busy() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        let engine = match &mut self.engine {
            EngineState::Ready(e) => Some(&mut **e),
            _ => None,
        };
        if let Some(run) = self
            .packages_window
            .show(ctx, &pal, &mut self.package_index, engine)
        {
            actions.push(Action::Submit(Source::Menu(run.menu.into()), run.code));
        }

        // a plot that has just been drawn is shown in the Graph tab
        if let EngineState::Ready(engine) = &self.engine {
            if self.graph.note_new(engine) {
                self.view = View::Graph;
            }
        }

        // a catalog dialog sees the data frame's columns as they are now
        let fresh = self
            .spec_dialog
            .dataset()
            .and_then(|name| self.dialog_context(name));
        if let Some(fresh) = fresh {
            self.spec_dialog.refresh(fresh);
        }
        self.watch_chosen_frames();
        let engine = match &mut self.engine {
            EngineState::Ready(e) => Some(&mut **e),
            _ => None,
        };
        match self.spec_dialog.show(ctx, &pal, engine) {
            Some(SpecAction::Run(run)) => {
                // a renamed object is followed only when it was the one shown;
                // otherwise the window would wait for a data frame that never comes
                let follow = if rcraft_catalog::data::is_object_menu(&run.menu) {
                    self.dataset.as_deref() == Some(run.dataset.as_str())
                } else {
                    true
                };
                actions.push(Action::Submit(Source::Menu(run.menu), run.code));
                if !run.replaces && follow {
                    actions.push(Action::ShowWhenListed(run.result));
                }
            }
            Some(SpecAction::Install { code }) => {
                actions.push(Action::Submit(
                    Source::Menu("Packages > Install".into()),
                    code,
                ));
            }
            None => {}
        }

        for action in actions {
            self.perform(ctx, action);
        }
    }
}

impl App {
    fn menu_bar(&self, ui: &mut egui::Ui, pal: &Palette, out: &mut Vec<Action>) {
        egui::menu::bar(ui, |ui| {
            ui.label(
                RichText::new("R")
                    .strong()
                    .color(pal.on_accent)
                    .background_color(pal.accent),
            );
            ui.label(RichText::new("RCraft").strong());
            ui.add_space(6.0);
            ui.menu_button("File", |ui| {
                if ui.button("Use my file…").clicked() {
                    out.push(Action::OpenImport);
                    ui.close_menu();
                }
                ui.add_enabled(false, egui::Button::new("Open script…"));
                ui.separator();
                let has_data = self
                    .dataset
                    .as_deref()
                    .is_some_and(|name| self.engine().is_some_and(|e| e.table(name).is_some()));
                spec_menu(ui, has_data, rcraft_catalog::save::SPECS, out);
                ui.separator();
                if ui.button("Quit").clicked() {
                    out.push(Action::Quit);
                    ui.close_menu();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Clear results").clicked() {
                    out.push(Action::ClearResults);
                    ui.close_menu();
                }
                if ui.button("Copy session log").clicked() {
                    out.push(Action::CopyLog);
                    ui.close_menu();
                }
                if ui.button("Copy session as script").clicked() {
                    out.push(Action::CopyScript);
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button("Accent", |ui| {
                    for accent in Accent::ALL {
                        let dot = RichText::new("●").color(accent.color(pal.dark));
                        let selected = self.accent == accent;
                        let label =
                            format!("{}{}", if selected { "✓ " } else { "   " }, accent.name());
                        ui.horizontal(|ui| {
                            ui.label(dot);
                            if ui.selectable_label(selected, label).clicked() {
                                out.push(Action::SetAccent(accent));
                                ui.close_menu();
                            }
                        });
                    }
                });
                ui.menu_button("Appearance", |ui| {
                    for (theme, name) in [
                        (egui::ThemePreference::System, "Match the system"),
                        (egui::ThemePreference::Light, "Light"),
                        (egui::ThemePreference::Dark, "Dark"),
                    ] {
                        if ui.selectable_label(self.theme == theme, name).clicked() {
                            out.push(Action::SetTheme(theme));
                            ui.close_menu();
                        }
                    }
                });
            });
            ui.menu_button("Data", |ui| {
                let has_data = self
                    .dataset
                    .as_deref()
                    .is_some_and(|name| self.engine().is_some_and(|e| e.table(name).is_some()));
                spec_menu(ui, has_data, rcraft_catalog::data::SPECS, out);
            });
            ui.menu_button("Statistics", |ui| {
                let has_data = self
                    .dataset
                    .as_deref()
                    .is_some_and(|name| self.engine().is_some_and(|e| e.table(name).is_some()));
                spec_menu(ui, has_data, rcraft_catalog::stats::SPECS, out);
            });
            ui.menu_button("Graphs", |ui| {
                let has_data = self
                    .dataset
                    .as_deref()
                    .is_some_and(|name| self.engine().is_some_and(|e| e.table(name).is_some()));
                spec_menu(ui, has_data, rcraft_catalog::graphs::SPECS, out);
                ui.separator();
                let has_plot = self
                    .engine()
                    .is_some_and(|e| !crate::graph_view::plots(e).is_empty());
                if ui
                    .add_enabled(has_plot, egui::Button::new("Export plot…"))
                    .on_disabled_hover_text("Draw a plot first")
                    .clicked()
                {
                    out.push(Action::OpenPlotExport(String::new()));
                    ui.close_menu();
                }
            });
            ui.menu_button("Packages", |ui| {
                if ui.button("Find and install…").clicked() {
                    out.push(Action::OpenPackages(PackagesTab::Find));
                    ui.close_menu();
                }
                if ui.button("Installed packages…").clicked() {
                    out.push(Action::OpenPackages(PackagesTab::Installed));
                    ui.close_menu();
                }
            });
            ui.menu_button("Session", |ui| {
                if ui.button("Stop the running command").clicked() {
                    out.push(Action::Interrupt);
                    ui.close_menu();
                }
                if ui.button("Restart R").clicked() {
                    out.push(Action::Restart);
                    ui.close_menu();
                }
            });
            ui.menu_button("Help", |ui| {
                ui.label(format!(
                    "{} {}",
                    rcraft_core::PRODUCT,
                    rcraft_core::version()
                ));
                ui.label("Runs your code in R. The window never runs R itself.");
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(engine) = self.engine() {
                    ui.label(RichText::new(short_version(engine.r_version())).color(pal.dim));
                }
            });
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui, pal: &Palette) {
        ui.horizontal(|ui| {
            let (text, color) = match &self.engine {
                EngineState::Starting(_) => ("Starting R…".to_string(), pal.dim),
                EngineState::Failed(_) => ("R is not available".to_string(), pal.error),
                EngineState::Ready(e) if self.needs_restart || !e.is_running() => {
                    ("R stopped. Session > Restart R".to_string(), pal.warning)
                }
                EngineState::Ready(e) if e.is_busy() => ("Running…".to_string(), pal.accent),
                EngineState::Ready(_) => ("R ready".to_string(), pal.accent),
            };
            ui.label(RichText::new(text).strong().color(color));
            ui.separator();
            let data_note = match (&self.dataset, self.engine()) {
                (Some(name), Some(engine)) => match engine.table(name) {
                    Some(t) => format!("{name}: {} rows × {} columns", t.nrow, t.columns.len()),
                    None => name.clone(),
                },
                _ => "No data in memory".to_string(),
            };
            ui.label(RichText::new(data_note).color(pal.dim));
            if let Some(message) = &self.message {
                ui.separator();
                ui.label(RichText::new(message).color(pal.error));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let folder = std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                ui.label(RichText::new(format!("Working folder: {folder}")).color(pal.dim));
            });
        });
    }

    fn side_panel(&self, ui: &mut egui::Ui, pal: &Palette, out: &mut Vec<Action>) {
        let total = ui.available_height();
        ui.horizontal(|ui| {
            for (tab, label) in [
                (SideTab::Variables, "Variables"),
                (SideTab::Objects, "Objects"),
            ] {
                let chosen = self.side_tab == tab;
                let text = RichText::new(label).strong();
                let text = if chosen {
                    text.color(pal.accent)
                } else {
                    text.color(pal.dim)
                };
                if ui.selectable_label(chosen, text).clicked() && !chosen {
                    out.push(Action::SetSideTab(tab));
                }
            }
        });
        let frames: Vec<String> = self
            .engine()
            .map(|e| {
                e.workspace()
                    .iter()
                    .filter(|w| w.is_data_frame())
                    .map(|w| w.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        if self.side_tab == SideTab::Variables && !frames.is_empty() {
            let shown = self.dataset.clone().unwrap_or_else(|| "(none)".into());
            egui::ComboBox::from_id_salt("dataset")
                .selected_text(RichText::new(shown).monospace())
                .width(ui.available_width() - 8.0)
                .show_ui(ui, |ui| {
                    for name in &frames {
                        let chosen = self.dataset.as_deref() == Some(name.as_str());
                        if ui
                            .selectable_label(chosen, RichText::new(name).monospace())
                            .clicked()
                            && !chosen
                        {
                            out.push(Action::SelectDataset(Some(name.clone())));
                        }
                    }
                });
            egui::ComboBox::from_id_salt("type-filter")
                .selected_text(self.type_filter.label())
                .width(ui.available_width() - 8.0)
                .show_ui(ui, |ui| {
                    for choice in TypeFilter::ALL {
                        let chosen = self.type_filter == choice;
                        if ui.selectable_label(chosen, choice.label()).clicked() && !chosen {
                            out.push(Action::SetTypeFilter(choice));
                        }
                    }
                });
        }
        egui::ScrollArea::vertical()
            .id_salt("variables")
            .max_height(total * 0.42)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);
                if self.side_tab == SideTab::Objects {
                    let items = self.engine().map_or(&[][..], |e| e.workspace());
                    let mut clicks = Vec::new();
                    data_view::object_list(ui, pal, items, self.dataset.as_deref(), &mut clicks);
                    for click in clicks {
                        match click {
                            ObjectClick::Open(name) => {
                                out.push(Action::SelectDataset(Some(name)));
                                out.push(Action::SetSideTab(SideTab::Variables));
                            }
                            ObjectClick::Insert(name) => out.push(Action::Insert(name)),
                            ObjectClick::Dialog(spec, name) => {
                                out.push(Action::OpenObjectSpec(spec, name));
                            }
                        }
                    }
                    return;
                }
                let table = self
                    .dataset
                    .as_deref()
                    .and_then(|name| self.engine().and_then(|e| e.table(name)));
                if let Some(table) = table {
                    let mut clicks = Vec::new();
                    data_view::variable_list(ui, pal, table, self.type_filter, &mut clicks);
                    data_view::conversion_report(ui, pal, table);
                    for VariableClick::Insert(name) in clicks {
                        out.push(Action::Insert(name));
                    }
                } else if let Some(why) = self
                    .dataset
                    .as_deref()
                    .and_then(|name| self.engine().and_then(|e| e.table_error(name)))
                {
                    ui.label(RichText::new(why).color(pal.error));
                } else if self.dataset.is_some() {
                    ui.label(RichText::new("Bringing the table over…").color(pal.dim));
                } else {
                    ui.label(
                        RichText::new("Create or open a data frame to see its variables.")
                            .color(pal.dim),
                    );
                }
            });
        ui.separator();
        let count = self.engine().map_or(0, |e| e.history().len());
        pane_title(ui, pal, &format!("History  ({count})"));
        egui::ScrollArea::vertical()
            .id_salt("history")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if let Some(engine) = self.engine() {
                    history_list(ui, pal, engine.history(), out);
                }
            });
    }

    fn script_pane(&mut self, ui: &mut egui::Ui, pal: &Palette, busy: bool, out: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            pane_title(ui, pal, "Script");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let run_all = ui.add_enabled(!busy, egui::Button::new("Run all"));
                if run_all.clicked() {
                    out.push(Action::Submit(Source::Script, self.script.clone()));
                }
            });
        });
        let output = egui::ScrollArea::vertical()
            .id_salt("script")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::TextEdit::multiline(&mut self.script)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(8)
                    .lock_focus(true)
                    .show(ui)
            })
            .inner;
        // Ctrl+Enter: run the selection, or the line the cursor is on
        let pressed = output.response.has_focus()
            && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter));
        if pressed && !busy {
            let chosen = output.cursor_range.and_then(|r| {
                selection_or_line(
                    &self.script,
                    r.primary.ccursor.index,
                    r.secondary.ccursor.index,
                )
            });
            if let Some(code) = chosen {
                out.push(Action::Submit(Source::Script, code));
            }
        }
    }

    fn results_pane(&mut self, ui: &mut egui::Ui, pal: &Palette, out: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            for (view, label) in [
                (View::Results, "Results"),
                (View::Data, "Data"),
                (View::Graph, "Graph"),
            ] {
                let chosen = self.view == view;
                let text = RichText::new(label).strong();
                let text = if chosen {
                    text.color(pal.accent)
                } else {
                    text.color(pal.dim)
                };
                if ui.selectable_label(chosen, text).clicked() && !chosen {
                    out.push(Action::SetView(view));
                }
            }
            if self.view == View::Data {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("label-view")
                        .selected_text(self.label_view.label())
                        .show_ui(ui, |ui| {
                            for choice in LabelView::ALL {
                                ui.selectable_value(&mut self.label_view, choice, choice.label());
                            }
                        });
                    ui.label(
                        RichText::new("Show labelled values as")
                            .small()
                            .color(pal.dim),
                    );
                });
            }
        });
        if self.view == View::Data {
            self.data_pane(ui, pal);
            return;
        }
        if self.view == View::Graph {
            match &self.engine {
                EngineState::Ready(engine) => {
                    if let Some(plot) = self.graph.show(ui, pal, engine) {
                        out.push(Action::OpenPlotExport(plot));
                    }
                }
                _ => {
                    ui.label(RichText::new("R is not running.").color(pal.dim));
                }
            }
            return;
        }
        egui::ScrollArea::both()
            .id_salt("results")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| match &self.engine {
                EngineState::Starting(_) => {
                    ui.label(RichText::new("Starting R…").color(pal.dim));
                }
                EngineState::Failed(why) => {
                    ui.label(RichText::new(why).color(pal.error));
                    ui.label(
                        RichText::new(format!(
                            "RCraft needs R {MIN_VERSION} or later. Install R, then restart RCraft."
                        ))
                        .color(pal.dim),
                    );
                }
                EngineState::Ready(engine) => results_list(ui, pal, engine),
            });
    }

    fn data_pane(&mut self, ui: &mut egui::Ui, pal: &Palette) {
        let Some(name) = self.dataset.clone() else {
            ui.label(
                RichText::new("No data frame yet. Create one, or open a data file.").color(pal.dim),
            );
            return;
        };
        let (table, error) = match &self.engine {
            EngineState::Ready(engine) => (engine.table(&name), engine.table_error(&name)),
            _ => (None, None),
        };
        match (table, error) {
            (Some(table), _) => data_view::grid(ui, pal, table, self.label_view, &mut self.grid_x),
            (None, Some(why)) => {
                ui.label(RichText::new(why).color(pal.error));
            }
            (None, None) => {
                ui.label(RichText::new("Bringing the table over…").color(pal.dim));
            }
        }
    }

    fn command_pane(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        busy: bool,
        out: &mut Vec<Action>,
    ) {
        ui.horizontal(|ui| {
            pane_title(ui, pal, "Command");
            ui.label(
                RichText::new("Page Up / Page Down: earlier commands")
                    .small()
                    .color(pal.dim),
            );
        });
        let id = egui::Id::new(COMMAND_ID);
        // the recall keys, taken before the text box can use them
        let has_focus = ui.memory(|m| m.has_focus(id));
        let mut recalled: Option<String> = None;
        if has_focus {
            if let EngineState::Ready(engine) = &self.engine {
                let back = ui.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::ArrowUp)
                        || i.consume_key(Modifiers::NONE, Key::PageUp)
                });
                let forward = ui.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::ArrowDown)
                        || i.consume_key(Modifiers::NONE, Key::PageDown)
                });
                if back {
                    recalled = self.recall.previous(engine.history(), &self.command);
                } else if forward {
                    recalled = self.recall.next(engine.history());
                }
            }
        }
        let mut submit = false;
        ui.horizontal(|ui| {
            let stop_width = 64.0;
            let run_width = 64.0;
            let field_width = (ui.available_width() - stop_width - run_width - 24.0).max(80.0);
            let field = egui::TextEdit::singleline(&mut self.command)
                .id(id)
                .font(TextStyle::Monospace)
                .hint_text("Type R code and press Enter")
                .desired_width(field_width);
            let response = ui.add(field);
            if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                submit = true;
                self.focus_command = true;
            }
            if ui
                .add_enabled(
                    !busy,
                    egui::Button::new(RichText::new("Run").strong().color(pal.on_accent))
                        .fill(pal.accent),
                )
                .clicked()
            {
                submit = true;
            }
            if ui.add_enabled(busy, egui::Button::new("Stop")).clicked() {
                out.push(Action::Interrupt);
            }
        });
        if let Some(text) = recalled {
            self.command = text;
            self.focus_command = true;
            cursor_to_end(ui.ctx(), id, &self.command);
        }
        if submit && !self.command.trim().is_empty() && !busy {
            let code = std::mem::take(&mut self.command);
            self.recall.reset();
            out.push(Action::Submit(Source::Typed, code));
        }
        if self.focus_command {
            ui.memory_mut(|m| m.request_focus(id));
            self.focus_command = false;
        }
    }
}

fn pane_title(ui: &mut egui::Ui, pal: &Palette, text: &str) {
    ui.label(RichText::new(text).strong().color(pal.dim));
}

fn history_list(ui: &mut egui::Ui, pal: &Palette, history: &History, out: &mut Vec<Action>) {
    let entries = history.entries();
    let start = entries.len().saturating_sub(HISTORY_SHOWN);
    for entry in entries.iter().skip(start) {
        let first = entry.code.lines().next().unwrap_or("");
        let more = if entry.code.lines().nth(1).is_some() {
            " …"
        } else {
            ""
        };
        let text = RichText::new(format!("{first}{more}")).monospace();
        let text = match entry.outcome {
            Some(Outcome::Ok) | None => text,
            Some(_) => text.color(pal.error),
        };
        let response = ui
            .selectable_label(false, text)
            .on_hover_text("Click to put this back in the Command line");
        if response.clicked() {
            out.push(Action::SetCommand(entry.code.trim_end().to_string()));
        }
    }
}

fn results_list(ui: &mut egui::Ui, pal: &Palette, engine: &Engine) {
    for block in engine.results().blocks() {
        if let Some(entry) = engine.history().get(block.command) {
            for (i, line) in entry.code.trim_end().lines().enumerate() {
                let prompt = if i == 0 { "> " } else { "+ " };
                mono(ui, &format!("{prompt}{line}"), pal.accent, true);
            }
        }
        for item in &block.items {
            match item {
                Item::Text { stream, text } => {
                    let color = if *stream == Stream::Stderr {
                        pal.dim
                    } else {
                        pal.text
                    };
                    mono(ui, text.trim_end_matches('\n'), color, false);
                }
                Item::Warning(message) => {
                    mono(ui, &format!("Warning: {message}"), pal.warning, false);
                }
                Item::Error { message, call } => {
                    let text = if call.is_empty() {
                        format!("Error: {message}")
                    } else {
                        format!("Error in {call} : {message}")
                    };
                    mono(ui, &text, pal.error, false);
                }
                Item::Notice(note) => {
                    ui.label(RichText::new(format!("({note})")).italics().color(pal.dim));
                }
                Item::Plot { .. } => {
                    ui.label(
                        RichText::new("(a plot was drawn; see the Graph tab)")
                            .italics()
                            .color(pal.dim),
                    );
                }
            }
        }
        ui.add_space(6.0);
    }
}

/// The submenu of a dialog whose menu path has three parts ("Data > Factor >
/// Reference level" is in Factor); `None` for a plain Data menu entry.
fn menu_group(spec: &'static Spec) -> Option<&'static str> {
    let mut path = spec.menu.split(" > ").skip(1);
    let group = path.next()?;
    path.next().map(|_| group)
}

/// A menu's dialogs in order; those with a three-part path are grouped into a
/// submenu named by its middle part ("Data > Factor > Reference level").
fn spec_menu(
    ui: &mut egui::Ui,
    has_data: bool,
    specs: &'static [&'static Spec],
    out: &mut Vec<Action>,
) {
    let mut opened: Vec<&str> = Vec::new();
    for spec in specs {
        match menu_group(spec) {
            None => spec_button(ui, has_data, spec, out),
            Some(group) if !opened.contains(&group) => {
                opened.push(group);
                ui.menu_button(group, |ui| {
                    for member in specs {
                        if menu_group(member) == Some(group) {
                            spec_button(ui, has_data, member, out);
                        }
                    }
                });
            }
            Some(_) => {}
        }
    }
}

fn spec_button(ui: &mut egui::Ui, has_data: bool, spec: &'static Spec, out: &mut Vec<Action>) {
    let button = ui
        .add_enabled(has_data, egui::Button::new(format!("{}…", spec.entry)))
        .on_disabled_hover_text("Open a data frame first");
    if button.clicked() {
        out.push(Action::OpenSpec(spec));
        ui.close_menu();
    }
}

fn mono(ui: &mut egui::Ui, text: &str, color: egui::Color32, strong: bool) {
    let mut rich = RichText::new(text).monospace().color(color);
    if strong {
        rich = rich.strong();
    }
    ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
}

/// "R version 4.4.3 (2025-02-28 ucrt)" as "R 4.4.3".
fn short_version(full: &str) -> String {
    let number = full
        .split_whitespace()
        .skip_while(|w| *w != "version")
        .nth(1)
        .unwrap_or("");
    if number.is_empty() {
        full.to_string()
    } else {
        format!("R {number}")
    }
}

/// The selected text, or the line the cursor is on if nothing is selected.
/// `a` and `b` are the two ends of the selection, in characters.
fn selection_or_line(text: &str, a: usize, b: usize) -> Option<String> {
    let (lo, hi) = (a.min(b), a.max(b));
    let picked: String = if lo != hi {
        text.chars().skip(lo).take(hi - lo).collect()
    } else {
        let before: String = text.chars().take(lo).collect();
        let line_index = before.matches('\n').count();
        text.lines().nth(line_index).unwrap_or("").to_string()
    };
    if picked.trim().is_empty() {
        None
    } else {
        Some(picked)
    }
}

/// Puts the text cursor at the end of the text in the box with `id`.
fn cursor_to_end(ctx: &egui::Context, id: egui::Id, text: &str) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let end = egui::text::CCursor::new(text.chars().count());
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(end)));
    state.store(ctx, id);
}

/// Finds R and starts the engine on another thread, so the window opens at
/// once and shows "Starting R…" meanwhile.
fn start_engine() -> Receiver<Result<Engine, String>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let result = (|| {
            let install = find_installs(&Env::from_process())
                .into_iter()
                .find(|i| i.is_supported())
                .ok_or_else(|| format!("No R found. RCraft needs R {MIN_VERSION} or later."))?;
            Engine::start(SessionConfig::new(install)).map_err(|e| e.to_string())
        })();
        let _ = tx.send(result);
    });
    rx
}

impl App {
    /// R is ready and nothing is running (or R could not start).
    pub fn is_settled(&self) -> bool {
        match &self.engine {
            EngineState::Ready(e) => !e.is_working(),
            EngineState::Failed(_) => true,
            EngineState::Starting(_) => false,
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.engine, EngineState::Ready(_))
    }

    pub fn set_theme(&mut self, ctx: &egui::Context, theme: egui::ThemePreference) {
        self.perform(ctx, Action::SetTheme(theme));
    }

    /// Opens "Use my file" on a file, as if it had been dropped on the window.
    pub fn open_import(&mut self, path: &std::path::Path) {
        self.import.open_with(path);
    }

    /// Opens the Packages window for a picture: over a package list read from
    /// `from` (a downloaded `packages.rds`) or a tiny made-up one, never over the
    /// user's cache or the network.
    pub fn show_packages(
        &mut self,
        tab: &str,
        query: &str,
        select: Option<&str>,
        confirm: bool,
        from: Option<&std::path::Path>,
    ) -> Result<(), String> {
        let tab = match tab.to_ascii_lowercase().as_str() {
            "find" => PackagesTab::Find,
            "installed" => PackagesTab::Installed,
            other => {
                return Err(format!(
                    "unknown Packages tab {other}; use find or installed"
                ))
            }
        };
        let index = match from {
            Some(path) => {
                let bytes = std::fs::read(path)
                    .map_err(|e| format!("could not read {}: {e}", path.display()))?;
                let node = rcraft_rpkg::rds::read(&bytes).map_err(|e| e.to_string())?;
                let table = rcraft_rpkg::table::from_node(&node, rcraft_rpkg::index::DB_COLUMNS)
                    .map_err(|e| e.to_string())?;
                rcraft_rpkg::index::Index::from_tables(
                    Some(&table),
                    None,
                    rcraft_engine::package_index::unix_now(),
                )
            }
            None => rcraft_rpkg::index::sample(),
        };
        self.package_index = PackageIndexService::in_memory(index);
        self.packages_window.open_on(tab);
        self.packages_window
            .preset(&self.package_index, query, select, confirm);
        Ok(())
    }

    /// Opens a Data menu dialog by its name (case does not matter) on the
    /// data frame shown in Variables, with some values already set.
    pub fn open_dialog(
        &mut self,
        ctx: &egui::Context,
        entry: &str,
        set: &[(String, String)],
    ) -> Result<(), String> {
        let spec = rcraft_catalog::data::SPECS
            .iter()
            .chain(rcraft_catalog::data::OBJECT_SPECS)
            .chain(rcraft_catalog::stats::SPECS)
            .chain(rcraft_catalog::graphs::SPECS)
            .chain(rcraft_catalog::postest::MODEL_SPECS)
            .chain(rcraft_catalog::save::SPECS)
            .chain(rcraft_catalog::plot_export::PLOT_SPECS)
            .find(|s| s.entry.eq_ignore_ascii_case(entry))
            .ok_or_else(|| format!("no dialog called {entry}"))?;
        if spec.entry == rcraft_catalog::plot_export::EXPORT_PLOT.entry {
            // a plot needs no data frame
            self.perform(ctx, Action::OpenPlotExport(String::new()));
        } else if rcraft_catalog::data::is_object_menu(spec.menu) {
            // the Objects pane's dialogs work on the object shown in Variables
            let name = self
                .dataset
                .clone()
                .ok_or_else(|| format!("{entry}: no data frame is shown in Variables"))?;
            self.perform(ctx, Action::OpenObjectSpec(spec, name));
        } else {
            self.perform(ctx, Action::OpenSpec(spec));
        }
        if self.spec_dialog.dataset().is_none() {
            return Err(format!("{entry}: no data frame is shown in Variables"));
        }
        for (id, value) in set {
            self.spec_dialog.set_text(id, value);
        }
        Ok(())
    }

    /// Switches the left pane to the Objects list.
    pub fn show_objects(&mut self, ctx: &egui::Context) {
        self.perform(ctx, Action::SetSideTab(SideTab::Objects));
    }

    /// Lists only the variables of one type, named as in the filter menu
    /// (`Factors`, `Numbers`, `Text`, ...; case does not matter).
    pub fn filter_variables(&mut self, ctx: &egui::Context, name: &str) -> Result<(), String> {
        let found = TypeFilter::ALL
            .into_iter()
            .find(|f| f.label().eq_ignore_ascii_case(name));
        match found {
            Some(filter) => {
                self.perform(ctx, Action::SetTypeFilter(filter));
                Ok(())
            }
            None => Err(format!("unknown type filter {name}")),
        }
    }

    /// Switches the middle pane to the Data tab.
    pub fn show_data(&mut self, ctx: &egui::Context) {
        self.perform(ctx, Action::SetView(View::Data));
    }

    pub fn set_accent(&mut self, ctx: &egui::Context, accent: Accent) {
        self.perform(ctx, Action::SetAccent(accent));
    }

    pub fn submit_typed(&mut self, code: &str) {
        if let EngineState::Ready(engine) = &mut self.engine {
            if let Err(e) = engine.submit(Source::Typed, code) {
                self.message = Some(e.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortens_the_version() {
        assert_eq!(
            short_version("R version 4.4.3 (2025-02-28 ucrt)"),
            "R 4.4.3"
        );
        assert_eq!(short_version("something else"), "something else");
    }

    #[test]
    fn runs_the_selection_when_there_is_one() {
        let text = "a <- 1\nb <- 2\nc <- 3";
        assert_eq!(selection_or_line(text, 7, 13), Some("b <- 2".into()));
        // either end first
        assert_eq!(selection_or_line(text, 13, 7), Some("b <- 2".into()));
    }

    #[test]
    fn runs_the_current_line_without_a_selection() {
        let text = "a <- 1\nb <- 2\nc <- 3";
        assert_eq!(selection_or_line(text, 0, 0), Some("a <- 1".into()));
        assert_eq!(selection_or_line(text, 9, 9), Some("b <- 2".into()));
        assert_eq!(selection_or_line(text, 20, 20), Some("c <- 3".into()));
    }

    #[test]
    fn blank_lines_and_empty_text_run_nothing() {
        assert_eq!(selection_or_line("a\n\nb", 2, 2), None);
        assert_eq!(selection_or_line("", 0, 0), None);
    }

    #[test]
    fn multibyte_text_is_cut_by_characters() {
        let text = "é <- 1\nü <- 2";
        assert_eq!(selection_or_line(text, 7, 13), Some("ü <- 2".into()));
    }

    /// Runs frames until `done` or a minute has passed.
    fn frames_until(app: &mut App, ctx: &egui::Context, done: impl Fn(&App) -> bool) -> bool {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let started = std::time::Instant::now();
        while started.elapsed() < Duration::from_secs(60) {
            let raw = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(started.elapsed().as_secs_f64()),
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| app.ui(ctx));
            if done(app) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    #[test]
    fn an_import_switches_to_the_new_data_frame_and_a_failed_one_does_not() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        let menu = || Source::Menu("File > Use my file".into());
        app.perform(
            &ctx,
            Action::Submit(menu(), "imported <- data.frame(a = 1:3)".into()),
        );
        app.perform(&ctx, Action::ShowWhenListed("imported".into()));
        assert!(frames_until(&mut app, &ctx, |a| {
            a.dataset.as_deref() == Some("imported")
                && a.engine().is_some_and(|e| e.table("imported").is_some())
        }));
        assert_eq!(app.view, View::Data);
        assert_eq!(app.show_when_listed, None);

        // a failing import: the wish is dropped and Results is shown
        app.perform(
            &ctx,
            Action::Submit(menu(), "broken <- read.table(\"no/such/file.csv\")".into()),
        );
        app.perform(&ctx, Action::ShowWhenListed("broken".into()));
        assert!(frames_until(&mut app, &ctx, |a| a
            .show_when_listed
            .is_none()
            && a.is_settled()));
        assert_eq!(app.view, View::Results);
        assert_eq!(app.dataset.as_deref(), Some("imported"));
    }

    #[test]
    fn the_objects_tab_lists_everything_and_the_tab_and_filter_are_remembered() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        app.perform(
            &ctx,
            Action::Submit(
                Source::Typed,
                "frame1 <- data.frame(a = 1:3); answer <- 42".into(),
            ),
        );
        assert!(frames_until(&mut app, &ctx, |a| {
            a.engine()
                .is_some_and(|e| e.workspace().iter().any(|w| w.name == "answer"))
        }));
        let items = app
            .engine()
            .map(|e| e.workspace().to_vec())
            .unwrap_or_default();
        let frame = items.iter().find(|w| w.name == "frame1");
        let other = items.iter().find(|w| w.name == "answer");
        assert_eq!(frame.and_then(|w| w.dims), Some((3, 1)));
        assert!(other.is_some_and(|w| !w.is_data_frame()));

        assert_eq!(app.side_tab, SideTab::Variables);
        app.perform(&ctx, Action::SetSideTab(SideTab::Objects));
        app.perform(&ctx, Action::SetTypeFilter(TypeFilter::Factors));
        assert_eq!(app.side_tab, SideTab::Objects);
        assert_eq!(app.type_filter, TypeFilter::Factors);
    }

    #[test]
    fn a_data_dialog_needs_a_data_frame_and_runs_its_code_as_a_menu_command() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        // nothing shown yet: the dialog does not open
        assert!(app.open_dialog(&ctx, "Sort rows", &[]).is_err());
        assert!(app.open_dialog(&ctx, "No such dialog", &[]).is_err());

        app.perform(
            &ctx,
            Action::Submit(Source::Typed, "d <- data.frame(x = c(3, 1, 2))".into()),
        );
        assert!(frames_until(&mut app, &ctx, |a| {
            a.dataset.as_deref() == Some("d") && a.engine().is_some_and(|e| e.table("d").is_some())
        }));
        let set = [
            ("by1".to_string(), "x".to_string()),
            ("result".to_string(), "sorted".to_string()),
        ];
        assert!(app.open_dialog(&ctx, "sort ROWS", &set).is_ok());
        assert_eq!(app.spec_dialog.dataset(), Some("d"));

        // what Run hands over, done the way the window does it
        let code = app.spec_dialog.code();
        assert_eq!(
            code,
            Some(Ok(
                "sorted <- d[order(d[[\"x\"]]), , drop = FALSE]".to_string()
            ))
        );
        let code = code.and_then(Result::ok);
        app.perform(
            &ctx,
            Action::Submit(
                Source::Menu("Data > Sort rows".into()),
                code.unwrap_or_default(),
            ),
        );
        app.perform(&ctx, Action::ShowWhenListed("sorted".into()));
        assert!(frames_until(&mut app, &ctx, |a| {
            a.dataset.as_deref() == Some("sorted")
                && a.engine().is_some_and(|e| e.table("sorted").is_some())
        }));
        let last = app
            .engine()
            .and_then(|e| e.history().entries().last().cloned());
        assert_eq!(
            last.map(|h| h.source),
            Some(Source::Menu("Data > Sort rows".into()))
        );
    }

    #[test]
    fn a_statistics_dialog_takes_several_variables_and_prints_in_results() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        app.perform(
            &ctx,
            Action::Submit(
                Source::Typed,
                "d <- data.frame(x = c(1, 2, 3, 4), y = c(2, 4, 5, 9))".into(),
            ),
        );
        assert!(frames_until(&mut app, &ctx, |a| {
            a.dataset.as_deref() == Some("d") && a.engine().is_some_and(|e| e.table("d").is_some())
        }));
        // several variables are set as one comma-separated list
        let set = [("vars".to_string(), "x, y".to_string())];
        assert!(app.open_dialog(&ctx, "correlation", &set).is_ok());
        let code = app.spec_dialog.code().and_then(Result::ok);
        let code = code.unwrap_or_default();
        assert!(code.ends_with("d[c(\"x\", \"y\")], \"pearson\")"), "{code}");
        app.perform(
            &ctx,
            Action::Submit(
                Source::Menu("Statistics > Descriptives > Correlation".into()),
                code,
            ),
        );
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        let printed = app.engine().map(|e| {
            e.results()
                .blocks()
                .iter()
                .flat_map(|b| b.items.iter())
                .filter_map(|i| match i {
                    Item::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<String>()
        });
        assert!(
            printed.is_some_and(|t| t.contains("Correlations") && t.contains("0.9")),
            "the table is in Results"
        );
        let last = app
            .engine()
            .and_then(|e| e.history().entries().last().cloned());
        assert_eq!(
            last.map(|h| h.source),
            Some(Source::Menu(
                "Statistics > Descriptives > Correlation".into()
            ))
        );
    }

    #[test]
    fn a_new_plot_opens_the_graph_tab_and_keeps_following_the_newest() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        assert_eq!(app.view, View::Results);
        app.perform(&ctx, Action::Submit(Source::Typed, "plot(1:5)".into()));
        assert!(frames_until(&mut app, &ctx, |a| a.view == View::Graph));
        let count = |a: &App| a.engine().map_or(0, |e| crate::graph_view::plots(e).len());
        assert_eq!(count(&app), 1);
        assert_eq!(app.graph.shown(), None, "the newest is followed");
        // the user goes back to Results; a command that draws nothing leaves them there
        app.perform(&ctx, Action::SetView(View::Results));
        app.perform(&ctx, Action::Submit(Source::Typed, "x <- 1".into()));
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        assert_eq!(app.view, View::Results);
        // the next plot brings the Graph tab back, with the added line in a new snapshot
        app.perform(&ctx, Action::Submit(Source::Typed, "abline(h = 3)".into()));
        assert!(frames_until(&mut app, &ctx, |a| a.view == View::Graph));
        assert_eq!(count(&app), 2);
    }

    #[test]
    fn export_plot_opens_without_a_data_frame_on_the_plot_shown() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        // no data frame is open, and a plot is all the dialog needs
        app.perform(&ctx, Action::Submit(Source::Typed, "plot(1:5)".into()));
        assert!(frames_until(&mut app, &ctx, |a| a.view == View::Graph));
        let name = app
            .engine()
            .and_then(|e| crate::graph_view::plots(e).pop())
            .and_then(|p| p.path.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_default();
        assert!(name.ends_with(".png"), "{name}");
        app.perform(&ctx, Action::OpenPlotExport(name.clone()));
        assert_eq!(app.spec_dialog.dataset(), Some("(plot)"));
        app.spec_dialog.set_text("file", "out/p.svg");
        app.spec_dialog.set_text("format", "svg");
        let code = app
            .spec_dialog
            .code()
            .and_then(Result::ok)
            .unwrap_or_default();
        assert!(code.contains(&format!("plots[[\"{name}\"]]")), "{code}");
        assert!(code.contains("grDevices::svg(\"out/p.svg\""), "{code}");
        // the scripted route opens the same dialog, on the newest plot
        assert!(app.open_dialog(&ctx, "Export plot", &[]).is_ok());
        assert_eq!(app.spec_dialog.dataset(), Some("(plot)"));
    }

    #[test]
    fn the_objects_menu_renames_and_removes_any_object() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        app.perform(
            &ctx,
            Action::Submit(
                Source::Typed,
                "frame1 <- data.frame(a = 1:3); answer <- 42".into(),
            ),
        );
        let listed = |a: &App, name: &str| {
            a.engine()
                .is_some_and(|e| e.workspace().iter().any(|w| w.name == name))
        };
        assert!(frames_until(&mut app, &ctx, |a| listed(a, "answer")
            && a.dataset.as_deref() == Some("frame1")));

        // rename a plain object: the data frame shown stays shown
        let rename = &rcraft_catalog::data::RENAME_OBJECT;
        app.perform(&ctx, Action::OpenObjectSpec(rename, "answer".into()));
        assert_eq!(app.spec_dialog.dataset(), Some("answer"));
        app.spec_dialog.set_text("result", "reply");
        let code = app.spec_dialog.code().and_then(Result::ok);
        assert!(code
            .as_deref()
            .is_some_and(|c| c.contains("reply <- answer")));
        app.perform(
            &ctx,
            Action::Submit(Source::Menu(rename.menu.into()), code.unwrap_or_default()),
        );
        assert!(frames_until(&mut app, &ctx, |a| listed(a, "reply")
            && !listed(a, "answer")
            && a.is_settled()));
        assert_eq!(app.dataset.as_deref(), Some("frame1"));
        assert_eq!(app.show_when_listed, None);

        // remove the data frame that is shown: the window lets go of it
        let remove = &rcraft_catalog::data::REMOVE_OBJECT;
        app.perform(&ctx, Action::OpenObjectSpec(remove, "frame1".into()));
        let code = app.spec_dialog.code().and_then(Result::ok);
        assert_eq!(code.as_deref(), Some("rm(frame1)"));
        app.perform(
            &ctx,
            Action::Submit(Source::Menu(remove.menu.into()), code.unwrap_or_default()),
        );
        assert!(frames_until(&mut app, &ctx, |a| !listed(a, "frame1")
            && a.dataset.is_none()
            && a.is_settled()));
    }

    #[test]
    fn the_packages_window_opens_on_a_tab_and_rejects_an_unknown_one() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(app
            .show_packages("nonsense", "", None, false, None)
            .is_err());
        assert!(!app.packages_window.open);
        assert!(app
            .show_packages("find", "robust", Some("sandwich"), false, None)
            .is_ok());
        assert!(app.packages_window.open);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        // the window stays up through frames, over the made-up list, with no network
        assert!(app.packages_window.open);
        assert!(!app.package_index.is_busy());
    }

    #[test]
    fn the_installed_tab_lists_what_r_has() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        assert!(app
            .show_packages("installed", "", None, false, None)
            .is_ok());
        assert!(frames_until(&mut app, &ctx, |a| a
            .packages_window
            .installed_names()
            .iter()
            .any(|n| n == "stats")));
    }
}
