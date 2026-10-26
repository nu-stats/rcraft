//! The window: panes, menus and keys.

use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

use eframe::egui::{self, Align, Key, Layout, Modifiers, RichText, TextStyle};
use rcraft_engine::{Engine, History, Item, Outcome, Recall, Source};
use rcraft_rhost::{find_installs, Env, InterruptOutcome, SessionConfig, Stream, MIN_VERSION};

use crate::look::{self, Accent, Palette};

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
}

impl App {
    pub fn new(ctx: &egui::Context) -> App {
        look::apply_static(ctx);
        let accent = Accent::default();
        look::apply_colors(ctx, accent);
        let theme = egui::ThemePreference::System;
        ctx.set_theme(theme);
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
        }
    }

    fn engine(&self) -> Option<&Engine> {
        match &self.engine {
            EngineState::Ready(e) => Some(&**e),
            _ => None,
        }
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
            if engine.is_busy() {
                ctx.request_repaint_after(Duration::from_millis(33));
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
                ui.add_enabled(false, egui::Button::new("Open file…"));
                ui.add_enabled(false, egui::Button::new("Open script…"));
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
            for title in ["Data", "Statistics", "Graphs", "Packages"] {
                ui.menu_button(title, |ui| {
                    ui.add_enabled(false, egui::Button::new("Coming in a later phase"));
                });
            }
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
            ui.label(RichText::new("No data in memory").color(pal.dim));
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
        pane_title(ui, pal, "Variables");
        egui::ScrollArea::vertical()
            .id_salt("variables")
            .max_height(total * 0.42)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.label(RichText::new("Open a data file to see its variables.").color(pal.dim));
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

    fn results_pane(&self, ui: &mut egui::Ui, pal: &Palette, _out: &mut Vec<Action>) {
        pane_title(ui, pal, "Results");
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
            }
        }
        ui.add_space(6.0);
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
            EngineState::Ready(e) => !e.is_busy(),
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
}
