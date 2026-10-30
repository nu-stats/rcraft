//! The window: panes, menus and keys.

use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

use eframe::egui::{self, Align, Key, Layout, Modifiers, RichText, TextStyle};
use rcraft_engine::{Engine, History, Item, Outcome, PackageIndexService, QueryId, Recall, Source};
use rcraft_rcodegen::dialog::Spec;
use rcraft_rcodegen::dialog_help;
use rcraft_rcodegen::help::{self, HelpCommand};
use rcraft_rcodegen::rc::{self, Rc};
use rcraft_rcodegen::tab;
use rcraft_rdata::display::LabelView;
use rcraft_rhost::recent::Recent;
use rcraft_rhost::{find_installs, Env, InterruptOutcome, SessionConfig, Stream, MIN_VERSION};
use rcraft_rpkg::describe::{describe, summary_line};
use rcraft_rpkg::search::Filters;

use crate::data_view::{self, ObjectClick, TypeFilter, VariableClick};
use crate::graph_view::GraphView;
use crate::help_window::{HelpOut, HelpWindow};
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

/// One R session as the window keeps it: the engine and everything the window
/// shows about it (the Command line and its recall, the Script, which tab, the
/// data frame shown and the tabs open, the plots). The live session's are
/// fields of [`App`]; the others wait in `App::sessions` and are swapped in.
struct SessionUi {
    name: String,
    /// The folder this session's R was started in (or last moved to by the window).
    start_folder: Option<std::path::PathBuf>,
    engine: EngineState,
    command: String,
    recall: Recall,
    script: String,
    needs_restart: bool,
    message: Option<String>,
    view: View,
    graph: GraphView,
    dataset: Option<String>,
    open_frames: Vec<String>,
    dialog_watch: Vec<String>,
    grid_x: f32,
    label_view: LabelView,
    side_tab: SideTab,
    type_filter: TypeFilter,
    show_when_listed: Option<String>,
    completions: Vec<String>,
    tab_pending: Option<(QueryId, String)>,
}

impl SessionUi {
    /// A new session called `name`, starting R in `folder` (or where the
    /// program is, with none).
    fn fresh(name: String, folder: Option<std::path::PathBuf>) -> SessionUi {
        SessionUi {
            name,
            start_folder: folder.clone(),
            engine: EngineState::Starting(start_engine(folder)),
            ..SessionUi::hollow()
        }
    }

    /// What a slot holds while its session is the live one: nothing that is read.
    fn hollow() -> SessionUi {
        SessionUi {
            name: String::new(),
            start_folder: None,
            engine: EngineState::Failed(String::new()),
            command: String::new(),
            recall: Recall::new(),
            script: SCRIPT_START.to_string(),
            needs_restart: false,
            message: None,
            view: View::Results,
            graph: GraphView::default(),
            dataset: None,
            open_frames: Vec::new(),
            dialog_watch: Vec::new(),
            grid_x: 0.0,
            label_view: LabelView::default(),
            side_tab: SideTab::Variables,
            type_filter: TypeFilter::default(),
            show_when_listed: None,
            completions: Vec::new(),
            tab_pending: None,
        }
    }
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
    /// A typed `rc` command, with the line as typed.
    Rc(Rc, String),
    /// A typed `rc` line that is wrong, with what is wrong.
    RcError(String, String),
    /// A typed help request (`?mean`, `help mean`), with the line as typed.
    Help(HelpCommand, String),
    /// Open the Help window at its search box.
    OpenHelp,
    /// Open the Help window on a package, vignettes or NEWS request (from a menu).
    HelpRequest(HelpCommand),
    /// Look for a package for this topic, in the Packages window.
    FindPackagesFor(String),
    /// Open the Packages window on this tab.
    OpenPackages(PackagesTab),
    /// Open a catalog dialog on the data frame shown in Variables.
    OpenSpec(&'static Spec),
    /// Open one of the Objects pane's dialogs on this object.
    OpenObjectSpec(&'static Spec, String),
    /// Open Export plot on this plot (its file name; empty: the newest).
    OpenPlotExport(String),
    /// Close this data frame's tab (the data frame stays in R).
    CloseFrame(String),
    SetView(View),
    SetSideTab(SideTab),
    SetTypeFilter(TypeFilter),
    /// Start another R session and switch to it.
    NewSession,
    /// Make this session (its place in the list) the one shown.
    SwitchSession(usize),
    /// Ask whether to end this session.
    AskCloseSession(usize),
    /// Ask for a folder and make it the one R works in.
    ChooseFolder,
    /// Make this the folder R works in (and the one it starts in from now on).
    SetFolder(std::path::PathBuf),
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
    /// The name of the session shown (`R 1`); the others are in `sessions`.
    session_name: String,
    /// Where the session shown was started (or last moved to by the window).
    start_folder: Option<std::path::PathBuf>,
    /// Every session, in the order made. The live one's state is in this struct
    /// (its slot here is empty); the others' is in their slots.
    sessions: Vec<SessionUi>,
    /// Which slot of `sessions` is the live one.
    active: usize,
    /// Numbers the next session made.
    next_session: usize,
    /// A session asked to close, waiting for a yes.
    confirm_close: Option<usize>,
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
    /// The data frames open as tabs above the Data grid, in the order opened.
    open_frames: Vec<String>,
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
    help_window: HelpWindow,
    /// What Tab could complete the `rc` line to, when it was more than one word.
    completions: Vec<String>,
    /// A completion asked of R and not yet answered: the question, and the
    /// line it was asked for (an answer is used only if the line is unchanged).
    tab_pending: Option<(QueryId, String)>,
    /// Whether choices (the working folder) are kept between runs. Pictures and
    /// tests turn it off, so they never touch the user's own settings.
    keep_settings: bool,
}

impl App {
    pub fn new(ctx: &egui::Context) -> App {
        look::apply_static(ctx);
        let accent = Accent::default();
        look::apply_colors(ctx, accent);
        let theme = egui::ThemePreference::System;
        ctx.set_theme(theme);
        let recent = Recent::load(&Env::from_process());
        let start_folder = rcraft_rhost::folder::load(&Env::from_process());
        let mut import = ImportDialog::default();
        import.set_recent(recent.paths());
        App {
            session_name: "R 1".to_string(),
            start_folder: start_folder.clone(),
            sessions: vec![SessionUi::hollow()],
            active: 0,
            next_session: 2,
            confirm_close: None,
            engine: EngineState::Starting(start_engine(rcraft_rhost::folder::load(
                &Env::from_process(),
            ))),
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
            open_frames: Vec::new(),
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
            help_window: HelpWindow::default(),
            completions: Vec::new(),
            tab_pending: None,
            keep_settings: true,
        }
    }

    /// Turns off keeping choices between runs (for pictures and tests).
    pub fn keep_settings(&mut self, keep: bool) {
        self.keep_settings = keep;
    }

    /// Uses another list of recent files (pictures and tests keep their own,
    /// so they neither read nor change the user's).
    pub fn use_recent(&mut self, recent: Recent) {
        self.import.set_recent(recent.paths());
        self.recent = recent;
    }

    /// Swaps the live session with the one in slot `i`: the live one goes into
    /// the slot and the slot's comes alive (a slot then holds what was live).
    fn exchange(&mut self, i: usize) {
        use std::mem::swap;
        let Some(mut other) = self
            .sessions
            .get_mut(i)
            .map(|s| std::mem::replace(s, SessionUi::hollow()))
        else {
            return;
        };
        swap(&mut self.session_name, &mut other.name);
        swap(&mut self.start_folder, &mut other.start_folder);
        swap(&mut self.engine, &mut other.engine);
        swap(&mut self.command, &mut other.command);
        swap(&mut self.recall, &mut other.recall);
        swap(&mut self.script, &mut other.script);
        swap(&mut self.needs_restart, &mut other.needs_restart);
        swap(&mut self.message, &mut other.message);
        swap(&mut self.view, &mut other.view);
        swap(&mut self.graph, &mut other.graph);
        swap(&mut self.dataset, &mut other.dataset);
        swap(&mut self.open_frames, &mut other.open_frames);
        swap(&mut self.dialog_watch, &mut other.dialog_watch);
        swap(&mut self.grid_x, &mut other.grid_x);
        swap(&mut self.label_view, &mut other.label_view);
        swap(&mut self.side_tab, &mut other.side_tab);
        swap(&mut self.type_filter, &mut other.type_filter);
        swap(&mut self.show_when_listed, &mut other.show_when_listed);
        swap(&mut self.completions, &mut other.completions);
        swap(&mut self.tab_pending, &mut other.tab_pending);
        if let Some(slot) = self.sessions.get_mut(i) {
            *slot = other;
        }
    }

    /// Makes session `to` the live one. A dialog open on the old session is
    /// closed: it was working on that session's data frames.
    fn switch_session(&mut self, to: usize) {
        if to == self.active || to >= self.sessions.len() {
            return;
        }
        self.exchange(self.active);
        self.exchange(to);
        self.active = to;
        self.spec_dialog = SpecDialog::default();
        self.import.open = false;
        self.focus_command = true;
    }

    /// Starts another R session, in the folder the live one works in, and
    /// switches to it.
    fn new_session(&mut self) {
        let folder = self
            .engine()
            .and_then(|e| e.working_dir())
            .map(std::path::PathBuf::from)
            .filter(|d| d.is_dir())
            .or_else(|| self.start_folder.clone());
        let name = format!("R {}", self.next_session);
        self.next_session += 1;
        self.sessions.push(SessionUi::fresh(name, folder));
        let last = self.sessions.len() - 1;
        self.switch_session(last);
    }

    /// Ends session `i`: its R stops and its objects are gone. The last
    /// session cannot be closed. Closing the live one switches to its neighbour.
    fn close_session(&mut self, i: usize) {
        if self.sessions.len() < 2 || i >= self.sessions.len() {
            return;
        }
        if i == self.active {
            let neighbour = if i > 0 { i - 1 } else { 1 };
            self.switch_session(neighbour);
        }
        // the slot of a session that is not live holds its real state
        if i < self.sessions.len() {
            self.sessions.remove(i);
            if i < self.active {
                self.active -= 1;
            }
        }
    }

    /// The sessions' names for the menu: the name and the folder R is in.
    fn session_labels(&self) -> Vec<String> {
        (0..self.sessions.len())
            .map(|j| {
                let (name, engine) = if j == self.active {
                    (self.session_name.as_str(), &self.engine)
                } else {
                    match self.sessions.get(j) {
                        Some(s) => (s.name.as_str(), &s.engine),
                        None => ("", &self.engine),
                    }
                };
                let folder = match engine {
                    EngineState::Ready(e) => e
                        .working_dir()
                        .and_then(|w| std::path::Path::new(w).file_name())
                        .map(|f| f.to_string_lossy().into_owned()),
                    _ => None,
                };
                match folder {
                    Some(f) => format!("{name}  ({f})"),
                    None => name.to_string(),
                }
            })
            .collect()
    }

    /// Serves the sessions that are not live: they finish starting, and what
    /// R prints is taken in, so a long command in one goes on while another is
    /// shown.
    fn poll_parked(&mut self, ctx: &egui::Context) {
        for (j, session) in self.sessions.iter_mut().enumerate() {
            if j == self.active {
                continue;
            }
            if let EngineState::Starting(rx) = &session.engine {
                match rx.try_recv() {
                    Ok(Ok(engine)) => session.engine = EngineState::Ready(Box::new(engine)),
                    Ok(Err(why)) => session.engine = EngineState::Failed(why),
                    Err(TryRecvError::Empty) => {
                        ctx.request_repaint_after(Duration::from_millis(100));
                    }
                    Err(TryRecvError::Disconnected) => {
                        session.engine = EngineState::Failed("R did not start".into());
                    }
                }
            }
            if let EngineState::Ready(engine) = &mut session.engine {
                if engine.poll(Duration::ZERO).changed || engine.is_working() {
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
            }
        }
    }

    /// Keeps the tabs above the Data grid: the data frame shown has one, and a
    /// tab whose data frame has gone from R is dropped.
    fn track_open_frames(&mut self) {
        if let Some(name) = &self.dataset {
            if !self.open_frames.contains(name) {
                self.open_frames.push(name.clone());
            }
        }
        if let EngineState::Ready(engine) = &self.engine {
            let listed = engine.workspace();
            let shown = self.dataset.clone();
            self.open_frames.retain(|f| {
                shown.as_deref() == Some(f.as_str())
                    || listed.iter().any(|w| &w.name == f && w.is_data_frame())
            });
        }
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
        self.poll_parked(ctx);
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
            Action::CloseFrame(name) => {
                let at = self.open_frames.iter().position(|f| *f == name);
                self.open_frames.retain(|f| *f != name);
                if self.dataset.as_deref() == Some(name.as_str()) {
                    // the neighbour on the left, else the first one left, else none
                    let next = at
                        .and_then(|i| i.checked_sub(1))
                        .and_then(|i| self.open_frames.get(i))
                        .or_else(|| self.open_frames.first())
                        .cloned();
                    self.perform(ctx, Action::SelectDataset(next));
                }
            }
            Action::SetView(view) => self.view = view,
            Action::SetSideTab(tab) => self.side_tab = tab,
            Action::SetTypeFilter(filter) => self.type_filter = filter,
            Action::OpenPackages(tab) => self.packages_window.open_on(tab),
            Action::Rc(command, line) => self.run_rc(command, &line),
            Action::Help(command, line) => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    engine.echo(&line, "Opening the Help window.", Outcome::Ok);
                }
                self.help_window.request(command);
            }
            Action::OpenHelp => self.help_window.open_search(),
            Action::HelpRequest(command) => self.help_window.request(command),
            Action::FindPackagesFor(topic) => {
                self.packages_window.start_rc(Rc::Search { words: topic });
            }
            Action::RcError(why, line) => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    engine.echo(&line, &why, Outcome::Error);
                }
            }
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
            Action::NewSession => self.new_session(),
            Action::SwitchSession(i) => self.switch_session(i),
            Action::AskCloseSession(i) => self.confirm_close = Some(i),
            Action::ChooseFolder => {
                let here = self
                    .engine()
                    .and_then(|e| e.working_dir())
                    .map(std::path::PathBuf::from)
                    .filter(|d| d.is_dir());
                let mut chooser = rfd::FileDialog::new().set_title("Choose the working folder");
                if let Some(dir) = here {
                    chooser = chooser.set_directory(dir);
                }
                if let Some(dir) = chooser.pick_folder() {
                    self.perform(ctx, Action::SetFolder(dir));
                }
            }
            Action::SetFolder(dir) => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    // R moves with an ordinary command, so it is in Results and the log
                    let path = dir.to_string_lossy().replace('\\', "/");
                    let code = format!("setwd({})", rcraft_rcodegen::r_string(&path));
                    match engine.submit(Source::Menu("Session > Working folder".into()), &code) {
                        Ok(_) => {
                            self.message = None;
                            engine.set_start_folder(Some(dir.clone()));
                            self.start_folder = Some(dir.clone());
                            if self.keep_settings {
                                rcraft_rhost::folder::save(&Env::from_process(), &dir);
                            }
                        }
                        Err(e) => self.message = Some(e.to_string()),
                    }
                }
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
    /// What Tab does on an `rc` line: the line completed as far as it can be,
    /// and, when several words fit, the choices shown above the line.
    fn complete_rc(&mut self) -> Option<String> {
        let cran = |p: &str| -> Vec<String> {
            self.package_index
                .index()
                .map(|ix| {
                    ix.with_prefix(p)
                        .iter()
                        .take(60)
                        .map(|q| q.name.clone())
                        .collect()
                })
                .unwrap_or_default()
        };
        let mine = |p: &str| -> Vec<String> {
            self.packages_window
                .installed_names()
                .into_iter()
                .filter(|n| n.starts_with(p))
                .collect()
        };
        let c = rc::complete(&self.command, &cran, &mine)?;
        if c.options.is_empty() {
            self.completions.clear();
            return None;
        }
        let typed = self.command.get(c.from..).unwrap_or("").to_string();
        let common = rc::common_prefix(&c.options);
        let mut line = self.command.get(..c.from).unwrap_or("").to_string();
        line.push_str(if common.len() > typed.len() {
            &common
        } else {
            &typed
        });
        if let [only] = c.options.as_slice() {
            if !only.ends_with('(') {
                line.push(' ');
            }
            self.completions.clear();
        } else {
            self.completions = c.options;
        }
        Some(line)
    }

    /// Carries out a typed `rc` command. The ones that change packages open the
    /// Packages window with the list to say yes to; the others answer here, in
    /// Results, and are recorded in the log and the history like any command.
    fn run_rc(&mut self, command: Rc, line: &str) {
        let EngineState::Ready(engine) = &mut self.engine else {
            self.message = Some("Wait until R has started, then type the command again.".into());
            return;
        };
        self.package_index.ensure_loaded();
        let loading = matches!(
            self.package_index.status(),
            rcraft_engine::IndexStatus::Loading
        );
        let no_list = "The CRAN package list is not on this computer yet. Open Packages > Find \
                       and install and press \"Download the package list\" (about 15 MB); \
                       searching then works offline.";
        let reading = "The saved package list is being read. Type the command again in a second.";
        match command {
            Rc::Help => {
                engine.echo(line, rcraft_rcodegen::rc::HELP, Outcome::Ok);
            }
            Rc::Search { words } => {
                let text = match self.package_index.index() {
                    None => (if loading { reading } else { no_list }).to_string(),
                    Some(_) => {
                        let hits = self.package_index.search(&words, &Filters::default(), 15);
                        if hits.is_empty() {
                            format!("No package on CRAN matches \"{words}\".")
                        } else {
                            let mut text = format!("Best matches for \"{words}\":\n");
                            for (p, _) in &hits {
                                text.push_str(&format!("  {}\n", summary_line(p)));
                            }
                            text.push_str("rc describe NAME tells more; rc install NAME installs.");
                            text
                        }
                    }
                };
                let ok = self.package_index.index().is_some();
                engine.echo(line, &text, if ok { Outcome::Ok } else { Outcome::Error });
            }
            Rc::Describe { package } => {
                let (text, ok) = match self.package_index.index() {
                    None => ((if loading { reading } else { no_list }).to_string(), false),
                    Some(ix) => match describe(ix, &package) {
                        Some(d) => (d.to_text(), true),
                        None => match ix.get_ignoring_case(&package) {
                            Some(p) => (
                                format!(
                                    "No package \"{package}\" on CRAN. Did you mean \"{}\"?",
                                    p.name
                                ),
                                false,
                            ),
                            None => (format!("No package \"{package}\" on CRAN."), false),
                        },
                    },
                };
                engine.echo(line, &text, if ok { Outcome::Ok } else { Outcome::Error });
            }
            Rc::Installed { filter } => {
                let code = rc::installed_code(line, &filter);
                if let Err(e) = engine.submit(Source::Typed, &code) {
                    self.message = Some(e.to_string());
                }
            }
            Rc::InstallFrom { source, what } => {
                engine.echo(
                    line,
                    "Opening the Packages window: it looks the source up, then shows the list to say yes to. Nothing is installed before that.",
                    Outcome::Ok,
                );
                self.packages_window.start_source(source, what);
            }
            Rc::Install { .. } | Rc::Update { .. } | Rc::Remove { .. } => {
                engine.echo(
                    line,
                    "Opening the Packages window with the list. Nothing happens until you say yes there.",
                    Outcome::Ok,
                );
                self.packages_window.start_rc(command);
            }
        }
    }
}

impl App {
    /// Draws one frame and carries out what was asked. The window and the
    /// offscreen picture (`headless.rs`) both call this.
    pub fn ui(&mut self, ctx: &egui::Context) {
        self.poll(ctx);
        // the dialogs' file choosers open in the folder R is working in
        let folder = self
            .engine()
            .and_then(|e| e.working_dir())
            .map(std::path::PathBuf::from);
        self.import.set_folder(folder.clone());
        self.spec_dialog.set_folder(folder);
        let dark = ctx.style().visuals.dark_mode;
        self.track_open_frames();
        let pal = Palette::new(self.accent, dark);
        let mut actions: Vec<Action> = Vec::new();
        let busy = self.engine().is_some_and(Engine::is_busy);

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            self.menu_bar(ui, &pal, &mut actions);
        });
        egui::TopBottomPanel::bottom("status")
            .show(ctx, |ui| self.status_bar(ui, &pal, &mut actions));
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
            Some(DialogAction::Help) => self.help_window.show_dialog(dialog_help::use_my_file()),
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

        if std::mem::take(&mut self.packages_window.wants_help) {
            self.help_window.show_dialog(dialog_help::packages_window());
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

        let engine = match &mut self.engine {
            EngineState::Ready(e) => Some(&mut **e),
            _ => None,
        };
        let shown = self.help_window.shown_package().map(str::to_string);
        let on_cran = shown.and_then(|n| self.package_index.index().map(|ix| ix.get(&n).is_some()));
        let mirror = self.package_index.mirror().to_string();
        self.help_window.set_cran(&mirror, on_cran);
        match self.help_window.show(ctx, &pal, engine) {
            Some(HelpOut::Run(code)) => actions.push(Action::Submit(Source::Typed, code)),
            Some(HelpOut::FindPackages(topic)) => actions.push(Action::FindPackagesFor(topic)),
            None => {}
        }

        if let Some(i) = self.confirm_close {
            let name = self.session_labels().get(i).cloned().unwrap_or_default();
            let mut open = true;
            let mut decision: Option<bool> = None;
            egui::Window::new("Close session")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("End {name}?"));
                    ui.label(
                        RichText::new("Its R stops and every object in it is lost.").color(pal.dim),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Close session").clicked() {
                            decision = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            decision = Some(false);
                        }
                    });
                });
            if decision == Some(true) {
                self.close_session(i);
            }
            if !open || decision.is_some() {
                self.confirm_close = None;
            }
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
            Some(SpecAction::Help(spec)) => {
                self.help_window.show_dialog(dialog_help::for_spec(spec))
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
                if ui.button("Project package list…").clicked() {
                    out.push(Action::OpenPackages(PackagesTab::List));
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
                ui.separator();
                if ui.button("Working folder…").clicked() {
                    out.push(Action::ChooseFolder);
                    ui.close_menu();
                }
                ui.separator();
                let labels = self.session_labels();
                for (j, label) in labels.iter().enumerate() {
                    if ui.selectable_label(j == self.active, label).clicked() {
                        out.push(Action::SwitchSession(j));
                        ui.close_menu();
                    }
                }
                if ui.button("New R session").clicked() {
                    out.push(Action::NewSession);
                    ui.close_menu();
                }
                if ui
                    .add_enabled(labels.len() > 1, egui::Button::new("Close this session…"))
                    .on_disabled_hover_text("The only session cannot be closed")
                    .clicked()
                {
                    out.push(Action::AskCloseSession(self.active));
                    ui.close_menu();
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Search R help…").clicked() {
                    out.push(Action::OpenHelp);
                    ui.close_menu();
                }
                if ui.button("Vignettes and package manuals…").clicked() {
                    out.push(Action::HelpRequest(HelpCommand::Vignettes {
                        package: None,
                    }));
                    ui.close_menu();
                }
                ui.separator();
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

    fn status_bar(&self, ui: &mut egui::Ui, pal: &Palette, out: &mut Vec<Action>) {
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
            if self.sessions.len() > 1 {
                ui.label(RichText::new(format!("Session {}", self.session_name)).color(pal.dim));
                ui.separator();
            }
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
                // the folder R is in (what getwd() says), not the program's own
                let folder = self
                    .engine()
                    .and_then(|e| e.working_dir())
                    .unwrap_or("(R is starting)")
                    .to_string();
                let label = egui::Label::new(
                    RichText::new(format!("Working folder: {folder}")).color(pal.dim),
                )
                .truncate()
                .sense(egui::Sense::click());
                if ui
                    .add(label)
                    .on_hover_text(format!(
                        "{folder}
Click to choose another folder"
                    ))
                    .clicked()
                {
                    out.push(Action::ChooseFolder);
                }
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
            self.data_pane(ui, pal, out);
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

    fn data_pane(&mut self, ui: &mut egui::Ui, pal: &Palette, out: &mut Vec<Action>) {
        if !self.open_frames.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for name in &self.open_frames {
                    let chosen = self.dataset.as_deref() == Some(name.as_str());
                    let text = RichText::new(name).monospace();
                    if ui.selectable_label(chosen, text).clicked() && !chosen {
                        out.push(Action::SelectDataset(Some(name.clone())));
                    }
                    if ui
                        .small_button("×")
                        .on_hover_text("Close this tab. The data frame stays in R.")
                        .clicked()
                    {
                        out.push(Action::CloseFrame(name.clone()));
                    }
                    ui.add_space(6.0);
                }
            });
            ui.separator();
        }
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
        if !self.completions.is_empty() {
            ui.label(
                RichText::new(self.completions.join("   "))
                    .monospace()
                    .small()
                    .color(pal.accent),
            );
        }
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
        // F1 asks for help on the word at the end of the line, or opens the search
        if has_focus && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F1)) {
            out.push(match help::word_at_end(&self.command) {
                Some(word) => {
                    let line = format!("?{word}");
                    match help::parse_command(&line) {
                        Some(request) => Action::Help(request, line),
                        None => Action::OpenHelp,
                    }
                }
                None => Action::OpenHelp,
            });
        }
        // R's answer to an earlier Tab, if it has come and the line is the same
        if let Some((id, line)) = self.tab_pending.clone() {
            let answer = self
                .engine()
                .and_then(|e| e.query_result(id))
                .map(|r| (r.ok, r.text.clone()));
            match answer {
                Some((ok, text)) => {
                    self.tab_pending = None;
                    if ok && self.command == line {
                        if let Some(c) = tab::parse_answer(&line, &text) {
                            let (done, shown) = tab::apply(&line, &c);
                            self.completions = shown;
                            recalled = Some(done);
                        } else {
                            self.completions.clear();
                        }
                    }
                }
                None => ui.ctx().request_repaint_after(Duration::from_millis(30)),
            }
        }
        // Tab completes an `rc` line (commands, CRAN package names, options);
        // any other line is completed by R itself (names, columns, files)
        if has_focus && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) {
            let t = self.command.trim_start();
            if t == "rc" || t.starts_with("rc ") || t.starts_with("rc	") {
                if let Some(done) = self.complete_rc() {
                    recalled = Some(done);
                }
            } else if !busy && !self.command.trim().is_empty() && self.tab_pending.is_none() {
                let line = self.command.clone();
                if let EngineState::Ready(engine) = &mut self.engine {
                    let id = engine.query(&tab::completion_code(&line));
                    self.tab_pending = Some((id, line));
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
            self.completions.clear();
            out.push(match rc::parse(&code) {
                Some(Ok(command)) => Action::Rc(command, code),
                Some(Err(why)) => Action::RcError(why, code),
                None => match help::parse_command(&code) {
                    Some(request) => Action::Help(request, code),
                    None => Action::Submit(Source::Typed, code),
                },
            });
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
fn start_engine(folder: Option<std::path::PathBuf>) -> Receiver<Result<Engine, String>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let result = (|| {
            let install = find_installs(&Env::from_process())
                .into_iter()
                .find(|i| i.is_supported())
                .ok_or_else(|| format!("No R found. RCraft needs R {MIN_VERSION} or later."))?;
            let mut config = SessionConfig::new(install);
            config.working_dir = folder;
            Engine::start(config).map_err(|e| e.to_string())
        })();
        let _ = tx.send(result);
    });
    rx
}

impl App {
    /// R is ready and nothing is running (or R could not start).
    pub fn is_settled(&self) -> bool {
        match &self.engine {
            EngineState::Ready(e) => !e.is_working() && !self.help_window.is_busy(),
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

    /// For pictures: gives the Packages window the task views of a downloaded
    /// `Views.rds`, and opens (and filters by) one of them when named.
    pub fn show_packages_views(
        &mut self,
        path: &std::path::Path,
        view: Option<&str>,
    ) -> Result<(), String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let node = rcraft_rpkg::rds::read(&bytes).map_err(|e| e.to_string())?;
        let views = rcraft_rpkg::views::Views::from_node(&node).map_err(|e| e.to_string())?;
        self.package_index.set_views(views);
        self.packages_window.preset_view(view);
        Ok(())
    }

    /// For pictures: a GitHub install waiting for its yes.
    pub fn show_packages_source_demo(&mut self) {
        self.packages_window.preset_source_demo(&self.package_index);
    }

    /// For pictures: opens a package list file in the Packages window.
    pub fn show_packages_lock(&mut self, path: &std::path::Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        self.packages_window.load_lock_text(&text);
        Ok(())
    }

    /// For pictures: the Packages window with a made-up install failure showing.
    pub fn show_packages_failure(&mut self, kind: &str) -> Result<(), String> {
        self.packages_window.preset_failure(kind)
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
            "list" => PackagesTab::List,
            "browse" => PackagesTab::Browse,
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
    /// Shows the Help window on the page of a dialog, by menu entry name (or
    /// `Use my file` or `Packages`).
    pub fn show_dialog_help(&mut self, entry: &str) -> Result<(), String> {
        let page = if entry.eq_ignore_ascii_case("Use my file") {
            dialog_help::use_my_file()
        } else if entry.eq_ignore_ascii_case("Packages") {
            dialog_help::packages_window()
        } else {
            let spec = rcraft_catalog::all_specs()
                .into_iter()
                .find(|s| s.entry.eq_ignore_ascii_case(entry))
                .ok_or_else(|| format!("no dialog called {entry}"))?;
            dialog_help::for_spec(spec)
        };
        self.help_window.show_dialog(page);
        Ok(())
    }

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
        match rc::parse(code) {
            Some(Ok(command)) => self.run_rc(command, code),
            Some(Err(why)) => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    engine.echo(code, &why, Outcome::Error);
                }
            }
            None => match help::parse_command(code) {
                Some(request) => {
                    if let EngineState::Ready(engine) = &mut self.engine {
                        engine.echo(code, "Opening the Help window.", Outcome::Ok);
                    }
                    self.help_window.request(request);
                }
                None => {
                    if let EngineState::Ready(engine) = &mut self.engine {
                        if let Err(e) = engine.submit(Source::Typed, code) {
                            self.message = Some(e.to_string());
                        }
                    }
                }
            },
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
    fn data_frames_open_as_tabs_and_closing_one_picks_a_neighbour() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        for name in ["a", "b", "c"] {
            app.perform(&ctx, Action::SelectDataset(Some(name.into())));
            app.track_open_frames();
        }
        assert_eq!(app.open_frames, ["a", "b", "c"]);
        assert_eq!(app.dataset.as_deref(), Some("c"));
        // going back to one already open adds nothing
        app.perform(&ctx, Action::SelectDataset(Some("a".into())));
        app.track_open_frames();
        assert_eq!(app.open_frames.len(), 3);
        // closing a tab that is not shown leaves the one shown
        app.perform(&ctx, Action::CloseFrame("c".into()));
        assert_eq!(app.open_frames, ["a", "b"]);
        assert_eq!(app.dataset.as_deref(), Some("a"));
        // closing the one shown goes to its neighbour on the left, else the first
        app.perform(&ctx, Action::SelectDataset(Some("b".into())));
        app.perform(&ctx, Action::CloseFrame("b".into()));
        assert_eq!(app.dataset.as_deref(), Some("a"));
        app.perform(&ctx, Action::CloseFrame("a".into()));
        assert_eq!(app.dataset, None);
        assert!(app.open_frames.is_empty());
        // closing one that is not there is harmless
        app.perform(&ctx, Action::CloseFrame("zzz".into()));
    }

    #[test]
    fn a_tab_whose_data_frame_is_gone_from_r_is_dropped() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start here");
            return;
        }
        app.submit_typed("one <- data.frame(x = 1); two <- data.frame(y = 2)");
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()
            && a.engine().is_some_and(|e| e.workspace().len() == 2)));
        for name in ["one", "two"] {
            app.perform(&ctx, Action::SelectDataset(Some(name.into())));
            app.track_open_frames();
        }
        assert_eq!(app.open_frames, ["one", "two"]);
        app.submit_typed("rm(one)");
        assert!(frames_until(&mut app, &ctx, |a| a.open_frames == ["two"]));
    }

    /// The names of the objects in the live session's workspace.
    fn object_names(app: &App) -> Vec<String> {
        app.engine()
            .map(|e| e.workspace().iter().map(|w| w.name.clone()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn sessions_are_separate_keep_their_own_state_and_a_parked_one_goes_on_working() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start here");
            return;
        }
        app.submit_typed("first <- data.frame(a = 1)");
        assert!(frames_until(&mut app, &ctx, |a| {
            a.is_settled() && object_names(a) == ["first"]
        }));
        app.command = "typed in the first".into();
        app.script = "# the first script".into();

        // a second session starts empty, with nothing of the first one's
        app.perform(&ctx, Action::NewSession);
        assert_eq!(app.sessions.len(), 2);
        assert_eq!(app.active, 1);
        assert_eq!(app.session_name, "R 2");
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        assert!(app.is_ready(), "the second R did not start");
        assert!(object_names(&app).is_empty(), "{:?}", object_names(&app));
        assert_eq!(app.command, "");
        assert_ne!(app.script, "# the first script");

        // a slow command in the second session, then back to the first
        app.submit_typed("second <- data.frame(b = 2); Sys.sleep(1.5); done <- TRUE");
        app.perform(&ctx, Action::SwitchSession(0));
        assert_eq!(app.session_name, "R 1");
        assert_eq!(app.command, "typed in the first");
        assert_eq!(app.script, "# the first script");
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        assert_eq!(object_names(&app), ["first"]);

        // the second session carried on while it was parked
        app.perform(&ctx, Action::SwitchSession(1));
        assert!(frames_until(&mut app, &ctx, |a| {
            a.is_settled() && object_names(a) == ["done", "second"]
        }));

        // ending the shown session goes back to the other one
        app.close_session(1);
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.session_name, "R 1");
        assert_eq!(app.active, 0);
        assert!(frames_until(&mut app, &ctx, |a| {
            a.is_settled() && object_names(a) == ["first"]
        }));

        // the only session cannot be closed
        app.close_session(0);
        assert_eq!(app.sessions.len(), 1);
        assert!(app.is_ready());
    }

    #[test]
    fn a_new_session_starts_where_the_shown_one_is_and_closing_an_earlier_one_keeps_the_shown() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start here");
            return;
        }
        let dir = std::env::temp_dir().join(format!("rcraft-sessions-{}", std::process::id()));
        assert!(std::fs::create_dir_all(&dir).is_ok());
        assert!(std::fs::write(dir.join("marker.txt"), "x").is_ok());
        let in_dir = |a: &App| {
            a.engine()
                .and_then(|e| e.working_dir())
                .is_some_and(|w| std::path::Path::new(w).join("marker.txt").exists())
        };
        app.perform(&ctx, Action::SetFolder(dir.clone()));
        assert!(frames_until(&mut app, &ctx, in_dir));
        app.perform(&ctx, Action::NewSession);
        app.perform(&ctx, Action::NewSession);
        assert_eq!(app.sessions.len(), 3);
        assert_eq!(app.session_name, "R 3");
        // the new sessions started in the folder the one they were made from is in
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled() && in_dir(a)));
        let labels = app.session_labels();
        assert_eq!(labels.len(), 3);
        assert!(
            labels.get(2).is_some_and(|l| l.starts_with("R 3")),
            "{labels:?}"
        );
        assert!(
            labels.get(2).is_some_and(|l| l.contains("rcraft-sessions")),
            "{labels:?}"
        );
        // close the first while the third is shown: the third stays shown
        app.close_session(0);
        assert_eq!(app.sessions.len(), 2);
        assert_eq!(app.active, 1);
        assert_eq!(app.session_name, "R 3");
        app.perform(&ctx, Action::SwitchSession(0));
        assert_eq!(app.session_name, "R 2");
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled() && in_dir(a)));
        // requests out of range do nothing
        app.perform(&ctx, Action::SwitchSession(9));
        app.close_session(9);
        assert_eq!(app.sessions.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn choosing_a_working_folder_moves_r_and_a_restart_keeps_it() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start here");
            return;
        }
        assert!(frames_until(&mut app, &ctx, |a| a
            .engine()
            .is_some_and(|e| e.working_dir().is_some())));
        let dir = std::env::temp_dir().join(format!("rcraft-app-folder-{}", std::process::id()));
        assert!(std::fs::create_dir_all(&dir).is_ok());
        assert!(std::fs::write(dir.join("marker.txt"), "x").is_ok());
        let in_dir = |a: &App| {
            a.engine()
                .and_then(|e| e.working_dir())
                .is_some_and(|w| std::path::Path::new(w).join("marker.txt").exists())
        };
        app.perform(&ctx, Action::SetFolder(dir.clone()));
        assert!(frames_until(&mut app, &ctx, in_dir), "R did not move");
        // a restarted R begins in the folder chosen, not where the program started
        app.perform(&ctx, Action::Restart);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        assert!(
            frames_until(&mut app, &ctx, in_dir),
            "the restart went elsewhere"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_dialogs_help_page_opens_in_the_help_window() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(app.show_dialog_help("no such dialog").is_err());
        assert!(!app.help_window.open);
        for name in ["Join", "Linear model", "Use my file", "Packages"] {
            assert!(app.show_dialog_help(name).is_ok(), "{name}");
            assert!(app.help_window.open);
        }
        // it stays up through frames, with no R needed for these pages
        for _ in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
        }
        assert!(app.help_window.open);
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

    /// The text of the newest Results block.
    fn newest_text(app: &App) -> String {
        let Some(engine) = app.engine() else {
            return String::new();
        };
        let mut out = String::new();
        if let Some(block) = engine.results().blocks().last() {
            for item in &block.items {
                if let Item::Text { text, .. } = item {
                    out.push_str(text);
                }
            }
        }
        out
    }

    fn app_with_sample_index(ctx: &egui::Context) -> App {
        let mut app = App::new(ctx);
        app.package_index = PackageIndexService::in_memory(rcraft_rpkg::index::sample());
        app
    }

    #[test]
    fn rc_search_and_describe_answer_in_results_without_r_running_them() {
        let ctx = egui::Context::default();
        let mut app = app_with_sample_index(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        app.submit_typed("rc search robust");
        let text = newest_text(&app);
        assert!(text.contains("sandwich"), "{text}");
        assert!(text.contains("rc describe NAME"), "{text}");
        app.submit_typed("rc describe fixest");
        let text = newest_text(&app);
        assert!(text.starts_with("fixest 0.12.1"), "{text}");
        assert!(text.contains("Imports: Rcpp (>= 1.0)"), "{text}");
        // a name that differs only in case is suggested
        app.submit_typed("rc describe FIXEST");
        assert!(newest_text(&app).contains("Did you mean \"fixest\""));
        // they are in the history and the log, and a comment in the script
        let e = app.engine();
        let history: Vec<&str> = e
            .map(|e| {
                e.history()
                    .entries()
                    .iter()
                    .map(|h| h.code.as_str())
                    .collect()
            })
            .unwrap_or_default();
        assert!(history.contains(&"rc search robust"), "{history:?}");
        let script = e.map(|e| e.script_text()).unwrap_or_default();
        assert!(script.contains("# rc search robust"), "{script}");
        assert!(!script.contains("\nrc search"), "{script}");
    }

    #[test]
    fn a_wrong_rc_line_says_what_is_wrong_and_other_lines_go_to_r() {
        let ctx = egui::Context::default();
        let mut app = app_with_sample_index(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        app.submit_typed("rc install");
        assert!(newest_text(&app).contains("needs a package name"));
        app.submit_typed("rc help");
        assert!(newest_text(&app).contains("rc install PACKAGE"));
        // R code that begins with rc is R
        app.submit_typed("rc <- 41 + 1");
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        app.submit_typed("cat(rc)");
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        assert_eq!(newest_text(&app).trim(), "42");
    }

    #[test]
    fn rc_install_opens_the_list_to_say_yes_to_and_runs_nothing() {
        let ctx = egui::Context::default();
        let mut app = app_with_sample_index(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        let before = app.engine().map(|e| e.history().len()).unwrap_or(0);
        app.submit_typed("rc install fixest");
        assert!(app.packages_window.open);
        // once R has listed its packages the list is there, waiting
        assert!(frames_until(&mut app, &ctx, |a| a
            .packages_window
            .has_confirmation()));
        let after = app.engine().map(|e| e.history().len()).unwrap_or(0);
        assert_eq!(after, before + 1, "only the typed line was recorded");
        let code = app.packages_window.confirmation_code().unwrap_or_default();
        assert!(code.contains("install.packages(\"fixest\""), "{code}");
    }

    #[test]
    fn tab_completes_rc_lines() {
        let ctx = egui::Context::default();
        let mut app = app_with_sample_index(&ctx);
        app.command = "rc ins".into();
        let done = app.complete_rc();
        assert_eq!(done.as_deref(), Some("rc install"));
        assert_eq!(app.completions, vec!["install", "installed"]);
        app.command = "rc install fixe".into();
        assert_eq!(app.complete_rc().as_deref(), Some("rc install fixest "));
        assert!(app.completions.is_empty());
        app.command = "plot(".into();
        assert_eq!(app.complete_rc(), None);
    }
}
