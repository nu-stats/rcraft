//! Run my do-file (Phase 2C): File > Run do-file…, or a `.do` file dropped on
//! the window. The do-file is read, never changed, and gone through first
//! without R (the scan: what translates, what does not and why, the files it
//! names, what a run on R cannot do as Stata does). Run sends it to R a line
//! at a time, each Stata line beside its R, works out with R the values only
//! R knows (`local m = r(mean)`), and at the first problem asks whether to
//! stop or go on. Afterwards it can be saved as an R script, a Quarto
//! document or the run's log.
//!
//! The window only drives the translator (`statatr::dofile`) and the engine;
//! what each line means is decided there.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, RichText};
use rcraft_engine::{CommandId, Engine, Item, Outcome, QueryId, Source};
use rcraft_statatr::dofile::{
    self, dry_run, status, DoFile, Event, FileRole, FileStatus, Files, Options, Ran, Run, Scan,
    Status,
};

use crate::look::Palette;

/// Do-files read from disk (`do`, `run`, `include`).
struct Disk;

impl Files for Disk {
    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
    fn exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }
}

static DISK: Disk = Disk;

/// Most lines with nothing for R (macros, loops) gone through in one frame.
const LINES_PER_FRAME: usize = 400;
/// How long a frame may spend moving a run on.
const FRAME_BUDGET: Duration = Duration::from_millis(12);

/// What the window asks of the app.
pub enum DoFileOut {
    /// Open the differences guide on this Stata line.
    Guide(String),
    /// The run left this data frame in memory: show it.
    Follow(String),
}

/// What the run waits for.
enum Waiting {
    Nothing,
    /// `setwd()` to the folder the do-file runs in.
    Setup(CommandId),
    /// A line's R, with the helpers sent along with it: if R stops there
    /// (a line R cannot read runs nothing), they are sent again.
    Line {
        index: usize,
        id: CommandId,
        helpers: Vec<&'static str>,
    },
    Value(QueryId, Vec<&'static str>),
}

/// A problem the user is asked about: the line, and what went wrong.
struct Problem {
    index: usize,
    what: String,
}

struct Running {
    /// The session it runs in (a run waits while another one is shown).
    session: String,
    run: Option<Run<'static>>,
    /// The run, once it is done.
    done: Option<DoFile>,
    sent: BTreeSet<&'static str>,
    ran: Vec<Ran>,
    printed: Vec<String>,
    waiting: Waiting,
    problem: Option<Problem>,
    /// Go on past every problem without asking.
    go_on: bool,
    stopped: bool,
}

impl Running {
    fn lines(&self) -> (&[rcraft_statatr::data_r::Line], &[dofile::Origin]) {
        match (&self.done, &self.run) {
            (Some(d), _) => (&d.translation.lines, &d.origins),
            (None, Some(r)) => (r.lines(), r.origins()),
            _ => (&[], &[]),
        }
    }
}

#[derive(Default)]
pub struct DoFileWindow {
    open: bool,
    path: PathBuf,
    name: String,
    text: String,
    /// The folder it runs in (its own unless changed), with `/`.
    folder: String,
    /// Folders it names that are elsewhere here: from, to.
    map: Vec<(String, String)>,
    dry: DoFile,
    scan: Scan,
    running: Option<Running>,
    selected: Option<usize>,
    message: Option<String>,
}

fn slashes(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

impl DoFileWindow {
    /// Whether a run is going on (or waits for an answer).
    pub fn is_running(&self) -> bool {
        self.running
            .as_ref()
            .is_some_and(|r| r.done.is_none() && !r.stopped)
    }

    /// Reads a do-file and scans it.
    pub fn open_file(&mut self, path: &Path) {
        match std::fs::read(path) {
            Ok(bytes) => {
                self.text = String::from_utf8_lossy(&bytes).into_owned();
                self.path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
                self.name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.folder = self.path.parent().map(slashes).unwrap_or_default();
                self.map.clear();
                self.running = None;
                self.selected = None;
                self.message = None;
                self.rescan();
                self.open = true;
            }
            Err(e) => {
                self.message = Some(format!("{}: {e}", path.display()));
                self.open = true;
            }
        }
    }

    fn options(&self) -> Options {
        Options {
            name: self.name.clone(),
            folder: self.folder.clone(),
            map: self.map.clone(),
            temp_folder: slashes(&std::env::temp_dir().join("RCraft-Stata")),
        }
    }

    fn rescan(&mut self) {
        self.dry = dry_run(&self.text, &DISK, self.options());
        self.scan = dofile::scan(&self.dry);
    }

    /// The scan, in words (the headless picture and tests read it).
    pub fn scan_text(&self) -> String {
        self.scan.text(&self.name)
    }

    fn start(&mut self, session: &str) {
        self.rescan();
        self.running = Some(Running {
            session: session.to_string(),
            run: Some(Run::new(&self.text, &DISK, self.options())),
            done: None,
            sent: BTreeSet::new(),
            ran: Vec::new(),
            printed: Vec::new(),
            waiting: Waiting::Nothing,
            problem: None,
            go_on: false,
            stopped: false,
        });
        self.selected = None;
    }

    /// Moves the run on as far as R lets it for a few milliseconds; called
    /// every frame. Each line R runs is a short trip, so waiting a frame for
    /// each made a long do-file take minutes where R needed seconds.
    pub fn poll(&mut self, engine: &mut Engine, session: &str) -> Option<DoFileOut> {
        let until = Instant::now() + FRAME_BUDGET;
        loop {
            let out = self.step(engine, session);
            let going = self.running.as_ref().is_some_and(|r| {
                r.session == session && r.done.is_none() && !r.stopped && r.problem.is_none()
            });
            let now = Instant::now();
            if out.is_some() || !going || now >= until {
                return out;
            }
            engine.poll((until - now).min(Duration::from_millis(4)));
        }
    }

    /// One move of the run: what R finished, then the next lines.
    fn step(&mut self, engine: &mut Engine, session: &str) -> Option<DoFileOut> {
        let folder = self.folder.clone();
        let r = self.running.as_mut()?;
        if r.session != session || r.done.is_some() || r.stopped || r.problem.is_some() {
            return None;
        }
        // what it waits for
        match std::mem::replace(&mut r.waiting, Waiting::Nothing) {
            Waiting::Nothing => {}
            Waiting::Setup(id) => match outcome(engine, id) {
                None => {
                    r.waiting = Waiting::Setup(id);
                    return None;
                }
                Some(Outcome::Ok) => r.waiting = Waiting::Nothing,
                Some(_) => {
                    r.stopped = true;
                    let (_, error) = printed(engine, id);
                    self.message = Some(format!("R could not go to {folder}: {error}"));
                    return None;
                }
            },
            Waiting::Line { index, id, helpers } => {
                let Some(o) = outcome(engine, id) else {
                    r.waiting = Waiting::Line { index, id, helpers };
                    return None;
                };
                if o != Outcome::Ok {
                    for h in &helpers {
                        r.sent.remove(h);
                    }
                }
                let (text, error) = printed(engine, id);
                if let Some(p) = r.printed.get_mut(index) {
                    *p = text;
                }
                r.waiting = Waiting::Nothing;
                if o != Outcome::Ok {
                    let why = if error.is_empty() {
                        format!("{o:?}")
                    } else {
                        error
                    };
                    if let Some(x) = r.ran.get_mut(index) {
                        *x = Ran::Error(why.clone());
                    }
                    if !r.go_on {
                        r.problem = Some(Problem {
                            index,
                            what: format!("R stopped with an error: {why}"),
                        });
                        return None;
                    }
                }
            }
            Waiting::Value(q, helpers) => {
                let Some(answer) = engine.query_result(q) else {
                    r.waiting = Waiting::Value(q, helpers);
                    return None;
                };
                if !answer.ok {
                    for h in &helpers {
                        r.sent.remove(h);
                    }
                }
                let given = if answer.ok {
                    Ok(answer.text.clone())
                } else {
                    Err(format!(
                        "R could not work the value out: {}",
                        answer.text.trim()
                    ))
                };
                if let Some(run) = r.run.as_mut() {
                    run.give(given);
                }
                r.waiting = Waiting::Nothing;
            }
        }
        if engine.is_working() {
            return None;
        }
        if r.ran.is_empty() && matches!(r.waiting, Waiting::Nothing) && r.printed.is_empty() {
            // first, R goes to the folder the do-file runs in (once)
            if !folder.is_empty() && r.sent.insert("setwd") {
                let code = format!("setwd({})", r_string(&folder));
                match engine.submit(Source::Script, &code) {
                    Ok(id) => r.waiting = Waiting::Setup(id),
                    Err(e) => {
                        r.stopped = true;
                        self.message = Some(e.to_string());
                    }
                }
                return None;
            }
        }
        for _ in 0..LINES_PER_FRAME {
            let run = r.run.as_mut()?;
            // each line made so far, in order, before the run goes on
            let index = r.ran.len();
            if let Some(line) = run.lines().get(index).cloned() {
                let (code, helpers) = helpers_code(run, &r.sent);
                r.printed.push(String::new());
                match &line.result {
                    Err(e) => {
                        r.ran.push(Ran::Skipped);
                        if !r.go_on {
                            r.problem = Some(Problem {
                                index,
                                what: format!("not translated: {}", e.reason),
                            });
                            return None;
                        }
                    }
                    Ok(rc) if rc.code.is_empty() && helpers.is_empty() => r.ran.push(Ran::Ok),
                    Ok(_) => {
                        let code = code + &line.script();
                        r.ran.push(Ran::Ok);
                        r.sent.extend(helpers.iter().copied());
                        match engine.submit(Source::Script, &code) {
                            Ok(id) => r.waiting = Waiting::Line { index, id, helpers },
                            Err(e) => {
                                r.stopped = true;
                                self.message = Some(e.to_string());
                            }
                        }
                        return None;
                    }
                }
                continue;
            }
            match run.next_event() {
                Event::Line => {}
                Event::Ask(v) => {
                    let (code, helpers) = helpers_code(run, &r.sent);
                    let code = code + &v.code;
                    r.sent.extend(helpers.iter().copied());
                    r.waiting = Waiting::Value(engine.query(&code), helpers);
                    return None;
                }
                Event::Done => {
                    let done = r.run.take().map(Run::finish).unwrap_or_default();
                    let data = done.translation.data.clone();
                    r.done = Some(done);
                    return data.map(DoFileOut::Follow);
                }
            }
        }
        None
    }

    /// The user's answer at a problem.
    fn answer(&mut self, choice: Choice) {
        let Some(r) = self.running.as_mut() else {
            return;
        };
        r.problem = None;
        match choice {
            Choice::Stop => r.stopped = true,
            Choice::GoOn => {}
            Choice::GoOnAlways => r.go_on = true,
        }
    }

    fn ran_and_printed(&self) -> (Vec<Ran>, Vec<String>) {
        match &self.running {
            Some(r) => (r.ran.clone(), r.printed.clone()),
            None => (Vec::new(), Vec::new()),
        }
    }

    /// What the files written afterwards are made from: the run if there was
    /// one (with the values R worked out), else the scan.
    fn document(&self) -> DoFile {
        match &self.running {
            Some(Running { done: Some(d), .. }) => d.clone(),
            Some(Running { run: Some(r), .. }) => r.snapshot(),
            _ => self.dry.clone(),
        }
    }

    fn stem(&self) -> String {
        Path::new(&self.name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "dofile".into())
    }

    fn save(&mut self, what: Save) {
        let (ext, title) = match what {
            Save::R => ("R", "Save as R script"),
            Save::Quarto => ("qmd", "Save as Quarto"),
            Save::Log => ("log", "Save the run's log"),
        };
        let suggested = match what {
            Save::Log => format!("{}-rcraft.log", self.stem()),
            _ => format!("{}.{ext}", self.stem()),
        };
        let mut chooser = rfd::FileDialog::new()
            .set_title(title)
            .set_file_name(&suggested)
            .add_filter(ext, &[ext]);
        if let Some(dir) = self.path.parent() {
            chooser = chooser.set_directory(dir);
        }
        let Some(to) = chooser.save_file() else {
            return;
        };
        let text = self.saved_text(what);
        self.message = Some(match std::fs::write(&to, text) {
            Ok(()) => format!("Saved {}.", to.display()),
            Err(e) => format!("{}: {e}", to.display()),
        });
    }

    fn saved_text(&self, what: Save) -> String {
        let d = self.document();
        match what {
            Save::R => dofile::r_script(&d, &self.name, &self.folder),
            Save::Quarto => dofile::quarto(&d, &self.name),
            Save::Log => {
                let (ran, printed) = self.ran_and_printed();
                dofile::report(&d, &self.name, &ran, &printed)
            }
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, pal: &Palette, session: &str) -> Option<DoFileOut> {
        if !self.open {
            return None;
        }
        let mut open = true;
        let mut out = None;
        let title = if self.name.is_empty() {
            "Run do-file".to_string()
        } else {
            format!("Run do-file: {}", self.name)
        };
        egui::Window::new(title)
            .id(egui::Id::new("rcraft-dofile-window"))
            .open(&mut open)
            .default_size([960.0, 600.0])
            .max_width(ctx.screen_rect().width() - 24.0)
            .resizable(true)
            .show(ctx, |ui| {
                out = self.body(ui, pal, session);
            });
        if !open {
            self.open = false;
        }
        out
    }

    fn body(&mut self, ui: &mut egui::Ui, pal: &Palette, session: &str) -> Option<DoFileOut> {
        let mut out = None;
        if self.name.is_empty() {
            if let Some(m) = &self.message {
                ui.colored_label(pal.error, m);
            }
            return None;
        }
        ui.label(RichText::new(self.path.display().to_string()).color(pal.dim));
        let running = self.is_running();
        ui.horizontal(|ui| {
            ui.label("Runs in:");
            ui.monospace(&self.folder);
            if ui
                .add_enabled(!running, egui::Button::new("Change…"))
                .clicked()
            {
                if let Some(dir) = rfd::FileDialog::new()
                    .set_title("The folder the do-file runs in")
                    .set_directory(&self.folder)
                    .pick_folder()
                {
                    self.folder = slashes(&dir);
                    self.rescan();
                }
            }
        });
        self.missing_files(ui, pal, running);
        egui::CollapsingHeader::new("What it holds")
            .default_open(self.running.is_none())
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("dofile-scan")
                    .max_height(160.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(self.scan_text()).monospace().size(12.0));
                    });
            });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if running {
                if ui.button("Stop").clicked() {
                    self.answer(Choice::Stop);
                }
            } else if ui.button("Run on R").clicked() {
                self.start(session);
            }
            if ui.button("Save as R script…").clicked() {
                self.save(Save::R);
            }
            if ui.button("Save as Quarto…").clicked() {
                self.save(Save::Quarto);
            }
            let ran = self.running.is_some();
            if ui
                .add_enabled(ran, egui::Button::new("Save log…"))
                .clicked()
            {
                self.save(Save::Log);
            }
            if ui.button("Copy summary").clicked() {
                ui.ctx().copy_text(self.scan_text());
            }
        });
        if let Some(m) = &self.message {
            ui.label(RichText::new(m).color(pal.dim));
        }
        self.progress(ui, pal);
        if let Some(choice) = self.problem_choice(ui, pal) {
            self.answer(choice);
        }
        ui.separator();
        if let Some(g) = self.lines_list(ui, pal) {
            out = Some(g);
        }
        out
    }

    fn missing_files(&mut self, ui: &mut egui::Ui, pal: &Palette, running: bool) {
        let missing: Vec<dofile::FileRef> = self
            .scan
            .files
            .iter()
            .filter(|f| f.status == FileStatus::Missing && f.role != FileRole::Write)
            .cloned()
            .collect();
        if missing.is_empty() {
            return;
        }
        ui.label(RichText::new("Not found on this machine:").color(pal.warning));
        let mut mapped: Option<(String, String)> = None;
        for f in missing.iter().take(12) {
            ui.horizontal(|ui| {
                ui.monospace(&f.path);
                let absolute = is_absolute(&f.written);
                if absolute
                    && ui
                        .add_enabled(!running, egui::Button::new("Find its folder…"))
                        .on_hover_text("Choose where that folder is on this machine; the do-file is not changed")
                        .clicked()
                {
                    let from = parent(&f.path);
                    if let Some(dir) = rfd::FileDialog::new()
                        .set_title(format!("Where is {from}?"))
                        .pick_folder()
                    {
                        mapped = Some((from, slashes(&dir)));
                    }
                } else if !absolute {
                    ui.label(
                        RichText::new("(looked for in the folder the do-file is in at that line)")
                            .color(pal.dim),
                    );
                }
            });
        }
        if missing.len() > 12 {
            ui.label(RichText::new(format!("and {} more", missing.len() - 12)).color(pal.dim));
        }
        if let Some(m) = mapped {
            self.map.push(m);
            self.rescan();
        }
    }

    fn progress(&self, ui: &mut egui::Ui, pal: &Palette) {
        let Some(r) = &self.running else {
            return;
        };
        let (lines, origins) = r.lines();
        let errors = r.ran.iter().filter(|x| matches!(x, Ran::Error(_))).count();
        let skipped = r.ran.iter().filter(|x| matches!(x, Ran::Skipped)).count();
        let notes = lines
            .iter()
            .zip(origins)
            .zip(&r.ran)
            .filter(|((l, o), x)| **x == Ran::Ok && status(l, o) == Status::Notes)
            .count();
        let ok = r.ran.iter().filter(|x| **x == Ran::Ok).count();
        let state = if r.done.is_some() {
            "Done"
        } else if r.stopped {
            "Stopped"
        } else if r.problem.is_some() {
            "Waiting for you"
        } else {
            "Running"
        };
        ui.label(
            RichText::new(format!(
                "{state}: {} lines{}; {ok} ran ({notes} with a note), {errors} stopped with an error in R, {skipped} not run.",
                lines.len(),
                if r.done.is_some() { "" } else { " so far" }
            ))
            .color(if errors > 0 { pal.warning } else { pal.text }),
        );
    }

    fn problem_choice(&self, ui: &mut egui::Ui, pal: &Palette) -> Option<Choice> {
        let r = self.running.as_ref()?;
        let p = r.problem.as_ref()?;
        let (lines, _) = r.lines();
        let stata = lines
            .get(p.index)
            .map(|l| l.stata.clone())
            .unwrap_or_default();
        let mut choice = None;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new(format!(". {stata}")).monospace());
            ui.colored_label(pal.warning, &p.what);
            ui.label(
                RichText::new(
                    "Stata would have run this line, so what comes after it may differ from Stata's results.",
                )
                .color(pal.dim),
            );
            ui.horizontal(|ui| {
                if ui.button("Stop here").clicked() {
                    choice = Some(Choice::Stop);
                }
                if ui.button("Go on").clicked() {
                    choice = Some(Choice::GoOn);
                }
                if ui.button("Go on without asking").clicked() {
                    choice = Some(Choice::GoOnAlways);
                }
            });
        });
        choice
    }

    fn lines_list(&mut self, ui: &mut egui::Ui, pal: &Palette) -> Option<DoFileOut> {
        let mut out = None;
        let (lines, origins, ran): (Vec<_>, Vec<_>, Vec<Option<Ran>>) = match &self.running {
            Some(r) => {
                let (l, o) = r.lines();
                (
                    l.to_vec(),
                    o.to_vec(),
                    (0..l.len()).map(|i| r.ran.get(i).cloned()).collect(),
                )
            }
            None => (
                self.dry.translation.lines.clone(),
                self.dry.origins.clone(),
                vec![None; self.dry.translation.lines.len()],
            ),
        };
        let row_h = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
        let detail_h = if self.selected.is_some() { 150.0 } else { 0.0 };
        let list_h = (ui.available_height() - detail_h - 8.0).max(row_h * 4.0);
        egui::ScrollArea::vertical()
            .id_salt("dofile-lines")
            .max_height(list_h)
            .auto_shrink([false, false])
            .show_rows(ui, row_h, lines.len(), |ui, rows| {
                for i in rows {
                    let (Some(line), Some(origin)) = (lines.get(i), origins.get(i)) else {
                        continue;
                    };
                    let (glyph, colour) = match ran.get(i).cloned().flatten() {
                        Some(Ran::Ok) => match status(line, origin) {
                            Status::Notes => ("note ", pal.warning),
                            _ => ("ran  ", pal.accent),
                        },
                        Some(Ran::Error(_)) => ("error", pal.error),
                        Some(Ran::Skipped) => ("skip ", pal.warning),
                        Some(Ran::NotReached) => ("     ", pal.dim),
                        None => match status(line, origin) {
                            Status::Clean | Status::ByRcraft => ("     ", pal.dim),
                            Status::Notes => ("note ", pal.warning),
                            Status::Not => ("not  ", pal.error),
                            Status::Later => ("later", pal.dim),
                        },
                    };
                    let right = match &line.result {
                        Ok(r) if !r.code.is_empty() => {
                            r.code.lines().next().unwrap_or("").to_string()
                        }
                        Ok(_) => origin.info.clone().unwrap_or_default(),
                        Err(e) => e.reason.clone(),
                    };
                    let selected = self.selected == Some(i);
                    let row = ui.horizontal(|ui| {
                        ui.label(RichText::new(glyph).monospace().color(colour));
                        ui.label(
                            RichText::new(if origin.file == 0 {
                                format!("{:>5}", origin.span.line)
                            } else {
                                format!("{:>4}+", origin.span.line)
                            })
                            .monospace()
                            .color(pal.dim),
                        );
                        let stata: String = line.stata.chars().take(52).collect();
                        let l = ui.add(
                            egui::Label::new(RichText::new(format!("{stata:<52}")).monospace())
                                .selectable(false)
                                .sense(egui::Sense::click()),
                        );
                        let right: String = right.chars().take(90).collect();
                        let r = ui.add(
                            egui::Label::new(RichText::new(right).monospace().color(pal.dim))
                                .truncate()
                                .selectable(false)
                                .sense(egui::Sense::click()),
                        );
                        l.clicked() || r.clicked()
                    });
                    if selected {
                        ui.painter().rect_stroke(
                            row.response.rect,
                            2.0,
                            egui::Stroke::new(1.0_f32, pal.accent),
                        );
                    }
                    if row.inner {
                        self.selected = if selected { None } else { Some(i) };
                    }
                }
            });
        if let Some(i) = self.selected {
            if let (Some(line), Some(origin)) = (lines.get(i), origins.get(i)) {
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("dofile-detail")
                    .max_height(detail_h)
                    .show(ui, |ui| {
                        if origin.written.trim() != line.stata.trim() && !origin.written.is_empty()
                        {
                            ui.label(
                                RichText::new(format!("as written: {}", origin.written))
                                    .monospace()
                                    .color(pal.dim),
                            );
                        }
                        if !origin.loops.is_empty() {
                            ui.label(
                                RichText::new(format!("in: {}", origin.loops.join("; ")))
                                    .color(pal.dim),
                            );
                        }
                        let printed = self
                            .running
                            .as_ref()
                            .and_then(|r| r.printed.get(i).cloned())
                            .unwrap_or_default();
                        match &line.result {
                            Ok(r) => {
                                if !r.code.is_empty() {
                                    ui.label(RichText::new(&r.code).monospace().size(12.0));
                                }
                                for c in &r.caveats {
                                    ui.label(
                                        RichText::new(format!("note: {c}")).color(pal.warning),
                                    );
                                }
                            }
                            Err(e) => {
                                ui.colored_label(
                                    pal.error,
                                    format!("not translated: {}", e.reason),
                                );
                            }
                        }
                        if let Some(Some(Ran::Error(e))) = ran.get(i) {
                            ui.colored_label(pal.error, format!("R stopped with an error: {e}"));
                        }
                        if !printed.trim().is_empty() {
                            ui.label(RichText::new(printed.trim_end()).monospace().size(12.0));
                        }
                        if rcraft_statatr::differences::for_line(&line.stata).is_some()
                            && ui
                                .link("Where Stata and R differ for this command")
                                .clicked()
                        {
                            out = Some(DoFileOut::Guide(line.stata.clone()));
                        }
                    });
            }
        }
        out
    }

    /// The run's log so far (tests).
    #[cfg(test)]
    pub fn log_text(&self) -> String {
        self.saved_text(Save::Log)
    }

    /// The run's state, for tests: whether it is done and how each line went.
    #[cfg(test)]
    pub fn outcome(&self) -> Option<(bool, Vec<Ran>, bool)> {
        self.running
            .as_ref()
            .map(|r| (r.done.is_some(), r.ran.clone(), r.problem.is_some()))
    }

    /// Answers the question at a problem (tests).
    #[cfg(test)]
    pub fn go_on(&mut self, always: bool) {
        self.answer(if always {
            Choice::GoOnAlways
        } else {
            Choice::GoOn
        });
    }

    /// Starts the run, as the Run button does (tests and pictures).
    pub fn run_now(&mut self, session: &str) {
        if !self.name.is_empty() && !self.is_running() {
            self.start(session);
        }
    }
}

#[derive(Clone, Copy)]
enum Choice {
    Stop,
    GoOn,
    GoOnAlways,
}

#[derive(Clone, Copy)]
enum Save {
    R,
    Quarto,
    Log,
}

/// The helpers the run calls that R has not been given yet, between the
/// lines Results folds; and their names, which count as sent once the R
/// they go with has run.
fn helpers_code(run: &Run<'_>, sent: &BTreeSet<&'static str>) -> (String, Vec<&'static str>) {
    let new: Vec<(&'static str, &'static str)> = run
        .helpers()
        .into_iter()
        .filter(|(n, _)| !sent.contains(n))
        .collect();
    if new.is_empty() {
        return (String::new(), Vec::new());
    }
    let names: Vec<&str> = new.iter().map(|(n, _)| *n).collect();
    let mut code = format!(
        "{} {}\n",
        rcraft_statatr::live::HELPERS_BEGIN,
        names.join(", ")
    );
    for (_, h) in &new {
        code.push_str(h);
        code.push('\n');
    }
    code.push_str(rcraft_statatr::live::HELPERS_END);
    code.push('\n');
    (code, names)
}

fn outcome(engine: &Engine, id: CommandId) -> Option<Outcome> {
    engine.history().get(id).and_then(|h| h.outcome)
}

/// What a command printed, and its error if it stopped with one.
fn printed(engine: &Engine, id: CommandId) -> (String, String) {
    let mut text = String::new();
    let mut error = String::new();
    if let Some(b) = engine.results().blocks().iter().find(|b| b.command == id) {
        for item in &b.items {
            match item {
                Item::Text { text: t, .. } => text.push_str(t),
                Item::Warning(w) => text.push_str(&format!("Warning: {w}\n")),
                Item::Error { message, .. } => error.push_str(message),
                _ => {}
            }
        }
    }
    (text, error.trim().to_string())
}

fn r_string(s: &str) -> String {
    rcraft_statatr::expr_r::r_text(s)
}

fn is_absolute(path: &str) -> bool {
    let b = path.as_bytes();
    path.starts_with('/')
        || path.starts_with('\\')
        || (b.len() >= 2
            && b.first().is_some_and(u8::is_ascii_alphabetic)
            && b.get(1) == Some(&b':'))
}

fn parent(path: &str) -> String {
    match path.rfind('/') {
        Some(i) => path.get(..i).unwrap_or("").to_string(),
        None => String::new(),
    }
}
