//! The window: panes, menus and keys.

use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

use eframe::egui::{self, Align, Key, Layout, Modifiers, RichText, TextStyle};
use rcraft_engine::{
    CommandId, DocKind, Engine, History, Item, Outcome, PackageIndexService, QueryId, Recall,
    Source,
};
use rcraft_rcodegen::dialog::Spec;
use rcraft_rcodegen::dialog_help;
use rcraft_rcodegen::help::{self, HelpCommand};
use rcraft_rcodegen::packages::{package_to_install, Need};
use rcraft_rcodegen::rc::{self, Rc};
use rcraft_rcodegen::tab;
use rcraft_rdata::display::LabelView;
use rcraft_rhost::recent::Recent;
use rcraft_rhost::{find_installs, Env, InterruptOutcome, SessionConfig, Stream, MIN_VERSION};
use rcraft_rpkg::describe::{describe, summary_line};
use rcraft_rpkg::search::Filters;

use crate::data_view::{self, ObjectClick, TypeFilter, VariableClick};
use crate::dofile_window::{DoFileOut, DoFileWindow};
use crate::graph_view::GraphView;
use crate::help_window::{HelpOut, HelpWindow};
use crate::import_dialog::{DialogAction, ImportDialog};
use crate::look::{self, Accent, Palette};
use crate::package_window::{PackagesWindow, Tab as PackagesTab};
use crate::project;
use crate::script_file::ScriptFile;
use crate::script_tools;
use crate::spec_dialog::{self, SpecAction, SpecDialog};
use crate::stata_mode::{self, After, StataMode};

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
const SCRIPT_ID: &str = "script-editor";
/// The find bar's text box.
const FIND_ID: &str = "script-find";

/// A file name made from a session's name: letters, digits, `-` and `_`
/// kept, the rest made `_`; `session` if nothing is left.
fn file_stem_of(name: &str) -> String {
    let stem: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = stem.trim_matches('_').to_string();
    if stem.is_empty() {
        "session".to_string()
    } else {
        stem
    }
}

/// What is selected in the Script Editor, as character positions, in order.
fn editor_selection(ctx: &egui::Context) -> Option<(usize, usize)> {
    egui::TextEdit::load_state(ctx, egui::Id::new(SCRIPT_ID))
        .and_then(|s| s.cursor.char_range())
        .map(|r| {
            let (a, b) = (r.primary.index, r.secondary.index);
            (a.min(b), a.max(b))
        })
}

/// Selects characters `a` to `b` in the Script Editor and gives it the focus.
fn select_in_editor(ctx: &egui::Context, a: usize, b: usize) {
    let id = egui::Id::new(SCRIPT_ID);
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(a),
            egui::text::CCursor::new(b),
        )));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}
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
    tab_pending: Option<TabAsk>,
    stata: StataMode,
    script_file: ScriptFile,
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
            stata: StataMode::default(),
            script_file: ScriptFile::unnamed(SCRIPT_START),
        }
    }
}

/// What a click or key asks for; collected while drawing, done afterwards,
/// so drawing never needs the engine mutably.
enum Action {
    Submit(Source, String),
    /// P1A.10: install a package a command said it needs, from an offer in
    /// Results; then, if given, run that command again.
    InstallNeeded {
        package: String,
        rerun: Option<(Source, String)>,
    },
    /// Stata, typed or from the Script, translated after the lines before it
    /// and run (P2B.7).
    SubmitStata(Source, String),
    /// Turns Stata mode on or off for the session shown.
    SetStataMode(bool),
    /// Open the differences guide on the command this Stata line runs (P2B.8).
    OpenStataGuide(String),
    SetCommand(String),
    /// Add text (a variable name) to the Command line.
    Insert(String),
    SelectDataset(Option<String>),
    /// Show this data frame as soon as R lists it (after an import).
    ShowWhenListed(String),
    OpenImport,
    /// Ask for a do-file and open the do-file window on it (P2C).
    OpenDoFile,
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
    /// A Reports menu dialog, which needs no data frame (P7.2).
    OpenFreeSpec(&'static Spec),
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
    /// Ask for a new name for the session shown.
    AskRenameSession,
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
    /// File > Open script... (asks first when the script has changes not saved).
    OpenScript,
    /// Open this script file (after any question).
    OpenScriptPath(std::path::PathBuf),
    /// File > Save script: to its file, or as a new one.
    SaveScript,
    /// File > Save script as...
    SaveScriptAs,
    /// Write the script to this file.
    SaveScriptTo(std::path::PathBuf),
    /// The answer to "save the changes first?": save, don't, or cancel.
    ScriptAnswer(Option<bool>),
    /// Show the find bar, with the Replace row or not.
    OpenFind(bool),
    CloseFind,
    /// Select the next (or previous) match after the cursor.
    FindStep(bool),
    /// Replace the selected match and select the next one.
    ReplaceOne,
    ReplaceAll,
    /// Put the cursor at the start of this line of the script (the outline).
    GoToLine(usize),
    /// Show this variable in Properties, or the data frame with none (P8.1).
    PickVariable(Option<String>),
    /// File > Save session as... (P7.4): an R script, Quarto or R Markdown.
    SaveSessionAs,
    /// Write the session to this file, the kind by its ending.
    SaveSessionTo(std::path::PathBuf),
    /// File > Save session as Quarto project...
    SaveSessionProjectAs,
    /// Write the session as a Quarto project in this folder.
    SaveSessionProject(std::path::PathBuf),
    /// File > Save project... (P7.5): choose its folder.
    SaveProjectAs,
    /// Save the script, R's objects, the log and the package list in this folder.
    SaveProjectTo(std::path::PathBuf),
    /// File > Open project...
    OpenProject,
    /// Open the project whose file this is.
    OpenProjectPath(std::path::PathBuf),
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
    /// The name being typed in the Rename session window (open when set).
    renaming: Option<String>,
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
    /// A completion asked of R and not yet answered (an answer is used only
    /// if the line is unchanged).
    tab_pending: Option<TabAsk>,
    /// Installs offered in Results for commands that needed a package.
    offers: InstallOffers,
    /// Whether choices (the working folder) are kept between runs. Pictures and
    /// tests turn it off, so they never touch the user's own settings.
    keep_settings: bool,
    /// Stata mode for the session shown (P2B.7).
    stata: StataMode,
    /// Run my do-file (P2C).
    dofile: DoFileWindow,
    /// The Script Editor's file (P7.3), for the session shown.
    script_file: ScriptFile,
    /// The script has changes not saved and another is about to replace it.
    script_ask: Option<ScriptAsk>,
    /// The Script Editor's find and replace bar (P7.3).
    find: FindBar,
    /// The variable Properties shows (P8.1); the data frame with none.
    picked_variable: Option<String>,
}

/// The Script Editor's find and replace bar.
#[derive(Debug, Clone, Default)]
struct FindBar {
    open: bool,
    /// The Replace row is shown too.
    replace: bool,
    search: script_tools::Search,
    with: String,
    /// What the last step found or did, in words.
    note: Option<String>,
}

/// What waits for the answer to "save the script's changes first?".
#[derive(Debug, Clone, PartialEq, Eq)]
enum ScriptAsk {
    /// Choose a script to open.
    Choose,
    /// Open this file.
    Open(std::path::PathBuf),
    /// Open this project (its script replaces this one).
    Project(std::path::PathBuf),
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
            renaming: None,
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
            offers: InstallOffers::default(),
            keep_settings: true,
            stata: StataMode::default(),
            dofile: DoFileWindow::default(),
            script_file: ScriptFile::unnamed(SCRIPT_START),
            script_ask: None,
            find: FindBar::default(),
            picked_variable: None,
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
        swap(&mut self.stata, &mut other.stata);
        swap(&mut self.script_file, &mut other.script_file);
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

    /// Renames the session shown. A blank name keeps the old one; a name is
    /// cut to 30 characters and line breaks become spaces.
    fn rename_session(&mut self, name: &str) {
        let clean: String = name
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .take(30)
            .collect();
        let clean = clean.trim();
        if !clean.is_empty() {
            self.session_name = clean.to_string();
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
                if engine.poll(Duration::ZERO).changed || !engine.is_quiet() {
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
                Ok(Ok(engine)) => {
                    self.packages_window.offer_other_r(&engine);
                    self.engine = EngineState::Ready(Box::new(engine));
                }
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
            if engine.is_working() || engine.refresh_waiting() {
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
            let gone = engine.is_quiet()
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
            } else if engine.is_quiet() {
                // the import failed: the error is in Results
                self.show_when_listed = None;
                self.view = View::Results;
            }
        }
        // the data Stata mode works on, once R lists it, without leaving Results
        if let Some(wanted) = self.stata.follow.clone() {
            let listed = engine
                .workspace()
                .iter()
                .any(|w| w.name == wanted && w.is_data_frame());
            if listed {
                if self.dataset.as_deref() != Some(wanted.as_str()) {
                    if let Some(old) = self.dataset.take() {
                        engine.unwatch(&old);
                    }
                    engine.watch(&wanted);
                    self.dataset = Some(wanted);
                }
                self.stata.follow = None;
            } else if engine.is_quiet() {
                self.stata.follow = None;
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
            Action::InstallNeeded { package, rerun } => {
                if let EngineState::Ready(engine) = &mut self.engine {
                    let code = offer_install_code(&package, engine.library().as_deref());
                    match engine.submit(Source::Menu("Packages > Install".into()), &code) {
                        Ok(id) => {
                            self.message = None;
                            self.offers.running = Some((id, package, rerun));
                        }
                        Err(e) => self.message = Some(e.to_string()),
                    }
                }
            }
            Action::SubmitStata(source, text) => {
                // a value the line before left is still being worked out: in turn
                if self.stata.after.is_some() {
                    self.stata.held.push((source, text));
                    return;
                }
                let mut offer = Vec::new();
                if let EngineState::Ready(engine) = &mut self.engine {
                    // the translator moves on only if R takes the line
                    // the helpers sent with the step before, again if R stopped there
                    if let Some((id, helpers)) = self.stata.sent_with.take() {
                        let o = engine.history().get(id).and_then(|h| h.outcome);
                        if o.is_some_and(|o| o != Outcome::Ok) {
                            self.stata.live.unsend(&helpers);
                        }
                    }
                    let before = self.stata.live.clone();
                    let step = self.stata.live.step(&text);
                    // a value only R knows first (local m = r(mean)): asked out of
                    // sight, and the line stepped again with the answer
                    if let Some(code) = step.ask {
                        let q = engine.query(&code);
                        self.stata.asking = Some((q, source, text, step.helpers));
                        return;
                    }
                    match engine.submit(source.clone(), &step.code) {
                        Ok(id) => {
                            self.stata.sent_with = Some((id, step.helpers.clone()));
                            // the value its last line leaves, once it has run
                            if let Some(code) = step.then_ask.clone() {
                                self.stata.after = Some(After::Line(id, code));
                            }
                            self.message = None;
                            if source == Source::Typed {
                                self.stata.typed(text.trim());
                            }
                            // show the data the Stata is working on
                            let data = self.stata.live.data().map(str::to_string);
                            if data.is_some() && data != before.data().map(str::to_string) {
                                self.stata.follow = data;
                            }
                            offer = step.ssc.clone();
                        }
                        Err(e) => {
                            self.stata.live = before;
                            self.message = Some(e.to_string());
                        }
                    }
                }
                // ssc install: the R package of the same name, if CRAN has one
                for name in offer {
                    self.offer_for_ssc(&name);
                }
            }
            Action::OpenStataGuide(line) => self.help_window.show_stata(&line),
            Action::SetStataMode(on) => {
                self.stata.on = on;
                // an untouched script starts over in the other language
                if on && self.script == SCRIPT_START {
                    self.script = stata_mode::SCRIPT_START.to_string();
                } else if !on && self.script == stata_mode::SCRIPT_START {
                    self.script = SCRIPT_START.to_string();
                }
                self.completions.clear();
                self.focus_command = true;
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
                self.picked_variable = None;
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
            Action::OpenDoFile => {
                let mut chooser = rfd::FileDialog::new()
                    .set_title("Run do-file")
                    .add_filter("Stata do-file", &["do", "doh", "ado"]);
                if let Some(dir) = self.engine().and_then(|e| e.working_dir()) {
                    chooser = chooser.set_directory(dir);
                }
                if let Some(path) = chooser.pick_file() {
                    self.dofile.open_file(&path);
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
            Action::OpenFreeSpec(spec) => {
                // the Reports menu's dialogs need no data frame
                self.spec_dialog.open(
                    spec,
                    rcraft_rcodegen::dialog::Context {
                        dataset: "(report)".into(),
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
                    // the translator followed an R that is gone
                    self.stata.forget_r();
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
            Action::AskRenameSession => self.renaming = Some(self.session_name.clone()),
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
            Action::OpenScript => {
                if self.script_file.is_changed(&self.script) {
                    self.script_ask = Some(ScriptAsk::Choose);
                } else if let Some(path) = self.choose_script() {
                    self.perform(ctx, Action::OpenScriptPath(path));
                }
            }
            Action::OpenScriptPath(path) => {
                if self.script_file.is_changed(&self.script) {
                    self.script_ask = Some(ScriptAsk::Open(path));
                    return;
                }
                match ScriptFile::open(&path) {
                    Ok(opened) => {
                        let stata = opened.file.is_stata();
                        self.script = opened.text;
                        self.script_file = opened.file;
                        if stata != self.stata.on {
                            self.stata.on = stata;
                            self.completions.clear();
                        }
                        self.message = None;
                    }
                    Err(e) => self.message = Some(e),
                }
            }
            Action::SaveScript => match self.script_file.path().map(|p| p.to_path_buf()) {
                Some(path) => self.perform(ctx, Action::SaveScriptTo(path)),
                None => self.perform(ctx, Action::SaveScriptAs),
            },
            Action::SaveScriptAs => {
                let (kind, ext) = if self.stata.on {
                    ("Stata do-file", "do")
                } else {
                    ("R script", "R")
                };
                let name = self
                    .script_file
                    .path()
                    .and_then(|p| p.file_name())
                    .map_or_else(
                        || format!("script.{ext}"),
                        |n| n.to_string_lossy().into_owned(),
                    );
                let mut chooser = rfd::FileDialog::new()
                    .set_title("Save script")
                    .set_file_name(&name)
                    .add_filter(kind, &[ext]);
                if let Some(dir) = self.script_folder() {
                    chooser = chooser.set_directory(dir);
                }
                if let Some(path) = chooser.save_file() {
                    self.perform(ctx, Action::SaveScriptTo(path));
                }
            }
            Action::SaveScriptTo(path) => {
                let text = self.script.clone();
                self.message = self.script_file.save(&path, &text).err();
            }
            Action::ScriptAnswer(answer) => {
                let Some(waiting) = self.script_ask.take() else {
                    return;
                };
                let Some(save_first) = answer else {
                    return;
                };
                if save_first {
                    self.perform(ctx, Action::SaveScript);
                    // not saved (cancelled or failed): nothing is replaced
                    if self.script_file.is_changed(&self.script) {
                        return;
                    }
                } else {
                    // the changes are let go
                    self.script_file = ScriptFile::unnamed(&self.script);
                }
                match waiting {
                    ScriptAsk::Open(path) => self.perform(ctx, Action::OpenScriptPath(path)),
                    ScriptAsk::Project(path) => self.perform(ctx, Action::OpenProjectPath(path)),
                    ScriptAsk::Choose => {
                        if let Some(path) = self.choose_script() {
                            self.perform(ctx, Action::OpenScriptPath(path));
                        }
                    }
                }
            }
            Action::OpenFind(replace) => {
                self.find.open = true;
                self.find.replace = replace;
                self.find.note = None;
                // what is selected is what to look for
                if let Some((a, b)) = editor_selection(ctx) {
                    if a != b {
                        let picked: String = self.script.chars().skip(a).take(b - a).collect();
                        if !picked.contains('\n') {
                            self.find.search.needle = picked;
                        }
                    }
                }
                ctx.memory_mut(|m| m.request_focus(egui::Id::new(FIND_ID)));
            }
            Action::CloseFind => {
                self.find.open = false;
                self.find.note = None;
                ctx.memory_mut(|m| m.request_focus(egui::Id::new(SCRIPT_ID)));
            }
            Action::FindStep(forward) => {
                let from = editor_selection(ctx).map_or(0, |(a, b)| if forward { b } else { a });
                match self.find.search.next(&self.script, from, forward) {
                    Some((a, b)) => {
                        select_in_editor(ctx, a, b);
                        let n = self.find.search.matches(&self.script).len();
                        self.find.note = Some(if n == 1 {
                            "1 found".into()
                        } else {
                            format!("{n} found")
                        });
                    }
                    None => self.find.note = Some("Not found".into()),
                }
            }
            Action::ReplaceOne => {
                let selected = editor_selection(ctx);
                let all = self.find.search.matches(&self.script);
                match selected.filter(|r| all.contains(r)) {
                    Some(range) => {
                        let (text, end) =
                            script_tools::replace_range(&self.script, range, &self.find.with);
                        self.script = text;
                        select_in_editor(ctx, end, end);
                        self.perform(ctx, Action::FindStep(true));
                    }
                    // nothing found is selected yet: find it first
                    None => self.perform(ctx, Action::FindStep(true)),
                }
            }
            Action::ReplaceAll => {
                let (text, n) =
                    script_tools::replace_all(&self.script, &self.find.search, &self.find.with);
                if n > 0 {
                    self.script = text;
                    select_in_editor(ctx, 0, 0);
                }
                self.find.note = Some(match n {
                    0 => "Not found".into(),
                    1 => "1 replaced".into(),
                    n => format!("{n} replaced"),
                });
            }
            Action::SaveSessionAs => {
                let stem = file_stem_of(&self.session_name);
                let mut chooser = rfd::FileDialog::new()
                    .set_title("Save session as")
                    .set_file_name(format!("{stem}.R"))
                    .add_filter("R script", &["R"])
                    .add_filter("Quarto document", &["qmd"])
                    .add_filter("R Markdown document", &["Rmd"]);
                if let Some(dir) = self.script_folder() {
                    chooser = chooser.set_directory(dir);
                }
                if let Some(path) = chooser.save_file() {
                    self.perform(ctx, Action::SaveSessionTo(path));
                }
            }
            Action::SaveSessionTo(path) => {
                let Some(engine) = self.engine() else {
                    self.message = Some("R is not running, so there is no session to save.".into());
                    return;
                };
                let title = format!("RCraft session: {}", self.session_name);
                let ending = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_ascii_lowercase());
                let text = match ending.as_deref() {
                    Some("qmd") => engine.document_text(DocKind::Quarto, &title),
                    Some("rmd") => engine.document_text(DocKind::RMarkdown, &title),
                    _ => engine.script_text(),
                };
                self.message = std::fs::write(&path, text)
                    .err()
                    .map(|e| format!("{}: {e}", path.display()));
            }
            Action::SaveSessionProjectAs => {
                let mut chooser = rfd::FileDialog::new().set_title("Folder for the Quarto project");
                if let Some(dir) = self.script_folder() {
                    chooser = chooser.set_directory(dir);
                }
                if let Some(dir) = chooser.pick_folder() {
                    self.perform(ctx, Action::SaveSessionProject(dir));
                }
            }
            Action::SaveSessionProject(dir) => {
                let Some(engine) = self.engine() else {
                    self.message = Some("R is not running, so there is no session to save.".into());
                    return;
                };
                let title = format!("RCraft session: {}", self.session_name);
                let doc = dir.join(format!("{}.qmd", file_stem_of(&self.session_name)));
                let config = dir.join("_quarto.yml");
                // nothing already there is written over
                if let Some(there) = [&config, &doc].into_iter().find(|p| p.exists()) {
                    self.message = Some(format!(
                        "{} is already there; choose an empty folder for the project.",
                        there.display()
                    ));
                    return;
                }
                let text = engine.document_text(DocKind::Quarto, &title);
                let written = std::fs::create_dir_all(&dir)
                    .and_then(|()| std::fs::write(&config, "project:\n  type: default\n"))
                    .and_then(|()| std::fs::write(&doc, text));
                self.message = written.err().map(|e| format!("{}: {e}", dir.display()));
            }
            Action::SaveProjectAs => {
                let mut chooser = rfd::FileDialog::new().set_title("Folder for the project");
                if let Some(dir) = self.script_folder() {
                    chooser = chooser.set_directory(dir);
                }
                if let Some(dir) = chooser.pick_folder() {
                    self.perform(ctx, Action::SaveProjectTo(dir));
                }
            }
            Action::SaveProjectTo(folder) => {
                let Some(engine) = self.engine() else {
                    self.message = Some("R is not running, so there is no session to save.".into());
                    return;
                };
                let log = engine.log_text().to_string();
                // a project already in the folder keeps its own file names
                let file = project::project_file(&folder);
                let proj = project::Project::read(&file).unwrap_or_else(|_| {
                    project::Project::new(&project::name_of(&folder), self.stata.on)
                });
                let text = self.script.clone();
                let written = std::fs::create_dir_all(&folder)
                    .map_err(|e| format!("{}: {e}", folder.display()))
                    .and_then(|()| self.script_file.save(&folder.join(&proj.script), &text))
                    .and_then(|()| {
                        std::fs::write(folder.join(&proj.log), log)
                            .and_then(|()| std::fs::write(&file, proj.to_text()))
                            .map_err(|e| format!("{}: {e}", folder.display()))
                    });
                if let Err(e) = written {
                    self.message = Some(e);
                    return;
                }
                let code = project::save_code(&folder, &proj, self.package_index.mirror());
                self.perform(
                    ctx,
                    Action::Submit(Source::Menu("File > Save project".into()), code),
                );
            }
            Action::OpenProject => {
                let mut chooser = rfd::FileDialog::new()
                    .set_title("Open project")
                    .add_filter("RCraft project", &[project::ENDING]);
                if let Some(dir) = self.script_folder() {
                    chooser = chooser.set_directory(dir);
                }
                if let Some(path) = chooser.pick_file() {
                    self.perform(ctx, Action::OpenProjectPath(path));
                }
            }
            Action::OpenProjectPath(path) => {
                if self.script_file.is_changed(&self.script) {
                    self.script_ask = Some(ScriptAsk::Project(path));
                    return;
                }
                let proj = match project::Project::read(&path) {
                    Ok(p) => p,
                    Err(e) => {
                        self.message = Some(e);
                        return;
                    }
                };
                let folder = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                let script = folder.join(&proj.script);
                if script.exists() {
                    self.perform(ctx, Action::OpenScriptPath(script));
                }
                let code = project::open_code(&folder, &proj);
                self.perform(
                    ctx,
                    Action::Submit(Source::Menu("File > Open project".into()), code),
                );
                // the package list waits in the Packages window, to install what is missing
                if let Ok(list) = std::fs::read_to_string(folder.join(&proj.packages)) {
                    self.packages_window.load_lock_text(&list);
                }
                self.start_folder = Some(folder);
            }
            Action::PickVariable(name) => self.picked_variable = name,
            Action::GoToLine(line) => {
                let at = script_tools::line_start(&self.script, line);
                select_in_editor(ctx, at, at);
            }
            Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }

    /// Properties (P8.1): the variable picked in Variables, or the data frame
    /// shown, as name and value pairs.
    fn properties_pane(&self, ui: &mut egui::Ui, pal: &Palette, out: &mut Vec<Action>) {
        let Some(name) = self.dataset.as_deref() else {
            return;
        };
        let Some(table) = self.engine().and_then(|e| e.table(name)) else {
            return;
        };
        let column = self
            .picked_variable
            .as_deref()
            .and_then(|v| table.columns.iter().find(|c| c.name == v));
        let rows = match column {
            Some(col) => crate::properties::of_column(col),
            None => crate::properties::of_table(name, table),
        };
        ui.separator();
        ui.horizontal(|ui| {
            pane_title(ui, pal, "Properties");
            if column.is_some()
                && ui
                    .small_button("data frame")
                    .on_hover_text("Show the data frame's properties")
                    .clicked()
            {
                out.push(Action::PickVariable(None));
            }
        });
        egui::Grid::new("properties")
            .num_columns(2)
            .spacing([8.0, 2.0])
            .show(ui, |ui| {
                for (key, value) in rows {
                    ui.label(RichText::new(key).small().color(pal.dim));
                    ui.add(egui::Label::new(RichText::new(value).small()).wrap());
                    ui.end_row();
                }
            });
    }

    /// The find bar above the Script Editor.
    fn find_bar(&mut self, ui: &mut egui::Ui, pal: &Palette, out: &mut Vec<Action>) {
        if !self.find.open {
            return;
        }
        ui.horizontal(|ui| {
            ui.label("Find");
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.find.search.needle)
                    .id(egui::Id::new(FIND_ID))
                    .desired_width(160.0),
            );
            if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                let back = ui.input(|i| i.modifiers.shift);
                out.push(Action::FindStep(!back));
                field.request_focus();
            }
            if field.has_focus() && ui.input(|i| i.key_pressed(Key::Escape)) {
                out.push(Action::CloseFind);
            }
            ui.checkbox(&mut self.find.search.match_case, "Match case");
            ui.checkbox(&mut self.find.search.whole_word, "Whole word");
            if ui.button("Next").clicked() {
                out.push(Action::FindStep(true));
            }
            if ui.button("Previous").clicked() {
                out.push(Action::FindStep(false));
            }
            if let Some(note) = &self.find.note {
                ui.label(RichText::new(note).small().color(pal.dim));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("✕").on_hover_text("Close (Esc)").clicked() {
                    out.push(Action::CloseFind);
                }
                if !self.find.replace && ui.small_button("Replace…").clicked() {
                    self.find.replace = true;
                }
            });
        });
        if self.find.replace {
            ui.horizontal(|ui| {
                ui.label("Replace with");
                ui.add(egui::TextEdit::singleline(&mut self.find.with).desired_width(160.0));
                if ui.button("Replace").clicked() {
                    out.push(Action::ReplaceOne);
                }
                if ui.button("Replace all").clicked() {
                    out.push(Action::ReplaceAll);
                }
            });
        }
    }

    /// Where the script's file choosers start: its own folder, or R's.
    fn script_folder(&self) -> Option<std::path::PathBuf> {
        self.script_file
            .path()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .or_else(|| {
                self.engine()
                    .and_then(|e| e.working_dir())
                    .map(std::path::PathBuf::from)
            })
    }

    /// File > Open script...: an R script or a Stata do-file.
    fn choose_script(&self) -> Option<std::path::PathBuf> {
        let mut chooser = rfd::FileDialog::new()
            .set_title("Open script")
            .add_filter("R script or Stata do-file", &["R", "r", "do", "doh"])
            .add_filter("All files", &["*"]);
        if let Some(dir) = self.script_folder() {
            chooser = chooser.set_directory(dir);
        }
        chooser.pick_file()
    }

    /// Asks whether to save the script's changes before another replaces it.
    fn script_question(&mut self, ctx: &egui::Context, out: &mut Vec<Action>) {
        if self.script_ask.is_none() {
            return;
        }
        let name = self
            .script_file
            .title(&self.script)
            .trim_end_matches(" •")
            .to_string();
        egui::Window::new("Save the script?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(format!("{name} has changes that are not saved."));
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        out.push(Action::ScriptAnswer(Some(true)));
                    }
                    if ui.button("Don't save").clicked() {
                        out.push(Action::ScriptAnswer(Some(false)));
                    }
                    if ui.button("Cancel").clicked() {
                        out.push(Action::ScriptAnswer(None));
                    }
                });
            });
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
    /// Tab on a Stata line, from what the session holds: the variables of the
    /// data in memory (as R last sent them) and the files of R's folder.
    fn complete_stata(&self, line: &str) -> Option<(String, Vec<String>)> {
        let variables: Vec<String> = self
            .stata
            .live
            .data()
            .and_then(|d| self.engine().and_then(|e| e.table(d)))
            .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default();
        let folder = self
            .engine()
            .and_then(|e| e.working_dir())
            .map(std::path::PathBuf::from);
        self.stata.complete(line, &variables, folder.as_deref())
    }

    /// An install started from an offer in Results has ended: if it worked,
    /// note it and run the command again when that was asked for.
    fn follow_offered_install(&mut self) {
        let Some((id, package, rerun)) = self.offers.running.clone() else {
            return;
        };
        let EngineState::Ready(engine) = &mut self.engine else {
            self.offers.running = None;
            return;
        };
        match engine.history().get(id).and_then(|h| h.outcome) {
            None => {}
            Some(Outcome::Ok) => {
                self.offers.running = None;
                self.offers.done.push((package, id));
                if let Some((source, code)) = rerun {
                    if let Err(e) = engine.submit(source, &code) {
                        self.message = Some(e.to_string());
                    }
                }
            }
            // R's messages in Results say why
            Some(_) => self.offers.running = None,
        }
    }

    #[cfg(test)]
    fn complete_rc(&mut self) -> Option<String> {
        let line = self.command.clone();
        self.complete_rc_line(&line)
    }

    /// Completes an `rc` line's start (the text before the cursor).
    fn complete_rc_line(&mut self, before: &str) -> Option<String> {
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
        let c = rc::complete(before, &cran, &mine)?;
        if c.options.is_empty() {
            self.completions.clear();
            return None;
        }
        let typed = before.get(c.from..).unwrap_or("").to_string();
        let common = rc::common_prefix(&c.options);
        let mut line = before.get(..c.from).unwrap_or("").to_string();
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
    /// `ssc install name` in Stata mode (P2B.11): the R package of the same
    /// name, through `rc install` (the list to say yes to), when CRAN has
    /// one; otherwise it says so, and nothing is installed.
    fn offer_for_ssc(&mut self, name: &str) {
        self.package_index.ensure_loaded();
        let on_cran = self.package_index.index().map(|ix| ix.get(name).is_some());
        let loading = matches!(
            self.package_index.status(),
            rcraft_engine::IndexStatus::Loading
        );
        let shown = format!("ssc install {name}");
        let text = match on_cran {
            Some(true) => {
                let install = Rc::Install {
                    packages: vec![name.to_string()],
                    version: None,
                    from: rcraft_rcodegen::packages::InstallFrom::Default,
                };
                self.run_rc(install, &format!("rc install {name}"));
                return;
            }
            Some(false) => format!(
                "No R package called {name} on CRAN, so nothing is offered: {name} is a Stata add-on, \
                 and its commands are not translated. rc search {name} looks for R packages about it."
            ),
            None if loading => format!(
                "The CRAN package list is being read; type rc install {name} in a moment if CRAN has it."
            ),
            None => format!(
                "Whether CRAN has an R package called {name} is not known yet: the package list is not \
                 on this computer (Packages > Find and install downloads it). Then rc install {name} \
                 installs it, if it is there."
            ),
        };
        if let EngineState::Ready(engine) = &mut self.engine {
            engine.echo(&shown, &text, Outcome::Error);
        }
    }

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
        self.follow_offered_install();
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

        // a file dropped on the window opens the dialog on it (a do-file, the do-file window)
        let dropped = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone()));
        if let Some(path) = dropped {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if matches!(ext.as_str(), "do" | "doh") {
                self.dofile.open_file(&path);
            } else {
                self.import.set_recent(self.recent.paths());
                self.import.open_with(&path);
            }
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

        // a typed Stata line waiting for a value from R
        if let Some((q, source, text, helpers)) = self.stata.asking.clone() {
            let answer = match &self.engine {
                EngineState::Ready(e) => e.query_result(q).map(|r| {
                    if !r.ok {
                        self.stata.live.unsend(&helpers);
                    }
                    if r.ok {
                        Ok(r.text.clone())
                    } else {
                        Err(format!("R could not work the value out: {}", r.text.trim()))
                    }
                }),
                _ => Some(Err("R is not running".into())),
            };
            match answer {
                Some(a) => {
                    self.stata.asking = None;
                    self.stata.live.give(a);
                    actions.push(Action::SubmitStata(source, text));
                }
                None => ctx.request_repaint_after(Duration::from_millis(30)),
            }
        }

        // a value a typed line left (levelsof, local()), asked once it has run
        match (self.stata.after.clone(), &mut self.engine) {
            (Some(After::Line(id, code)), EngineState::Ready(e)) => {
                match e.history().get(id).and_then(|h| h.outcome) {
                    Some(Outcome::Ok) => {
                        let q = e.query(&code);
                        self.stata.after = Some(After::Answer(q));
                        ctx.request_repaint_after(Duration::from_millis(30));
                    }
                    // R stopped at the line: the next step asks again
                    Some(_) => self.stata.after = None,
                    None => ctx.request_repaint_after(Duration::from_millis(30)),
                }
            }
            (Some(After::Answer(q)), EngineState::Ready(e)) => match e.query_result(q) {
                Some(r) => {
                    let a = if r.ok {
                        Ok(r.text.clone())
                    } else {
                        Err(format!("R could not work the value out: {}", r.text.trim()))
                    };
                    self.stata.live.give(a);
                    self.stata.after = None;
                }
                None => ctx.request_repaint_after(Duration::from_millis(30)),
            },
            (Some(_), _) => self.stata.after = None,
            (None, _) => {}
        }
        if self.stata.after.is_none() && !self.stata.held.is_empty() {
            for (source, text) in std::mem::take(&mut self.stata.held) {
                actions.push(Action::SubmitStata(source, text));
            }
        }

        // the do-file run moves on as R lets it, in the session it started in
        let session = self.session_name.clone();
        let mut dofile_out = Vec::new();
        let was_moving = self.dofile.is_moving();
        if let EngineState::Ready(engine) = &mut self.engine {
            dofile_out.extend(self.dofile.poll(engine, &session));
        }
        // the run moved on after this frame drew Results: draw them again
        if was_moving {
            ctx.request_repaint();
        }
        dofile_out.extend(self.dofile.show(ctx, &pal, &session));
        // save the script's changes before another replaces them?
        self.script_question(ctx, &mut actions);
        if self.dofile.is_running() {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
        for out in dofile_out {
            match out {
                DoFileOut::Guide(line) => actions.push(Action::OpenStataGuide(line)),
                DoFileOut::Follow(name) => self.stata.follow = Some(name),
            }
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

        if let Some(mut text) = self.renaming.take() {
            let mut open = true;
            let mut decision: Option<bool> = None;
            egui::Window::new("Rename session")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let edit = ui.add(egui::TextEdit::singleline(&mut text).desired_width(220.0));
                    edit.request_focus();
                    let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Rename").clicked() || enter {
                            decision = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            decision = Some(false);
                        }
                    });
                });
            if decision == Some(true) {
                self.rename_session(&text);
            }
            if open && decision.is_none() {
                self.renaming = Some(text);
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
                if ui
                    .button("Example data…")
                    .on_hover_text("Data frames to practise on, from R's own datasets package")
                    .clicked()
                {
                    out.push(Action::OpenFreeSpec(&rcraft_catalog::examples::EXAMPLE_DATA));
                    ui.close_menu();
                }
                if ui
                    .button("Run do-file…")
                    .on_hover_text("Run a Stata do-file on R, unchanged")
                    .clicked()
                {
                    out.push(Action::OpenDoFile);
                    ui.close_menu();
                }
                if ui.button("Open script…").clicked() {
                    out.push(Action::OpenScript);
                    ui.close_menu();
                }
                if ui.button("Save script").clicked() {
                    out.push(Action::SaveScript);
                    ui.close_menu();
                }
                if ui.button("Save script as…").clicked() {
                    out.push(Action::SaveScriptAs);
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .button("Save session as…")
                    .on_hover_text(
                        "The commands that ran without error, as an R script, a Quarto or an R \
                         Markdown document",
                    )
                    .clicked()
                {
                    out.push(Action::SaveSessionAs);
                    ui.close_menu();
                }
                if ui
                    .button("Open project…")
                    .on_hover_text("A project saved by RCraft: its script, objects and package list")
                    .clicked()
                {
                    out.push(Action::OpenProject);
                    ui.close_menu();
                }
                if ui
                    .button("Save project…")
                    .on_hover_text(
                        "The script, R's objects, the log and the package list together in a folder",
                    )
                    .clicked()
                {
                    out.push(Action::SaveProjectAs);
                    ui.close_menu();
                }
                if ui
                    .button("Save session as Quarto project…")
                    .on_hover_text("A folder with _quarto.yml and the session as a Quarto document")
                    .clicked()
                {
                    out.push(Action::SaveSessionProjectAs);
                    ui.close_menu();
                }
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
            ui.menu_button("Reports", |ui| {
                for spec in rcraft_catalog::report::REPORT_SPECS {
                    if ui
                        .button(format!("{}…", spec.entry))
                        .on_hover_text(spec.about)
                        .clicked()
                    {
                        out.push(Action::OpenFreeSpec(spec));
                        ui.close_menu();
                    }
                }
                ui.separator();
                ui.label(
                    RichText::new("A table goes in from its data frame's menu in Objects")
                        .small()
                        .color(pal.dim),
                );
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
                if ui.button("Packages from another R version…").clicked() {
                    out.push(Action::OpenPackages(PackagesTab::OtherR));
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
                if ui.button("Rename this session…").clicked() {
                    out.push(Action::AskRenameSession);
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
                if ui.button("Stata and R: what differs…").clicked() {
                    out.push(Action::OpenStataGuide("general".into()));
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
                    for click in clicks {
                        out.push(match click {
                            VariableClick::Insert(name) => Action::Insert(name),
                            VariableClick::Pick(name) => Action::PickVariable(Some(name)),
                        });
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
        self.properties_pane(ui, pal, out);
        ui.separator();
        let count = self.engine().map_or(0, |e| e.history().len());
        pane_title(ui, pal, &format!("History  ({count})"));
        egui::ScrollArea::vertical()
            .id_salt("history")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if let Some(engine) = self.engine() {
                    history_list(ui, pal, engine.history(), self.stata.on, out);
                }
            });
    }

    fn script_pane(&mut self, ui: &mut egui::Ui, pal: &Palette, busy: bool, out: &mut Vec<Action>) {
        let stata = self.stata.on;
        // Ctrl+S saves the script, Ctrl+O opens one
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S)) {
            out.push(Action::SaveScript);
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::O)) {
            out.push(Action::OpenScript);
        }
        // Ctrl+F finds in the script, Ctrl+H replaces
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::F)) {
            out.push(Action::OpenFind(false));
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::H)) {
            out.push(Action::OpenFind(true));
        }
        ui.horizontal(|ui| {
            let title = self.script_file.title(&self.script);
            let label = ui.label(RichText::new(&title).strong().color(pal.dim));
            if let Some(path) = self.script_file.path() {
                label.on_hover_text(path.display().to_string());
            }
            stata_switch(ui, stata, out);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let run_all = ui.add_enabled(!busy, egui::Button::new("Run all"));
                if run_all.clicked() {
                    let script = self.script.clone();
                    out.push(if stata {
                        Action::SubmitStata(Source::Script, script)
                    } else {
                        Action::Submit(Source::Script, script)
                    });
                }
                if stata
                    && ui
                        .button("Copy R")
                        .on_hover_text("Copies the whole script's R, RCraft's helpers first")
                        .clicked()
                {
                    let r = self.stata.translation(&self.script).script();
                    ui.ctx().copy_text(r);
                }
                // the script's sections and functions; a pick moves the cursor there
                let sections = script_tools::outline(&self.script);
                if !sections.is_empty() {
                    ui.menu_button("Outline", |ui| {
                        for sec in &sections {
                            let indent = "   ".repeat(sec.depth.saturating_sub(1).min(3));
                            if ui.button(format!("{indent}{}", sec.title)).clicked() {
                                out.push(Action::GoToLine(sec.line));
                                ui.close_menu();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Sections (# Title ----) and functions in the script");
                }
                if ui
                    .button("Find")
                    .on_hover_text("Find and replace in the script (Ctrl+F, Ctrl+H)")
                    .clicked()
                {
                    out.push(Action::OpenFind(false));
                }
            });
        });
        self.find_bar(ui, pal, out);
        let editor = egui::Id::new(SCRIPT_ID);
        // Tab in Stata mode completes the line up to the cursor, as on the
        // Command line (taken before the editor would insert a tab)
        if stata
            && ui.memory(|m| m.has_focus(editor))
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab))
        {
            let at = egui::TextEdit::load_state(ui.ctx(), editor)
                .and_then(|s| s.cursor.char_range())
                .map(|r| r.primary.index);
            let done = at.and_then(|at| {
                stata_mode::complete_at(&self.script, at, |line| self.complete_stata(line))
            });
            match done {
                Some((text, cursor, shown)) => {
                    self.script = text;
                    let mut state =
                        egui::TextEdit::load_state(ui.ctx(), editor).unwrap_or_default();
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(
                            egui::text::CCursor::new(cursor),
                        )));
                    state.store(ui.ctx(), editor);
                    self.stata.script_choices =
                        (!shown.is_empty()).then(|| (self.script.clone(), shown));
                }
                None => self.stata.script_choices = None,
            }
        }
        if let Some((for_script, shown)) = &self.stata.script_choices {
            if stata && *for_script == self.script {
                ui.label(
                    RichText::new(shown.join("   "))
                        .monospace()
                        .small()
                        .color(pal.accent),
                );
            }
        }
        let draw_editor = |ui: &mut egui::Ui, script: &mut String| {
            egui::ScrollArea::vertical()
                .id_salt("script")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::TextEdit::multiline(script)
                        .id(editor)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(8)
                        .lock_focus(true)
                        .show(ui)
                })
                .inner
        };
        let output = if stata {
            // two panes: the Stata, and its R line by line, the cursor's marked
            let cursor_line = egui::TextEdit::load_state(ui.ctx(), editor)
                .and_then(|s| s.cursor.char_range())
                .map_or(0, |r| {
                    stata_mode::line_of(&self.script, r.primary.index) + 1
                });
            let mut picked = None;
            let shown = ui.columns(2, |cols| {
                let [left, right] = cols else {
                    return None;
                };
                let shown = draw_editor(left, &mut self.script);
                let t = self.stata.translation(&self.script);
                egui::ScrollArea::both()
                    .id_salt("script-r")
                    .auto_shrink([false, false])
                    .show(right, |ui| {
                        picked = stata_mode::translation_pane(ui, pal, t, cursor_line);
                    });
                Some(shown)
            });
            // a click on an R block selects its Stata
            // a click on a note opens the guide on its line's command
            if let Some(stata_mode::Clicked::Guide(i)) = picked {
                if let Some(l) = self.stata.translation(&self.script).lines.get(i) {
                    out.push(Action::OpenStataGuide(l.stata.clone()));
                }
            }
            let picked = match picked {
                Some(stata_mode::Clicked::Line(i)) => Some(i),
                _ => None,
            };
            let span = picked.and_then(|i| {
                self.stata
                    .translation(&self.script)
                    .lines
                    .get(i)
                    .map(|l| l.span)
            });
            if let Some(span) = span {
                let (a, b) = stata_mode::char_range(&self.script, span.line, span.lines);
                let mut state = egui::TextEdit::load_state(ui.ctx(), editor).unwrap_or_default();
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(a),
                        egui::text::CCursor::new(b),
                    )));
                state.store(ui.ctx(), editor);
                ui.memory_mut(|m| m.request_focus(editor));
            }
            shown
        } else {
            Some(draw_editor(ui, &mut self.script))
        };
        let Some(output) = output else {
            return;
        };
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
                out.push(if stata {
                    Action::SubmitStata(Source::Script, code)
                } else {
                    Action::Submit(Source::Script, code)
                });
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
                EngineState::Ready(engine) => {
                    let cran =
                        |name: &str| self.package_index.index().map(|ix| ix.get(name).is_some());
                    results_list(ui, pal, engine, self.stata.on, &self.offers, &cran, out)
                }
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
            stata_switch(ui, self.stata.on, out);
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
        // where the cursor goes in a recalled line, in characters (the end if None)
        let mut cursor_at: Option<usize> = None;
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
                if self.stata.on {
                    // the Stata typed, not the R it became
                    if back {
                        recalled = self.stata.previous();
                    } else if forward {
                        recalled = self.stata.next();
                    }
                } else if back {
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
        if let Some(ask) = self.tab_pending.clone() {
            let answer = self
                .engine()
                .and_then(|e| e.query_result(ask.id))
                .map(|r| (r.ok, r.text.clone()));
            match answer {
                Some((ok, text)) => {
                    self.tab_pending = None;
                    if ok && self.command == ask.line {
                        if let Some(c) = tab::parse_answer(&ask.before, &text) {
                            let (done, cursor, shown) =
                                tab::finish(&ask.before, &ask.after, &c, &ask.close);
                            self.completions = shown;
                            recalled = Some(done);
                            cursor_at = Some(cursor);
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
        // Each completes the text before the cursor; what follows it stays.
        if has_focus && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) {
            let (before, after) = split_at_cursor(ui.ctx(), id, &self.command);
            let t = before.trim_start();
            if self.stata.on {
                // a Stata line: completed here, from what the session holds
                match self.complete_stata(&before) {
                    Some((done, shown)) => {
                        self.completions = shown;
                        cursor_at = Some(done.chars().count());
                        recalled = Some(done + &after);
                    }
                    None => self.completions.clear(),
                }
            } else if t == "rc" || t.starts_with("rc ") || t.starts_with("rc\t") {
                if let Some(done) = self.complete_rc_line(&before) {
                    cursor_at = Some(done.chars().count());
                    recalled = Some(done + &after);
                }
            } else if !busy && !before.trim().is_empty() && self.tab_pending.is_none() {
                let line = self.command.clone();
                if let EngineState::Ready(engine) = &mut self.engine {
                    let (code, close) = tab::query(&before);
                    let id = engine.query(&code);
                    self.tab_pending = Some(TabAsk {
                        id,
                        line,
                        before,
                        after,
                        close,
                    });
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
                .hint_text(if self.stata.on {
                    "Type Stata and press Enter"
                } else {
                    "Type R code and press Enter"
                })
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
            match cursor_at {
                Some(at) => cursor_to(ui.ctx(), id, at),
                None => cursor_to_end(ui.ctx(), id, &self.command),
            }
        }
        if submit && !self.command.trim().is_empty() && !busy {
            let code = std::mem::take(&mut self.command);
            self.recall.reset();
            self.completions.clear();
            out.push(if self.stata.on {
                Action::SubmitStata(Source::Typed, code)
            } else {
                match rc::parse(&code) {
                    Some(Ok(command)) => Action::Rc(command, code),
                    Some(Err(why)) => Action::RcError(why, code),
                    None => match help::parse_command(&code) {
                        Some(request) => Action::Help(request, code),
                        None => Action::Submit(Source::Typed, code),
                    },
                }
            });
        }
        if self.focus_command {
            ui.memory_mut(|m| m.request_focus(id));
            self.focus_command = false;
        }
    }
}

/// The Stata mode switch, on the Command and Script panes alike.
fn stata_switch(ui: &mut egui::Ui, on: bool, out: &mut Vec<Action>) {
    let response = ui
        .selectable_label(on, "Stata")
        .on_hover_text("Stata mode: type Stata; RCraft translates it to R and runs the R");
    if response.clicked() {
        out.push(Action::SetStataMode(!on));
    }
}

fn pane_title(ui: &mut egui::Ui, pal: &Palette, text: &str) {
    ui.label(RichText::new(text).strong().color(pal.dim));
}

/// In Stata mode a click puts back a Stata line (the `# Stata:` comment its R starts
/// with), else the code.
fn history_list(
    ui: &mut egui::Ui,
    pal: &Palette,
    history: &History,
    stata: bool,
    out: &mut Vec<Action>,
) {
    let entries = history.entries();
    let start = entries.len().saturating_sub(HISTORY_SHOWN);
    for entry in entries.iter().skip(start) {
        let first = stata_mode::history_line(&entry.code);
        let more = if entry.code.trim_end().lines().count() > 1 {
            " …"
        } else {
            ""
        };
        // a Stata line as Stata shows it
        let shown = match first.strip_prefix(rcraft_statatr::data_r::STATA_COMMENT) {
            Some(line) => format!(". {line}"),
            None => format!("{first}{more}"),
        };
        let text = RichText::new(shown).monospace();
        let text = match entry.outcome {
            Some(Outcome::Ok) | None => text,
            Some(_) => text.color(pal.error),
        };
        let response = ui
            .selectable_label(false, text)
            .on_hover_text("Click to put this back in the Command line");
        if response.clicked() {
            let back = match first.strip_prefix(rcraft_statatr::data_r::STATA_COMMENT) {
                Some(line) if stata => line.to_string(),
                _ => entry.code.trim_end().to_string(),
            };
            out.push(Action::SetCommand(back));
        }
    }
}

/// The commands and what they printed. A Stata-mode command shows its Stata
/// as Stata does (`. summarize y`), the R it became under it and the notes;
/// in Stata mode a click on its Stata puts that line back.
fn results_list(
    ui: &mut egui::Ui,
    pal: &Palette,
    engine: &Engine,
    stata: bool,
    offers: &InstallOffers,
    on_cran: &dyn Fn(&str) -> Option<bool>,
    out: &mut Vec<Action>,
) {
    for block in engine.results().blocks() {
        // a Stata command's output reads as Stata's text, never as a grid
        let mut as_stata = false;
        if let Some(entry) = engine.history().get(block.command) {
            match stata_mode::parts(&entry.code) {
                Some(parts) => {
                    as_stata = true;
                    stata_parts(ui, pal, &parts, stata, out);
                }
                None => {
                    for (i, line) in stata_mode::shown_lines(&entry.code).iter().enumerate() {
                        let prompt = if i == 0 { "> " } else { "+ " };
                        mono(ui, &format!("{prompt}{line}"), pal.accent, true);
                    }
                }
            }
        }
        for (n, item) in block.items.iter().enumerate() {
            match item {
                Item::Text {
                    table: Some(table), ..
                } if !as_stata => {
                    crate::result_table::show(ui, pal, table, (block.command, n));
                }
                Item::Text { stream, text, .. } => {
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
                    // a Stata line: Stata's words and r(#) where RCraft knows them
                    let stata_text = rcraft_statatr::dofile::stata_error_text(message);
                    let text = if as_stata && stata_text != *message {
                        stata_text
                    } else if call.is_empty() {
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
        if block.outcome.is_some() {
            if let Some((package, need)) = needed_package(&block.items) {
                let entry = engine.history().get(block.command);
                // an offer's own install never offers another
                let from_install =
                    entry.is_some_and(|e| e.source == Source::Menu("Packages > Install".into()));
                if !from_install {
                    // a Stata block's code is not R to run again as it is
                    let rerun = entry
                        .filter(|_| !as_stata)
                        .map(|e| (e.source.clone(), e.code.clone()));
                    let offer = Offer {
                        command: block.command,
                        package: &package,
                        need,
                        rerun,
                    };
                    install_offer(ui, pal, &offer, engine.is_busy(), offers, on_cran, out);
                }
            }
        }
        ui.add_space(6.0);
    }
}

/// P1A.10: installs offered in Results.
#[derive(Default)]
struct InstallOffers {
    /// The install started from an offer: its command, the package, and the
    /// command to run again once it has worked.
    running: Option<OfferedInstall>,
    /// Packages installed from an offer, with the install's command.
    done: Vec<(String, CommandId)>,
}

/// An install started from an offer: its command, the package, and the
/// command (with where it came from) to run again once it has worked.
type OfferedInstall = (CommandId, String, Option<(Source, String)>);

/// One command's offer.
struct Offer<'a> {
    command: CommandId,
    package: &'a str,
    need: Need,
    rerun: Option<(Source, String)>,
}

/// The package a command's output says must be installed, if any: in its
/// error, a warning (`require()`), or a message (`requireNamespace()`).
fn needed_package(items: &[Item]) -> Option<(String, Need)> {
    items.iter().find_map(|item| match item {
        Item::Error { message, .. } | Item::Warning(message) => package_to_install(message),
        Item::Text {
            stream: Stream::Stderr,
            text,
            ..
        } => package_to_install(text),
        _ => None,
    })
}

/// R code an offer runs: the install into RCraft's library, then a check
/// that the package now loads (install.packages only warns when it fails).
fn offer_install_code(package: &str, library: Option<&str>) -> String {
    let install = crate::packages::install_code(&[package], library);
    format!(
        "{install}\nif (!requireNamespace({name}, quietly = TRUE)) \
         stop({why}, call. = FALSE)",
        name = rcraft_rcodegen::r_string(package),
        why = rcraft_rcodegen::r_string(&format!(
            "The package {package} could not be installed; R's messages above say why."
        )),
    )
}

/// Says what a command needs and offers to install it, with or without
/// running the command again.
fn install_offer(
    ui: &mut egui::Ui,
    pal: &Palette,
    offer: &Offer,
    busy: bool,
    offers: &InstallOffers,
    on_cran: &dyn Fn(&str) -> Option<bool>,
    out: &mut Vec<Action>,
) {
    let package = offer.package;
    let what = match offer.need {
        Need::Missing => format!("The R package {package} is not installed."),
        Need::Newer => format!("This needs a newer version of the R package {package}."),
        Need::Reinstall => format!(
            "The R package {package} was installed for an older R and must be installed again."
        ),
    };
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(RichText::new(what).color(pal.warning));
        let installed = offers
            .done
            .iter()
            .any(|(p, at)| p == package && *at > offer.command);
        if installed {
            ui.label(RichText::new(format!("{package} has been installed since.")).color(pal.dim));
            return;
        }
        if offers
            .running
            .as_ref()
            .is_some_and(|(_, p, _)| p == package)
        {
            ui.label(RichText::new(format!("Installing {package}…")).color(pal.dim));
            return;
        }
        if on_cran(package) == Some(false) {
            ui.label(
                RichText::new(format!(
                    "{package} is not on CRAN. Packages > Other sources installs from \
                     Bioconductor, r-universe, GitHub or a file."
                ))
                .small()
                .color(pal.dim),
            );
            return;
        }
        ui.label(
            RichText::new(
                "RCraft can install it from CRAN (with the packages it needs) into RCraft's own \
                 library. The install runs as a command, so it shows here and in the log.",
            )
            .small()
            .color(pal.dim),
        );
        let free = !busy && offers.running.is_none();
        ui.horizontal(|ui| {
            if let Some(rerun) = &offer.rerun {
                if ui
                    .add_enabled(
                        free,
                        egui::Button::new(format!("Install {package} and run again")),
                    )
                    .clicked()
                {
                    out.push(Action::InstallNeeded {
                        package: package.to_string(),
                        rerun: Some(rerun.clone()),
                    });
                }
            }
            if ui
                .add_enabled(free, egui::Button::new(format!("Install {package}")))
                .clicked()
            {
                out.push(Action::InstallNeeded {
                    package: package.to_string(),
                    rerun: None,
                });
            }
        });
    });
}

/// A Stata-mode command in Results: its Stata, its R, its notes.
fn stata_parts(
    ui: &mut egui::Ui,
    pal: &Palette,
    parts: &[stata_mode::Part],
    stata: bool,
    out: &mut Vec<Action>,
) {
    use stata_mode::Part;
    // the Stata line a note is about, for its page in the guide
    let mut line = "";
    for part in parts {
        match part {
            Part::Helpers(helpers) => mono(ui, helpers, pal.dim, false),
            Part::Stata(s) => {
                line = s;
                let text = RichText::new(format!(". {s}"))
                    .monospace()
                    .strong()
                    .color(pal.accent);
                let label = egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend);
                if stata {
                    let response = ui
                        .add(label.sense(egui::Sense::click()))
                        .on_hover_text("Click to put this Stata back in the Command line");
                    if response.clicked() {
                        out.push(Action::SetCommand(s.clone()));
                    }
                } else {
                    ui.add(label);
                }
            }
            Part::R(r) => mono(ui, &format!("  {r}"), pal.dim, false),
            Part::Note(n) => {
                let note = RichText::new(format!("  note: {n}"))
                    .small()
                    .color(pal.warning);
                guide_link(ui, note, line, out);
            }
            Part::Refused(why) => {
                let why = RichText::new(format!("  not translated: {why}")).color(pal.error);
                guide_link(ui, why, line, out);
            }
        }
    }
}

/// A note under a Stata line: a click opens the differences guide on its
/// command.
fn guide_link(ui: &mut egui::Ui, text: RichText, line: &str, out: &mut Vec<Action>) {
    let response = ui
        .add(egui::Label::new(text).sense(egui::Sense::click()))
        .on_hover_text("Click for where R and Stata differ here");
    if response.clicked() {
        out.push(Action::OpenStataGuide(line.to_string()));
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
    cursor_to(ctx, id, text.chars().count());
}

/// Puts the text box's cursor `at` characters into its text.
fn cursor_to(ctx: &egui::Context, id: egui::Id, at: usize) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let at = egui::text::CCursor::new(at);
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(at)));
    state.store(ctx, id);
}

/// The text box's text before and after its cursor (the end when it has none;
/// the far end of a selection).
fn split_at_cursor(ctx: &egui::Context, id: egui::Id, text: &str) -> (String, String) {
    let at = egui::TextEdit::load_state(ctx, id)
        .and_then(|s| s.cursor.char_range())
        .map_or(usize::MAX, |r| r.primary.index.max(r.secondary.index));
    split_at_char(text, at)
}

fn split_at_char(text: &str, at: usize) -> (String, String) {
    let byte = text.char_indices().nth(at).map_or(text.len(), |(b, _)| b);
    let (before, after) = text.split_at(byte);
    (before.to_string(), after.to_string())
}

/// A Tab completion asked of R: the line when it was asked, the text before
/// and after the cursor, and what closes a single finished choice.
#[derive(Clone, Debug)]
struct TabAsk {
    id: QueryId,
    line: String,
    before: String,
    after: String,
    close: String,
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
            EngineState::Ready(e) => {
                e.is_quiet()
                    && !self.help_window.is_busy()
                    && !self.dofile.is_running()
                    && self.stata.asking.is_none()
                    && self.stata.after.is_none()
                    && self.stata.held.is_empty()
            }
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
            "other" => PackagesTab::OtherR,
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
            .chain(rcraft_catalog::report::MODEL_REPORTS)
            .chain(rcraft_catalog::report::TABLE_REPORTS)
            .chain(rcraft_catalog::report::REPORT_SPECS)
            .chain(rcraft_catalog::examples::SPECS)
            .chain(rcraft_catalog::save::SPECS)
            .chain(rcraft_catalog::plot_export::PLOT_SPECS)
            .find(|s| s.entry.eq_ignore_ascii_case(entry))
            .ok_or_else(|| format!("no dialog called {entry}"))?;
        if rcraft_catalog::report::REPORT_SPECS
            .iter()
            .chain(rcraft_catalog::examples::SPECS)
            .any(|s| s.entry == spec.entry)
        {
            self.perform(ctx, Action::OpenFreeSpec(spec));
        } else if spec.entry == rcraft_catalog::plot_export::EXPORT_PLOT.entry {
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

    /// Turns Stata mode on or off (pictures and tests).
    pub fn set_stata_mode(&mut self, ctx: &egui::Context, on: bool) {
        self.perform(ctx, Action::SetStataMode(on));
    }

    /// Runs a line of Stata as if typed in the Command window (pictures).
    pub fn submit_stata(&mut self, ctx: &egui::Context, code: &str) {
        self.perform(ctx, Action::SubmitStata(Source::Typed, code.to_string()));
    }

    /// Opens the differences guide on the command this Stata line runs (pictures).
    pub fn show_stata_guide(&mut self, line: &str) {
        self.help_window.show_stata(line);
    }

    /// Opens the do-file window on this file, and runs it if asked (pictures).
    pub fn open_dofile(&mut self, path: &std::path::Path, run: bool) {
        self.dofile.open_file(path);
        if run {
            let session = self.session_name.clone();
            self.dofile.run_now(&session);
        }
    }

    /// Puts this text in the Script Editor (pictures).
    pub fn set_script(&mut self, text: &str) {
        self.script = text.to_string();
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

/// The checks that make the Stata mode entries and Vignettes Verified.
#[cfg(test)]
mod menu_checks;

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_statatr::dofile::Ran;

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
        // renaming: the label follows; blank keeps the name; long names are cut
        app.rename_session("  Survey\nanalysis ");
        assert_eq!(app.session_name, "Survey analysis");
        app.rename_session("   ");
        assert_eq!(app.session_name, "Survey analysis");
        app.rename_session(&"x".repeat(80));
        assert_eq!(app.session_name.chars().count(), 30);
        app.perform(&ctx, Action::AskRenameSession);
        assert_eq!(app.renaming.as_deref(), Some(app.session_name.as_str()));
        app.renaming = None;
        app.rename_session("R 2");
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
    fn ssc_install_offers_the_r_package_of_that_name_or_says_there_is_none() {
        let ctx = egui::Context::default();
        let mut app = app_with_sample_index(&ctx);
        app.keep_settings(false);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        app.perform(&ctx, Action::SetStataMode(true));
        // an R package of that name: the list to say yes to, nothing installed
        app.perform(
            &ctx,
            Action::SubmitStata(Source::Typed, "ssc install fixest".into()),
        );
        assert!(app.packages_window.open);
        assert!(frames_until(&mut app, &ctx, |a| a
            .packages_window
            .has_confirmation()));
        let code = app.packages_window.confirmation_code().unwrap_or_default();
        assert!(code.contains("install.packages(\"fixest\""), "{code}");
        // none: said so in Results
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        app.perform(
            &ctx,
            Action::SubmitStata(Source::Typed, "ssc install estout".into()),
        );
        let said = app
            .engine()
            .and_then(|e| e.results().blocks().last().cloned())
            .map(|b| {
                b.items
                    .iter()
                    .map(|i| match i {
                        Item::Text { text, .. } => text.clone(),
                        Item::Error { message, .. } => message.clone(),
                        _ => String::new(),
                    })
                    .collect::<String>()
            })
            .unwrap_or_default();
        assert!(said.contains("No R package called estout"), "{said}");
    }

    #[test]
    fn stata_mode_runs_each_line_after_the_ones_before() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        let dir = std::env::temp_dir().join(format!("rcraft-stata-mode-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let file = dir
            .join("cars.dta")
            .display()
            .to_string()
            .replace('\\', "/");
        app.perform(
            &ctx,
            Action::Submit(
                Source::Typed,
                format!(
                    "stopifnot(requireNamespace(\"haven\", quietly = TRUE), requireNamespace(\"dplyr\", quietly = TRUE)); \
                     haven::write_dta(data.frame(x = c(1, 2, 3), y = c(2, 4, 7)), \"{file}\")"
                ),
            ),
        );
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        let made = app
            .engine()
            .and_then(|e| e.history().entries().last().and_then(|h| h.outcome));
        if made != Some(Outcome::Ok) {
            eprintln!("skipped: haven and dplyr are needed");
            return;
        }
        // the switch: an untouched script starts over as Stata
        app.perform(&ctx, Action::SetStataMode(true));
        assert!(app.stata.on);
        assert_eq!(app.script, stata_mode::SCRIPT_START);
        let run = |app: &mut App, line: &str| {
            app.perform(&ctx, Action::SubmitStata(Source::Typed, line.to_string()));
            assert!(frames_until(app, &ctx, |a| a.is_settled()), "{line}");
        };
        run(&mut app, &format!("use \"{file}\""));
        // the data the Stata works on is shown
        assert!(frames_until(&mut app, &ctx, |a| a.dataset.as_deref() == Some("cars")));
        run(&mut app, "summarize y");
        run(&mut app, "display r(mean)");
        assert!(
            newest_text(&app).contains("4.3333333"),
            "{}",
            newest_text(&app)
        );
        // the helpers went to R once, with the line that needed them, and fold in Results
        let codes: Vec<String> = app
            .engine()
            .map(|e| {
                e.history()
                    .entries()
                    .iter()
                    .map(|h| h.code.clone())
                    .collect()
            })
            .unwrap_or_default();
        let summarize = codes
            .iter()
            .find(|c| c.contains("# Stata: summarize y"))
            .cloned()
            .unwrap_or_default();
        assert!(
            summarize.contains("stata_summarize <- function"),
            "{summarize}"
        );
        assert!(
            stata_mode::shown_lines(&summarize).len() < 6,
            "{:?}",
            stata_mode::shown_lines(&summarize)
        );
        let display = codes
            .iter()
            .find(|c| c.contains("# Stata: display r(mean)"))
            .cloned()
            .unwrap_or_default();
        // what came before (stata_fmt came with summarize's table) does not come again
        assert!(!display.contains("<- function"), "{display}");
        // a line that cannot be translated says why and leaves the session as it was
        run(&mut app, "xtlogit y x");
        assert!(
            codes_last(&app).contains("not translated"),
            "{}",
            codes_last(&app)
        );
        // Page Up brings back the Stata typed
        assert_eq!(app.stata.previous().as_deref(), Some("xtlogit y x"));
        // Tab from the data's variables
        let vars: Vec<String> = app
            .engine()
            .and_then(|e| e.table("cars"))
            .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default();
        assert_eq!(
            app.stata.complete("regress y if x", &vars, None),
            Some(("regress y if x".to_string(), Vec::new()))
        );
        // a restart forgets what the translator followed; switching off restores R's script
        app.perform(&ctx, Action::Restart);
        assert_eq!(app.stata.live.data(), None);
        app.perform(&ctx, Action::SetStataMode(false));
        assert_eq!(app.script, SCRIPT_START);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_note_opens_the_differences_guide_on_its_command() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        app.perform(&ctx, Action::OpenStataGuide("qui: reg y x".into()));
        assert!(app.help_window.open);
        assert_eq!(app.help_window.shown_stata(), Some("regress"));
        app.perform(&ctx, Action::OpenStataGuide("tab2 g h".into()));
        assert_eq!(app.help_window.shown_stata(), Some("tabulate"));
        // a command with no page of its own: the page on what differs everywhere
        app.perform(&ctx, Action::OpenStataGuide("xtlogit y x".into()));
        assert_eq!(app.help_window.shown_stata(), Some("general"));
    }

    fn codes_last(app: &App) -> String {
        app.engine()
            .and_then(|e| e.history().entries().last().map(|h| h.code.clone()))
            .unwrap_or_default()
    }

    #[test]
    fn the_line_splits_at_the_cursor_in_characters() {
        let s = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(split_at_char("mean(é)", 6), s("mean(é", ")"));
        assert_eq!(split_at_char("abc", 0), s("", "abc"));
        assert_eq!(split_at_char("abc", 99), s("abc", ""));
        assert_eq!(split_at_char("", 0), s("", ""));
    }

    /// Starts `code` and gives its command, without waiting for it.
    fn submit_now(app: &mut App, code: &str) -> CommandId {
        match &mut app.engine {
            EngineState::Ready(e) => e.submit(Source::Typed, code).unwrap_or_default(),
            _ => 0,
        }
    }

    #[test]
    fn a_missing_package_is_offered_and_the_command_runs_again_after() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        let submit = |app: &mut App, code: &str| {
            app.perform(&ctx, Action::Submit(Source::Typed, code.into()));
            assert!(frames_until(app, &ctx, |a| a.is_settled()), "{code}");
        };
        let last_block = |app: &App| {
            app.engine()
                .and_then(|e| e.results().blocks().last().cloned())
                .map(|b| b.items)
                .unwrap_or_default()
        };
        // library(), pkg::f() and require() all say which package
        for code in ["library(zzpkgq)", "zzpkgq::f()", "require(zzpkgq)"] {
            submit(&mut app, code);
            assert_eq!(
                needed_package(&last_block(&app)),
                Some(("zzpkgq".to_string(), Need::Missing)),
                "{code}: {:?}",
                last_block(&app)
            );
        }
        submit(&mut app, "stop('nothing to do with packages')");
        assert_eq!(needed_package(&last_block(&app)), None);

        // the offer's install code is R that parses
        let code = offer_install_code("zzpkgq", Some("C:/lib"));
        submit(
            &mut app,
            &format!(
                "invisible(parse(text = {}))",
                rcraft_rcodegen::r_string(&code)
            ),
        );
        let outcome = app
            .engine()
            .and_then(|e| e.history().entries().last().and_then(|h| h.outcome));
        assert_eq!(outcome, Some(Outcome::Ok), "{code}");

        // an install that worked: noted, and the command runs again
        let id = submit_now(&mut app, "invisible(1)");
        app.offers.running = Some((
            id,
            "zzpkgq".into(),
            Some((Source::Typed, "ran_again <- TRUE".into())),
        ));
        assert!(frames_until(&mut app, &ctx, |a| a.offers.running.is_none()
            && a.is_settled()));
        assert_eq!(app.offers.done, vec![("zzpkgq".to_string(), id)]);
        let last = app
            .engine()
            .and_then(|e| e.history().entries().last().map(|h| h.code.clone()));
        assert_eq!(last.as_deref(), Some("ran_again <- TRUE"));

        // one that failed: nothing noted, nothing run again
        let id = submit_now(&mut app, "stop('no')");
        app.offers.running = Some((
            id,
            "other".into(),
            Some((Source::Typed, "not_run <- TRUE".into())),
        ));
        assert!(frames_until(&mut app, &ctx, |a| a.offers.running.is_none()
            && a.is_settled()));
        assert_eq!(app.offers.done.len(), 1);
        let last = app
            .engine()
            .and_then(|e| e.history().entries().last().map(|h| h.code.clone()));
        assert_eq!(last.as_deref(), Some("stop('no')"));
    }

    #[test]
    fn tab_in_the_window_completes_at_the_cursor() {
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
                "my_cars2 <- data.frame(mpg = 1, cyl = 2)".into(),
            ),
        );
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        let id = egui::Id::new(COMMAND_ID);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0));
        // types `line`, puts the cursor `at` characters in, presses Tab
        let mut press_tab = |line: &str, at: usize| -> String {
            app.perform(&ctx, Action::SetCommand(line.into()));
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| app.ui(ctx),
            );
            cursor_to(&ctx, id, at);
            let tab = egui::Event::Key {
                key: Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            };
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events: vec![tab],
                    ..Default::default()
                },
                |ctx| app.ui(ctx),
            );
            assert!(frames_until(&mut app, &ctx, |a| a.tab_pending.is_none()
                && a.is_settled()));
            app.command.clone()
        };
        // a name in the middle of the line; what follows stays
        assert_eq!(press_tab("summary(my_ca)", 13), "summary(my_cars2)");
        // a column inside [[" at the end, closed
        assert_eq!(press_tab("my_cars2[[\"cy", 13), "my_cars2[[\"cyl\"]]");
        // an argument name
        assert_eq!(press_tab("mean(1:3, na.r", 14), "mean(1:3, na.rm = ");
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

    /// P2C: a do-file run in the window, line by line in R, a value worked out
    /// by R on the way, and the choice at each problem.
    #[test]
    fn a_do_file_runs_line_by_line_and_asks_at_a_problem() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        let dir = std::env::temp_dir().join(format!("rcraft-dofile-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("d.csv"), "x,y\n1,2\n2,4\n3,7\n");
        let dofile = dir.join("run.do");
        let _ = std::fs::write(
            &dofile,
            "import delimited d.csv, clear\nsummarize y\nlocal m = r(mean)\ngenerate dy = y - `m'\n\
             foreach v of varlist x-y {\n  quietly summarize `v'\n  display \"@`v' \" r(max)\n}\n\
             bogus line\ndisplay \"after\"\nregress y nosuchvar\ndisplay \"end\"\n",
        );
        let stopped = |a: &App| {
            a.dofile
                .outcome()
                .is_some_and(|(done, _, problem)| done || problem)
        };
        app.open_dofile(&dofile, true);
        assert!(frames_until(&mut app, &ctx, stopped));
        let log = app.dofile.log_text();
        if log.contains("there is no package") {
            eprintln!("skipped: readr and dplyr are needed");
            return;
        }
        // the first problem: a line that is not translated
        let (done, ran, problem) = app.dofile.outcome().unwrap_or_default();
        assert!(!done && problem, "{log}");
        assert_eq!(ran.last(), Some(&Ran::Skipped), "{log}");
        assert!(log.contains("m is \"4.333333333333333\""), "{log}");
        assert!(log.contains("@x 3") && log.contains("@y 7"), "{log}");
        app.dofile.go_on(false);
        // the second: R stops with an error; then on without asking
        assert!(frames_until(&mut app, &ctx, stopped));
        let (done, ran, problem) = app.dofile.outcome().unwrap_or_default();
        assert!(!done && problem, "{}", app.dofile.log_text());
        assert!(matches!(ran.last(), Some(Ran::Error(_))), "{ran:?}");
        app.dofile.go_on(true);
        assert!(frames_until(&mut app, &ctx, stopped));
        let (done, ran, _) = app.dofile.outcome().unwrap_or_default();
        assert!(done, "{ran:?}");
        let log = app.dofile.log_text();
        assert!(log.contains(". display \"end\"\nend"), "{log}");
        assert!(
            log.contains("1 stopped with an error in R, 1 were not run"),
            "{log}"
        );
        // the data it left are shown
        assert!(frames_until(&mut app, &ctx, |a| a.dataset.as_deref() == Some("d")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2C part two: a typed Stata line whose value only R knows is worked
    /// out by R first, and the lines after it use it.
    #[test]
    fn a_typed_stata_line_gets_a_value_from_r() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        let dir = std::env::temp_dir().join(format!("rcraft-typed-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("d.csv"), "x,y\n1,2\n2,4\n3,7\n");
        let file = dir.join("d.csv").display().to_string().replace('\\', "/");
        app.perform(&ctx, Action::SetStataMode(true));
        let run = |app: &mut App, line: &str| {
            app.perform(&ctx, Action::SubmitStata(Source::Typed, line.to_string()));
            assert!(frames_until(app, &ctx, |a| a.is_settled()), "{line}");
        };
        run(&mut app, &format!("import delimited \"{file}\", clear"));
        let ok = app
            .engine()
            .and_then(|e| e.history().entries().last().and_then(|h| h.outcome));
        if ok != Some(Outcome::Ok) {
            eprintln!("skipped: readr and dplyr are needed");
            return;
        }
        run(&mut app, "summarize y");
        run(&mut app, "local m = r(mean)");
        run(&mut app, "display `m' * 3");
        assert!(newest_text(&app).contains("13"), "{}", newest_text(&app));
        // a varlist with a range: the data's names from R
        run(&mut app, "foreach v of varlist x-y {\ndisplay \"[`v']\"\n}");
        let text = newest_text(&app);
        assert!(text.contains("[x]") && text.contains("[y]"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2C.11: typed Stata lines draw Stata's own random numbers, in turn; a
    /// value worked out by R out of sight (`local r = runiform()`) takes its
    /// draw once. The numbers are Stata 19's after `set seed 11`.
    #[test]
    fn typed_stata_lines_draw_statas_numbers_in_turn() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings(false);
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        app.perform(&ctx, Action::SetStataMode(true));
        let run = |app: &mut App, line: &str| {
            app.perform(&ctx, Action::SubmitStata(Source::Typed, line.to_string()));
            assert!(frames_until(app, &ctx, |a| a.is_settled()), "{line}");
        };
        run(&mut app, "set seed 11");
        run(&mut app, "display %21x runiform()");
        assert!(
            newest_text(&app).contains("+1.536165793c6e4X-003"),
            "{}",
            newest_text(&app)
        );
        run(&mut app, "local r = runiform()");
        run(&mut app, "display %21x runiform()");
        assert!(
            newest_text(&app).contains("+1.831909afac7caX-002"),
            "{}",
            newest_text(&app)
        );
    }

    /// P7.3: File > Open script and Save script, with real files: an R script
    /// with Windows line endings and a do-file (which turns Stata mode on);
    /// changes not saved are asked about before another script replaces them
    /// (Cancel keeps all, Don't save lets them go, Save writes them first);
    /// Save as gives the script a new file.
    #[test]
    fn scripts_open_and_save_and_changes_are_not_lost_unasked() {
        let dir = std::env::temp_dir().join(format!("rcraft-scripts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(std::fs::create_dir_all(&dir).is_ok());
        let (a, b, c) = (dir.join("a.R"), dir.join("b.do"), dir.join("c.R"));
        assert!(std::fs::write(&a, "x <- 1\r\ny <- 2\r\n").is_ok());
        assert!(std::fs::write(&b, "sysuse auto\nsummarize price\n").is_ok());
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings = false;

        // nothing typed yet: opens at once, Windows endings made plain
        app.perform(&ctx, Action::OpenScriptPath(a.clone()));
        assert_eq!(app.script, "x <- 1\ny <- 2\n");
        assert_eq!(app.script_file.title(&app.script), "a.R");
        assert!(!app.stata.on);

        // changed: opening another asks, and Cancel keeps everything
        app.script.push_str("z <- 3\n");
        assert_eq!(app.script_file.title(&app.script), "a.R •");
        app.perform(&ctx, Action::OpenScriptPath(b.clone()));
        assert_eq!(app.script_ask, Some(ScriptAsk::Open(b.clone())));
        app.perform(&ctx, Action::ScriptAnswer(None));
        assert_eq!(app.script_ask, None);
        assert!(app.script.ends_with("z <- 3\n"));

        // Don't save: the do-file opens in Stata mode, a.R is as it was on disk
        app.perform(&ctx, Action::OpenScriptPath(b.clone()));
        app.perform(&ctx, Action::ScriptAnswer(Some(false)));
        assert_eq!(app.script, "sysuse auto\nsummarize price\n");
        assert!(app.stata.on);
        assert_eq!(
            std::fs::read(&a).unwrap_or_default(),
            b"x <- 1\r\ny <- 2\r\n".to_vec()
        );

        // Save: the do-file's change is written before a.R replaces it
        app.script.push_str("regress price mpg\n");
        app.perform(&ctx, Action::OpenScriptPath(a.clone()));
        app.perform(&ctx, Action::ScriptAnswer(Some(true)));
        assert_eq!(
            std::fs::read_to_string(&b).unwrap_or_default(),
            "sysuse auto\nsummarize price\nregress price mpg\n"
        );
        assert_eq!(app.script, "x <- 1\ny <- 2\n");
        assert!(!app.stata.on);

        // Save as: a new file, which the script then belongs to, keeping a.R's endings out of it
        app.script.push_str("w <- 4\n");
        app.perform(&ctx, Action::SaveScriptTo(c.clone()));
        assert_eq!(
            std::fs::read_to_string(&c).unwrap_or_default(),
            "x <- 1\ny <- 2\nw <- 4\n"
        );
        assert_eq!(app.script_file.title(&app.script), "c.R");
        assert_eq!(app.message, None);

        // a file that cannot be read says why and leaves the script alone
        app.perform(&ctx, Action::OpenScriptPath(dir.join("missing.R")));
        assert!(app
            .message
            .as_deref()
            .is_some_and(|m| m.contains("missing.R")));
        assert!(app.script.ends_with("w <- 4\n"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P7.3: find and replace in the Script Editor, and the outline, through
    /// the window's actions: Next and Previous select each match in turn
    /// (whole words, wrapping), Replace changes the selected match and moves
    /// on, Replace all says how many, and an outline pick puts the cursor at
    /// the start of its line.
    #[test]
    fn find_replace_and_the_outline_move_through_the_script() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings = false;
        app.script = "# Models ----\nfit <- lm(y ~ x)\nfit2 <- 1\nsummary(fit)\n".into();
        app.perform(&ctx, Action::OpenFind(true));
        assert!(app.find.open && app.find.replace);
        app.find.search.needle = "fit".into();
        app.find.search.whole_word = true;

        let at = |s: &str, from: usize| s.chars().skip(from).take(3).collect::<String>();
        app.perform(&ctx, Action::FindStep(true));
        let first = editor_selection(&ctx);
        assert_eq!(first, Some((14, 17)), "{:?}", app.find.note);
        assert_eq!(app.find.note.as_deref(), Some("2 found"));
        app.perform(&ctx, Action::FindStep(true));
        let second = editor_selection(&ctx).unwrap_or_default();
        assert_eq!(at(&app.script, second.0), "fit");
        assert!(second.0 > 30, "summary(fit), not fit2: {second:?}");
        // wraps to the first, and back again
        app.perform(&ctx, Action::FindStep(true));
        assert_eq!(editor_selection(&ctx), Some((14, 17)));
        app.perform(&ctx, Action::FindStep(false));
        assert_eq!(editor_selection(&ctx), Some(second));

        // Replace: the selected match, then the next is selected
        app.find.with = "model".into();
        app.perform(&ctx, Action::ReplaceOne);
        assert!(app.script.ends_with("summary(model)\n"), "{}", app.script);
        assert_eq!(editor_selection(&ctx), Some((14, 17)));
        app.perform(&ctx, Action::ReplaceAll);
        assert_eq!(
            app.script,
            "# Models ----\nmodel <- lm(y ~ x)\nfit2 <- 1\nsummary(model)\n"
        );
        assert_eq!(app.find.note.as_deref(), Some("1 replaced"));
        app.perform(&ctx, Action::FindStep(true));
        assert_eq!(app.find.note.as_deref(), Some("Not found"));

        // the outline lists the section; a pick goes to its line
        let sections = script_tools::outline(&app.script);
        assert_eq!(sections.first().map(|s| s.title.as_str()), Some("Models"));
        app.perform(&ctx, Action::GoToLine(2));
        let line3 = "# Models ----\nmodel <- lm(y ~ x)\n".chars().count();
        assert_eq!(editor_selection(&ctx), Some((line3, line3)));

        // what is selected becomes what to look for
        select_in_editor(&ctx, line3, line3 + 4);
        app.perform(&ctx, Action::OpenFind(false));
        assert_eq!(app.find.search.needle, "fit2");
        app.perform(&ctx, Action::CloseFind);
        assert!(!app.find.open);
    }

    /// P7.4: File > Save session as writes the kind its ending names, and Save
    /// session as Quarto project writes _quarto.yml and the document into a
    /// folder, never over files already there.
    #[test]
    fn the_session_saves_as_a_script_a_document_or_a_quarto_project() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings = false;
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        app.perform(&ctx, Action::Submit(Source::Typed, "kept <- 41 + 1".into()));
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        let dir = std::env::temp_dir().join(format!("rcraft-save-session-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(std::fs::create_dir_all(&dir).is_ok());

        for (name, start) in [
            ("s.R", "# Script from an RCraft session"),
            ("s.qmd", "---\ntitle: \"RCraft session: "),
            ("s.Rmd", "---\ntitle: \"RCraft session: "),
        ] {
            app.perform(&ctx, Action::SaveSessionTo(dir.join(name)));
            assert_eq!(app.message, None, "{name}");
            let text = std::fs::read_to_string(dir.join(name)).unwrap_or_default();
            assert!(
                text.starts_with(start) && text.contains("kept <- 41 + 1"),
                "{name}: {text}"
            );
        }
        let rmd = std::fs::read_to_string(dir.join("s.Rmd")).unwrap_or_default();
        assert!(rmd.contains("output: html_document"), "{rmd}");

        let project = dir.join("project");
        app.perform(&ctx, Action::SaveSessionProject(project.clone()));
        assert_eq!(app.message, None);
        assert_eq!(
            std::fs::read_to_string(project.join("_quarto.yml")).unwrap_or_default(),
            "project:\n  type: default\n"
        );
        let docs: Vec<std::path::PathBuf> = std::fs::read_dir(&project)
            .map(|r| r.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        assert!(
            docs.iter()
                .any(|p| p.extension().is_some_and(|e| e == "qmd")),
            "{docs:?}"
        );
        // again into the same folder: nothing is written over
        app.perform(&ctx, Action::SaveSessionProject(project.clone()));
        assert!(
            app.message
                .as_deref()
                .is_some_and(|m| m.contains("already there")),
            "{:?}",
            app.message
        );
        assert_eq!(file_stem_of("Session 2: test/run"), "Session_2__test_run");
        assert_eq!(file_stem_of("  ::  "), "session");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P7.2: the Reports menu's dialogs open with no data frame shown.
    #[test]
    fn the_reports_dialogs_need_no_data_frame() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings = false;
        assert_eq!(app.dataset, None);
        for spec in rcraft_catalog::report::REPORT_SPECS {
            assert!(
                app.open_dialog(&ctx, spec.entry, &[]).is_ok(),
                "{}",
                spec.entry
            );
            assert_eq!(
                app.spec_dialog.dataset(),
                Some("(report)"),
                "{}",
                spec.entry
            );
        }
    }

    /// P7.5: a project saved from one session and opened in a fresh one: the
    /// same objects (identical in R), the script, R's working folder, the log,
    /// and a package list the Packages window reads, naming the packages the
    /// session had loaded with their versions.
    #[test]
    fn a_project_saved_and_opened_again_brings_the_work_back() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings = false;
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        let dir = std::env::temp_dir().join(format!("rcraft-project-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let folder = dir.join("wages");
        let want = dir
            .join("want.rds")
            .display()
            .to_string()
            .replace('\\', "/");
        assert!(std::fs::create_dir_all(&dir).is_ok());
        app.perform(
            &ctx,
            Action::Submit(
                Source::Typed,
                format!(
                    "set.seed(3); d <- data.frame(w = rnorm(25), e = rep(1:5, 5)); \
                     fit <- lm(w ~ e, data = d); .hidden <- 'kept too'; \
                     saveRDS(list(d = d, b = coef(fit), h = .hidden), '{want}')"
                ),
            ),
        );
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        app.script = "fit <- lm(w ~ e, data = d)\nsummary(fit)\n".into();
        app.perform(&ctx, Action::SaveProjectTo(folder.clone()));
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        assert_eq!(app.message, None);
        for name in [
            "wages.rcraft",
            "script.R",
            "workspace.RData",
            "session.log",
            "rcraft-packages.txt",
        ] {
            assert!(folder.join(name).is_file(), "{name}");
        }
        assert_eq!(app.script_file.title(&app.script), "script.R");
        assert!(std::fs::read_to_string(folder.join("session.log"))
            .unwrap_or_default()
            .contains("fit <- lm(w ~ e"));
        let list = std::fs::read_to_string(folder.join("rcraft-packages.txt")).unwrap_or_default();
        let lock =
            rcraft_rpkg::lockfile::Lock::parse(&list).unwrap_or_else(|e| panic!("{e}\n{list}"));
        assert!(lock.packages.iter().any(|p| p.name == "stats"), "{list}");

        // a fresh session: the work comes back
        let mut fresh = App::new(&ctx);
        fresh.keep_settings = false;
        assert!(frames_until(&mut fresh, &ctx, |a| a.is_settled()));
        fresh.perform(&ctx, Action::OpenProjectPath(folder.join("wages.rcraft")));
        assert!(frames_until(&mut fresh, &ctx, |a| a.is_settled()));
        assert_eq!(fresh.script, "fit <- lm(w ~ e, data = d)\nsummary(fit)\n");
        let wd = folder.display().to_string().replace('\\', "/");
        fresh.perform(
            &ctx,
            Action::Submit(
                Source::Typed,
                format!(
                    "stopifnot(identical(readRDS('{want}'), list(d = d, b = coef(fit), h = .hidden)), \
                     normalizePath(getwd(), '/') == normalizePath('{wd}', '/'))"
                ),
            ),
        );
        assert!(frames_until(&mut fresh, &ctx, |a| a.is_settled()));
        let last = fresh
            .engine()
            .and_then(|e| e.history().entries().last().and_then(|h| h.outcome));
        assert_eq!(last, Some(Outcome::Ok), "{}", newest_text(&fresh));

        // a project file that names a path outside its folder is refused
        let bad = dir.join("bad.rcraft");
        let text = std::fs::read_to_string(folder.join("wages.rcraft"))
            .unwrap_or_default()
            .replace("script\tscript.R", "script\t../escape.R");
        assert!(std::fs::write(&bad, text).is_ok());
        fresh.perform(&ctx, Action::OpenProjectPath(bad));
        assert!(
            fresh
                .message
                .as_deref()
                .is_some_and(|m| m.contains("not a file name")),
            "{:?}",
            fresh.message
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P8.1: Properties for a real data frame and a picked variable agree with
    /// what R says of them; another data frame shows its own again.
    #[test]
    fn properties_agree_with_r() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.keep_settings = false;
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        if !app.is_ready() {
            eprintln!("skipped: R did not start");
            return;
        }
        app.perform(
            &ctx,
            Action::Submit(
                Source::Typed,
                "d <- data.frame(x = c(2.5, NA, 4, 4, 10), g = factor(c('a', 'b', 'a', NA, 'c'))); \
                 attr(d$x, 'label') <- 'A measure'"
                    .into(),
            ),
        );
        assert!(frames_until(&mut app, &ctx, |a| {
            a.dataset.as_deref() == Some("d") && a.engine().is_some_and(|e| e.table("d").is_some())
        }));
        let Some(table) = app.engine().and_then(|e| e.table("d")).cloned() else {
            panic!("no table d");
        };
        let whole = crate::properties::of_table("d", &table);
        let get = |p: &[(String, String)], k: &str| {
            p.iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        assert_eq!(get(&whole, "Rows"), "5");
        assert_eq!(get(&whole, "Missing cells"), "2 of 10 (20%)");
        app.perform(&ctx, Action::PickVariable(Some("x".into())));
        assert_eq!(app.picked_variable.as_deref(), Some("x"));
        let col = table.columns.iter().find(|c| c.name == "x").cloned();
        let p = col
            .as_ref()
            .map(crate::properties::of_column)
            .unwrap_or_default();
        app.perform(
            &ctx,
            Action::Submit(
                Source::Typed,
                "cat(mean(d$x, na.rm = TRUE), length(unique(na.omit(d$x))), sum(is.na(d$x)), \
                 min(d$x, na.rm = TRUE), max(d$x, na.rm = TRUE), sep = '|')"
                    .into(),
            ),
        );
        assert!(frames_until(&mut app, &ctx, |a| a.is_settled()));
        let r = newest_text(&app);
        let r: Vec<&str> = r.trim().split('|').collect();
        assert_eq!(
            vec![
                get(&p, "Mean"),
                get(&p, "Distinct"),
                get(&p, "Missing")
                    .split(' ')
                    .next()
                    .unwrap_or("")
                    .to_string(),
                get(&p, "Smallest"),
                get(&p, "Largest"),
            ],
            r,
            "{p:?}"
        );
        assert_eq!(get(&p, "Label"), "A measure");
        let g = table
            .columns
            .iter()
            .find(|c| c.name == "g")
            .map(crate::properties::of_column);
        assert_eq!(
            g.as_ref().map(|p| get(p, "Levels")),
            Some("3: a, b, c".to_string())
        );
        // another data frame: its own properties
        app.perform(&ctx, Action::SelectDataset(Some("d".into())));
        assert_eq!(app.picked_variable, None);
    }
}
