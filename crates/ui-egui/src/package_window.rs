//! The Packages window: find a package on CRAN, read about it, install it;
//! see what is installed, update it, remove it, and find where a function comes
//! from. The list of CRAN packages is searched on this computer; it is
//! downloaded only when the user asks (the button says from where), and
//! nothing is installed, updated or removed without a clear yes to a list that
//! shows the source and the exact R call (AGENTS.md section 1).

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use rcraft_engine::{Engine, IndexStatus, PackageIndexService, QueryId};
use rcraft_rcodegen::failure::{diagnose, Diagnosis, Os, Retry};
use rcraft_rcodegen::packages::{
    install_code, install_version_code, installed_packages_code, installed_versions,
    loaded_packages_code, parse_installed, parse_loaded, remove_code, where_code, InstallFrom,
    Installed,
};
use rcraft_rhost::Platform;
use rcraft_rpkg::describe::{describe, Source};
use rcraft_rpkg::index::Package;
use rcraft_rpkg::plan::{compare_versions, plan, updates, Plan, Step, Why};
use rcraft_rpkg::refresh::Stage;
use rcraft_rpkg::search::Filters;

use crate::look::Palette;
use crate::packages::library_dir;

/// Most results listed for a search.
const RESULT_LIMIT: usize = 100;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Find,
    Installed,
}

/// What the window asks the app to run.
pub struct Run {
    pub menu: &'static str,
    pub code: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum Compile {
    #[default]
    Any,
    Yes,
    No,
}

/// Something waiting for the user's yes.
enum Confirm {
    Install {
        plan: Plan,
        code: Result<String, String>,
        menu: &'static str,
        /// A caution to show with the list (an older version is built from source).
        note: Option<String>,
    },
    Remove {
        names: Vec<String>,
        code: Result<String, String>,
    },
}

/// Why the last install did not work, in words, and what can be tried.
struct Failure {
    diagnoses: Vec<Diagnosis>,
    /// The packages that were asked for, with the version each should now have.
    expected: Vec<(String, String)>,
    /// The version typed for "install an older version".
    version_text: String,
    version_note: Option<String>,
}

fn this_os() -> Os {
    match Platform::current() {
        Platform::Windows => Os::Windows,
        Platform::MacOs => Os::MacOs,
        Platform::Linux => Os::Linux,
    }
}

/// `4.4.3` from R's `R version 4.4.3 (2025-02-28 ...)`.
fn r_number(full: &str) -> String {
    full.split_whitespace()
        .skip_while(|w| *w != "version")
        .nth(1)
        .unwrap_or("")
        .to_string()
}

/// Everything R printed for the newest command: its output, warnings and errors.
fn newest_output(engine: &Engine) -> String {
    use rcraft_engine::Item;
    let Some(block) = engine.results().blocks().last() else {
        return String::new();
    };
    let mut out = String::new();
    for item in &block.items {
        match item {
            Item::Text { text, .. } => out.push_str(text),
            Item::Warning(w) => out.push_str(w),
            Item::Error { message, .. } => out.push_str(message),
            Item::Notice(n) => out.push_str(n),
            Item::Plot { .. } => {}
        }
        out.push('\n');
    }
    out
}

/// What R has said about the installed packages.
#[derive(Default)]
struct RState {
    installed: Vec<Installed>,
    loaded: Vec<(String, bool)>,
    asked: Option<(QueryId, QueryId)>,
    known: bool,
    stale: bool,
    seen_busy: bool,
    waiting_for_run: bool,
}

pub struct PackagesWindow {
    pub open: bool,
    tab: Tab,
    query: String,
    licence: String,
    compile: Compile,
    selected: Option<String>,
    installed_filter: String,
    r: RState,
    confirm: Option<Confirm>,
    /// The packages of the install now running (set when the user said yes).
    last_install: Option<Vec<(String, String)>>,
    failure: Option<Failure>,
    mirror_text: String,
    mirror_note: Option<String>,
    show_mirror: bool,
    where_name: String,
    where_query: Option<QueryId>,
    where_pending: Option<String>,
    /// Height of the lists, from the room the window has.
    list_height: f32,
    where_result: Option<String>,
    /// The last search: what it was for, and what it found.
    found_for: Option<(String, String, Compile, u64, usize)>,
    found: Vec<Package>,
}

impl Default for PackagesWindow {
    fn default() -> PackagesWindow {
        PackagesWindow {
            open: false,
            tab: Tab::Find,
            query: String::new(),
            licence: String::new(),
            compile: Compile::Any,
            selected: None,
            installed_filter: String::new(),
            r: RState::default(),
            confirm: None,
            last_install: None,
            failure: None,
            mirror_text: String::new(),
            mirror_note: None,
            show_mirror: false,
            where_name: String::new(),
            where_query: None,
            where_pending: None,
            list_height: 360.0,
            where_result: None,
            found_for: None,
            found: Vec::new(),
        }
    }
}

/// "3 days ago" for an age in seconds.
pub fn ago(seconds: u64) -> String {
    let (n, unit) = match seconds {
        0..=59 => return "just now".to_string(),
        60..=3599 => (seconds / 60, "minute"),
        3600..=86_399 => (seconds / 3600, "hour"),
        _ => (seconds / 86_400, "day"),
    };
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}

/// `host` of a mirror address, for the button that downloads from it.
fn host_of(mirror: &str) -> &str {
    mirror
        .strip_prefix("https://")
        .unwrap_or(mirror)
        .split('/')
        .next()
        .unwrap_or(mirror)
}

fn same_folder(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_lowercase();
    norm(a) == norm(b)
}

impl PackagesWindow {
    /// The names of the installed packages R last listed.
    #[cfg(test)]
    pub(crate) fn installed_names(&self) -> Vec<String> {
        self.r.installed.iter().map(|i| i.name.clone()).collect()
    }

    /// Types a search and picks a package, for pictures.
    pub fn preset(
        &mut self,
        index: &PackageIndexService,
        query: &str,
        select: Option<&str>,
        confirm_install: bool,
    ) {
        self.query = query.to_string();
        self.selected = select.map(str::to_string);
        if let (true, Some(name)) = (confirm_install, select) {
            self.begin_install(index, name, &HashMap::new());
        }
    }

    /// Shows the "did not work" banner for a made-up failure, for pictures:
    /// `compiler`, `newer-r` or `network`, as R prints them on Windows.
    pub fn preset_failure(&mut self, kind: &str) -> Result<(), String> {
        let text = match kind {
            "compiler" => {
                "* installing *source* package 'fixest' ...
** libs
Error in system(paste(MAKE, p1(paste(\"-f\", shQuote(makefiles))), \"compilers\"),  : 
  'make' not found
installation of package 'fixest' had non-zero exit status"
            }
            "newer-r" => {
                "package 'fixest' is not available for this version of R
'fixest' version 0.14.2 is in the repositories but depends on R (>= 4.6.0)"
            }
            "network" => {
                "unable to access index for repository https://cloud.r-project.org/src/contrib:
  cannot open URL 'https://cloud.r-project.org/src/contrib/PACKAGES'"
            }
            other => {
                return Err(format!(
                    "unknown failure {other}; use compiler, newer-r or network"
                ))
            }
        };
        let diagnoses = diagnose(text, Os::Windows, "4.4.3");
        self.failure = Some(Failure {
            diagnoses,
            expected: vec![("fixest".to_string(), "0.14.2".to_string())],
            version_text: String::new(),
            version_note: None,
        });
        Ok(())
    }

    pub fn open_on(&mut self, tab: Tab) {
        self.open = true;
        self.tab = tab;
        self.r.stale = true;
    }

    /// Looks again at the installed packages when R is free: after opening,
    /// and after a command run from here finishes.
    fn service(&mut self, engine: Option<&mut Engine>) {
        let Some(engine) = engine else {
            return;
        };
        if self.r.waiting_for_run {
            if engine.is_busy() {
                self.r.seen_busy = true;
            } else if self.r.seen_busy {
                self.r.waiting_for_run = false;
                self.r.seen_busy = false;
                self.r.stale = true;
                // an install that did not work: say why, in words
                if let Some(expected) = self.last_install.take() {
                    let diagnoses = diagnose(
                        &newest_output(engine),
                        this_os(),
                        &r_number(engine.r_version()),
                    );
                    self.failure = (!diagnoses.is_empty()).then(|| Failure {
                        diagnoses,
                        expected,
                        version_text: String::new(),
                        version_note: None,
                    });
                }
            }
        }
        if let Some((a, b)) = self.r.asked {
            if let (Some(ia), Some(lb)) = (engine.query_result(a), engine.query_result(b)) {
                if ia.ok {
                    self.r.installed = parse_installed(&ia.text);
                    self.r.known = true;
                    // the install the banner is about did in fact work: no banner
                    let have = installed_versions(&self.r.installed);
                    if let Some(f) = &self.failure {
                        let worked = !f.expected.is_empty()
                            && f.expected.iter().all(|(n, v)| {
                                have.get(n).is_some_and(|h| {
                                    compare_versions(h, v) != std::cmp::Ordering::Less
                                })
                            });
                        if worked {
                            self.failure = None;
                        }
                    }
                }
                if lb.ok {
                    self.r.loaded = parse_loaded(&lb.text);
                }
                self.r.asked = None;
            }
        } else if self.r.stale && !engine.is_busy() {
            self.r.stale = false;
            self.r.asked = Some((
                engine.query(installed_packages_code()),
                engine.query(loaded_packages_code()),
            ));
        }
        if let Some(name) = self.where_pending.take() {
            if engine.is_busy() {
                self.where_pending = Some(name);
            } else {
                self.where_query = Some(engine.query(&where_code(&name)));
            }
        }
        if let Some(q) = self.where_query {
            if let Some(r) = engine.query_result(q) {
                self.where_result = Some(if r.ok {
                    let t = r.text.trim();
                    if t.is_empty() {
                        "Not found in any installed package.".to_string()
                    } else {
                        t.lines().map(str::trim).collect::<Vec<_>>().join(", ")
                    }
                } else {
                    "R could not look that up.".to_string()
                });
                self.where_query = None;
            }
        }
    }

    /// Works out what installing `name` would do and asks for the user's yes.
    fn begin_install(
        &mut self,
        index: &PackageIndexService,
        name: &str,
        installed: &HashMap<String, String>,
    ) {
        self.begin_install_of(
            index,
            &[name.to_string()],
            installed,
            InstallFrom::Default,
            "Packages > Install",
            None,
        );
    }

    /// The same for several packages, built the way `from` says, run under the
    /// menu name `menu`. Packages already there up to date are installed again
    /// (the user asked for them).
    fn begin_install_of(
        &mut self,
        index: &PackageIndexService,
        names: &[String],
        installed: &HashMap<String, String>,
        from: InstallFrom,
        menu: &'static str,
        note: Option<String>,
    ) {
        let Some(ix) = index.index() else {
            return;
        };
        let mut have_now = installed.clone();
        for n in names {
            have_now.remove(n);
        }
        let asked: Vec<&str> = names.iter().map(String::as_str).collect();
        let pl = plan(ix, &have_now, &asked);
        let code = if pl.problems.is_empty() {
            install_code(&pl.asked(), library_dir().as_deref(), index.mirror(), from)
        } else {
            Err("fix the problems first".into())
        };
        self.confirm = Some(Confirm::Install {
            plan: pl,
            code,
            menu,
            note,
        });
    }

    /// An older version of `package`, from CRAN's archive, built from source.
    fn begin_older_version(&mut self, index: &PackageIndexService, package: &str, version: &str) {
        let code = install_version_code(package, version, library_dir().as_deref(), index.mirror());
        let plan = Plan {
            steps: vec![Step {
                name: package.to_string(),
                version: version.to_string(),
                replaces: None,
                why: Why::Asked,
                needs_compilation: None,
            }],
            ..Plan::default()
        };
        self.confirm = Some(Confirm::Install {
            plan,
            code,
            menu: "Packages > Install",
            note: Some(
                "An older version is built from source. If the package has compiled code this \
                 needs a compiler, and its own dependencies are installed as usual. R tells you \
                 if the archive has no such version."
                    .to_string(),
            ),
        });
    }

    fn installed_map(&self) -> HashMap<String, String> {
        installed_versions(&self.r.installed)
    }

    /// Draws the window. Returns what to run, when the user said yes.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        pal: &Palette,
        index: &mut PackageIndexService,
        engine: Option<&mut Engine>,
    ) -> Option<Run> {
        if !self.open {
            return None;
        }
        index.ensure_loaded();
        let r_ready = engine.as_ref().is_some_and(|e| !e.is_busy());
        let r_present = engine.is_some();
        self.service(engine);
        let mut open = self.open;
        let mut run = None;
        self.list_height = (ctx.screen_rect().height() - 250.0).clamp(160.0, 520.0);
        egui::Window::new("Packages")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([820.0, 560.0])
            .default_pos(egui::pos2(260.0, 50.0))
            .show(ctx, |ui| {
                self.toolbar(ui, pal, index);
                ui.separator();
                self.failure_banner(ui, pal, index, r_ready);
                if index.index().is_none() && self.tab == Tab::Find {
                    self.no_index(ui, pal, index);
                    return;
                }
                match self.tab {
                    Tab::Find => run = self.find_tab(ui, pal, index, r_present, r_ready),
                    Tab::Installed => run = self.installed_tab(ui, pal, index, r_present, r_ready),
                }
            });
        self.open = open;
        if run.is_some() {
            self.r.waiting_for_run = true;
            self.failure = None;
            self.last_install = match &self.confirm {
                Some(Confirm::Install { plan, .. }) => Some(
                    plan.steps
                        .iter()
                        .filter(|s| s.why == Why::Asked)
                        .map(|s| (s.name.clone(), s.version.clone()))
                        .collect(),
                ),
                _ => None,
            };
            self.r.seen_busy = false;
            self.confirm = None;
        }
        run
    }

    /// What went wrong with the last install, in words, with what to do and
    /// the retries that can be offered. A retry only builds the list to say yes
    /// to; nothing runs until the user does.
    fn failure_banner(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        index: &PackageIndexService,
        r_ready: bool,
    ) {
        let Some(failure) = &mut self.failure else {
            return;
        };
        let mut retry: Option<Retry> = None;
        let mut older: Option<(String, String)> = None;
        let mut close = false;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("The last install did not work")
                        .strong()
                        .color(pal.error),
                );
                if ui.small_button("Close").clicked() {
                    close = true;
                }
            });
            for d in &failure.diagnoses {
                ui.add_space(2.0);
                ui.label(&d.summary);
                for step in &d.steps {
                    ui.label(RichText::new(format!("• {step}")).small().color(pal.dim));
                }
                ui.horizontal_wrapped(|ui| {
                    for r in &d.retry {
                        match r {
                            Retry::Again => {
                                if ui
                                    .add_enabled(r_ready, egui::Button::new("Try again…"))
                                    .clicked()
                                {
                                    retry = Some(Retry::Again);
                                }
                            }
                            Retry::BinaryOnly => {
                                if ui
                                    .add_enabled(r_ready, egui::Button::new("Binary only…"))
                                    .on_hover_text(
                                        "Ask for a ready-built package; no compiler needed",
                                    )
                                    .clicked()
                                {
                                    retry = Some(Retry::BinaryOnly);
                                }
                            }
                            Retry::FromSource => {
                                if ui
                                    .add_enabled(r_ready, egui::Button::new("Build from source…"))
                                    .clicked()
                                {
                                    retry = Some(Retry::FromSource);
                                }
                            }
                            Retry::ChangeMirror => {
                                if ui.button("Change mirror…").clicked() {
                                    retry = Some(Retry::ChangeMirror);
                                }
                            }
                            Retry::OlderVersion(pkg) => {
                                ui.label(format!("An older {pkg}, version:"));
                                ui.add(
                                    egui::TextEdit::singleline(&mut failure.version_text)
                                        .desired_width(70.0)
                                        .hint_text("e.g. 1.2.3"),
                                );
                                if ui
                                    .add_enabled(
                                        r_ready && !failure.version_text.trim().is_empty(),
                                        egui::Button::new("Install it…"),
                                    )
                                    .on_hover_text(
                                        "CRAN keeps old versions under src/contrib/Archive",
                                    )
                                    .clicked()
                                {
                                    older = Some((
                                        pkg.clone(),
                                        failure.version_text.trim().to_string(),
                                    ));
                                }
                            }
                        }
                    }
                });
            }
            if let Some(n) = &failure.version_note {
                ui.label(RichText::new(n).small().color(pal.error));
            }
            ui.label(
                RichText::new("R's own messages are in Results.")
                    .small()
                    .color(pal.dim),
            );
        });
        let names: Vec<String> = failure.expected.iter().map(|(n, _)| n.clone()).collect();
        if close {
            self.failure = None;
            return;
        }
        let installed = self.installed_map();
        if let Some(r) = retry {
            match r {
                Retry::Again => self.begin_install_of(
                    index,
                    &names,
                    &installed,
                    InstallFrom::Default,
                    "Packages > Install",
                    None,
                ),
                Retry::BinaryOnly => self.begin_install_of(
                    index,
                    &names,
                    &installed,
                    InstallFrom::Binary,
                    "Packages > Install",
                    None,
                ),
                Retry::FromSource => self.begin_install_of(
                    index,
                    &names,
                    &installed,
                    InstallFrom::Source,
                    "Packages > Install",
                    Some(
                        "Built from source: packages with compiled code need a compiler."
                            .to_string(),
                    ),
                ),
                Retry::ChangeMirror => {
                    self.show_mirror = true;
                    self.mirror_text = index.mirror().to_string();
                    self.mirror_note = None;
                    self.tab = Tab::Find;
                }
                Retry::OlderVersion(_) => {}
            }
        }
        if let Some((pkg, version)) = older {
            self.begin_older_version(index, &pkg, &version);
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, pal: &Palette, index: &mut PackageIndexService) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Find, "Find on CRAN");
            ui.selectable_value(&mut self.tab, Tab::Installed, "Installed");
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| match index.status().clone() {
                    IndexStatus::Refreshing { progress } => {
                        if ui.button("Cancel").clicked() {
                            index.cancel();
                        }
                        let what = match progress.map(|p| p.stage) {
                            Some(Stage::Details) => "Downloading titles and descriptions",
                            _ => "Downloading the package list",
                        };
                        let frac = progress
                            .and_then(|p| p.total.map(|t| p.done as f32 / t.max(1) as f32))
                            .unwrap_or(0.0);
                        ui.add(
                            egui::ProgressBar::new(frac.clamp(0.0, 1.0))
                                .desired_width(160.0)
                                .text(what),
                        );
                    }
                    IndexStatus::Loading => {
                        ui.label(RichText::new("Reading the saved package list…").color(pal.dim));
                    }
                    _ => {
                        let host = host_of(index.mirror()).to_string();
                        if ui
                            .button("Refresh list")
                            .on_hover_text(format!(
                                "Downloads the CRAN package list (about 15 MB) from {host}. \
                                 Nothing else is sent."
                            ))
                            .clicked()
                        {
                            index.refresh();
                        }
                        if let Some(ix) = index.index() {
                            let age = ix.age(rcraft_engine::package_index::unix_now());
                            ui.label(
                                RichText::new(format!(
                                    "{} packages, fetched {}",
                                    ix.len(),
                                    ago(age)
                                ))
                                .color(pal.dim),
                            );
                        }
                    }
                },
            );
        });
        if let IndexStatus::Failed(why) = index.status() {
            ui.label(RichText::new(why).color(pal.error));
        }
        if let Some(note) = index.note() {
            ui.label(RichText::new(note).color(pal.warning));
        }
    }

    fn no_index(&mut self, ui: &mut egui::Ui, pal: &Palette, index: &mut PackageIndexService) {
        ui.add_space(12.0);
        let busy = index.is_busy();
        if matches!(index.status(), IndexStatus::Loading) {
            return;
        }
        ui.label("The list of CRAN packages has not been downloaded on this computer yet.");
        ui.label(
            RichText::new(format!(
                "It is about 15 MB, comes from {}, and is kept so that searching works with no \
                 internet. Only the request for the two files is sent.",
                host_of(index.mirror())
            ))
            .color(pal.dim),
        );
        ui.add_space(6.0);
        ui.add_enabled_ui(!busy, |ui| {
            if ui.button("Download the package list").clicked() {
                index.refresh();
            }
        });
        ui.add_space(8.0);
        self.mirror_row(ui, pal, index);
    }

    fn mirror_row(&mut self, ui: &mut egui::Ui, pal: &Palette, index: &mut PackageIndexService) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Mirror: {}", index.mirror())).color(pal.dim));
            if ui.small_button("Change…").clicked() {
                self.show_mirror = !self.show_mirror;
                self.mirror_text = index.mirror().to_string();
                self.mirror_note = None;
            }
        });
        if self.show_mirror {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.mirror_text)
                        .desired_width(320.0)
                        .hint_text("https://cloud.r-project.org"),
                );
                if ui.button("Use this mirror").clicked() {
                    self.mirror_note = match index.set_mirror(&self.mirror_text) {
                        Ok(()) => {
                            self.show_mirror = false;
                            None
                        }
                        Err(e) => Some(e),
                    };
                }
                if ui.button("Default").clicked() {
                    index.reset_mirror();
                    self.mirror_text = index.mirror().to_string();
                    self.mirror_note = None;
                }
            });
            if let Some(n) = &self.mirror_note {
                ui.label(RichText::new(n).color(pal.error));
            }
        }
    }

    /// Searches again only when the words, filters or list have changed.
    fn refresh_results(&mut self, index: &PackageIndexService) {
        let Some(ix) = index.index() else {
            return;
        };
        let key = (
            self.query.clone(),
            self.licence.clone(),
            self.compile,
            ix.fetched_at,
            ix.len(),
        );
        if self.found_for.as_ref() == Some(&key) {
            return;
        }
        let filters = Filters {
            licence: Some(self.licence.trim().to_string()).filter(|l| !l.is_empty()),
            needs_compilation: match self.compile {
                Compile::Any => None,
                Compile::Yes => Some(true),
                Compile::No => Some(false),
            },
            ..Filters::default()
        };
        self.found = if self.query.trim().is_empty() && filters.licence.is_none() {
            // nothing asked yet: the newest packages
            ix.recently_published(RESULT_LIMIT)
                .into_iter()
                .filter(|p| match self.compile {
                    Compile::Any => true,
                    Compile::Yes => p.needs_compilation == Some(true),
                    Compile::No => p.needs_compilation == Some(false),
                })
                .cloned()
                .collect()
        } else {
            index
                .search(&self.query, &filters, RESULT_LIMIT)
                .into_iter()
                .map(|(p, _)| p.clone())
                .collect()
        };
        self.found_for = Some(key);
    }

    fn find_tab(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        index: &mut PackageIndexService,
        r_present: bool,
        r_ready: bool,
    ) -> Option<Run> {
        let mut run = None;
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .desired_width(300.0)
                    .hint_text("Search, e.g. fixed effects or survival"),
            );
            ui.label("Licence");
            ui.add(egui::TextEdit::singleline(&mut self.licence).desired_width(70.0));
            egui::ComboBox::from_id_salt("pkg_compile")
                .selected_text(match self.compile {
                    Compile::Any => "Any build",
                    Compile::Yes => "Needs a compiler",
                    Compile::No => "No compiler",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.compile, Compile::Any, "Any build");
                    ui.selectable_value(&mut self.compile, Compile::Yes, "Needs a compiler");
                    ui.selectable_value(&mut self.compile, Compile::No, "No compiler");
                });
        });
        self.refresh_results(index);
        ui.label(
            RichText::new(
                if self.query.trim().is_empty() && self.licence.trim().is_empty() {
                    "Newest on CRAN. Type to search."
                } else {
                    "Best matches first."
                },
            )
            .color(pal.dim)
            .small(),
        );
        ui.add_space(4.0);
        ui.columns(2, |cols| {
            if let Some(col) = cols.get_mut(0) {
                egui::ScrollArea::vertical()
                    .id_salt("pkg_results")
                    .max_height(self.list_height)
                    .auto_shrink([false, false])
                    .show(col, |ui| {
                        if self.found.is_empty() {
                            ui.label(RichText::new("No package matches.").color(pal.dim));
                        }
                        for p in &self.found {
                            let on = self.selected.as_deref() == Some(p.name.as_str());
                            let used = p.used_by_count();
                            let r = ui.selectable_label(
                                on,
                                RichText::new(format!("{}  {}", p.name, p.version)).strong(),
                            );
                            let mut sub = p.title.clone();
                            if sub.chars().count() > 70 {
                                sub = sub.chars().take(69).collect::<String>() + "…";
                            }
                            ui.label(RichText::new(sub).small());
                            ui.label(
                                RichText::new(format!(
                                    "{}{}",
                                    if p.published.is_empty() {
                                        String::new()
                                    } else {
                                        format!("published {}  ", p.published)
                                    },
                                    if used > 0 {
                                        format!("used by {used}")
                                    } else {
                                        String::new()
                                    }
                                ))
                                .small()
                                .color(pal.dim),
                            );
                            ui.add_space(3.0);
                            if r.clicked() {
                                self.selected = Some(p.name.clone());
                                self.confirm = None;
                            }
                        }
                    });
            }
            if let Some(col) = cols.get_mut(1) {
                run = self.detail_pane(col, pal, index, r_present, r_ready);
            }
        });
        ui.add_space(4.0);
        self.mirror_row(ui, pal, index);
        run
    }

    fn detail_pane(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        index: &PackageIndexService,
        r_present: bool,
        r_ready: bool,
    ) -> Option<Run> {
        let mut run = None;
        egui::ScrollArea::vertical()
            .id_salt("pkg_detail")
            .max_height(self.list_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(confirm) = &self.confirm {
                    let (go, cancel) = confirm_panel(ui, pal, confirm, index.mirror(), r_ready);
                    if go {
                        run = confirm.run_for();
                    }
                    if cancel {
                        self.confirm = None;
                    }
                    return;
                }
                let Some(name) = self.selected.clone() else {
                    ui.label(RichText::new("Pick a package to read about it.").color(pal.dim));
                    return;
                };
                let Some(d) = index.index().and_then(|ix| describe(ix, &name)) else {
                    ui.label(RichText::new("That package is not in the list.").color(pal.dim));
                    return;
                };
                let p = &d.package;
                ui.label(
                    RichText::new(format!("{} {}", p.name, p.version))
                        .heading()
                        .strong(),
                );
                ui.label(&p.title);
                ui.add_space(4.0);
                let installed = self.installed_map();
                let have = installed.get(&p.name).cloned();
                match &have {
                    Some(v) => ui.label(
                        RichText::new(format!("Installed: version {v}")).color(pal.accent),
                    ),
                    None if self.r.known => {
                        ui.label(RichText::new("Not installed").color(pal.dim))
                    }
                    None => ui.label(RichText::new("").color(pal.dim)),
                };
                ui.add_enabled_ui(r_present && r_ready, |ui| {
                    if ui
                        .button(if have.is_some() { "Reinstall or update…" } else { "Install…" })
                        .on_disabled_hover_text(if r_present {
                            "R is busy"
                        } else {
                            "R is not available"
                        })
                        .clicked()
                    {
                        self.begin_install(index, &p.name, &installed);
                    }
                });
                ui.add_space(6.0);
                for (label, value) in [
                    ("Published", p.published.as_str()),
                    ("Licence", p.license.as_str()),
                    ("Maintainer", p.maintainer.as_str()),
                    ("Authors", p.authors.as_str()),
                    ("System requirements", p.system_requirements.as_str()),
                ] {
                    if !value.is_empty() {
                        ui.label(RichText::new(label).small().color(pal.dim));
                        ui.label(value);
                    }
                }
                if d.needs_compiler() {
                    ui.label(
                        RichText::new(
                            "Has compiled code: a binary needs nothing extra; building from source needs a compiler.",
                        )
                        .small()
                        .color(pal.warning),
                    );
                }
                ui.add_space(4.0);
                if !p.description.is_empty() {
                    ui.label(&p.description);
                    ui.add_space(4.0);
                }
                for (kind, list) in &d.needs {
                    ui.label(RichText::new(kind.label()).small().color(pal.dim));
                    let text: Vec<String> = list
                        .iter()
                        .map(|n| {
                            let mut s = n.dependency.name.clone();
                            if let Some(c) = &n.dependency.constraint {
                                s.push_str(&format!(" ({c})"));
                            }
                            match n.source {
                                Source::Base => s.push_str(" – comes with R"),
                                Source::Unknown => s.push_str(" – not on CRAN"),
                                Source::Cran => {}
                            }
                            s
                        })
                        .collect();
                    ui.label(text.join(", "));
                }
                if !d.used_by.is_empty() {
                    ui.label(
                        RichText::new(format!("Needed by {} packages", d.used_by.len()))
                            .small()
                            .color(pal.dim),
                    );
                    let shown: Vec<&str> = d.used_by.iter().take(30).map(String::as_str).collect();
                    ui.label(shown.join(", "));
                }
                if !d.links.is_empty() || d.bug_reports.is_some() {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Links (opened in your browser only when you click)")
                            .small()
                            .color(pal.dim),
                    );
                    for l in d.links.iter().chain(d.bug_reports.iter()) {
                        if ui.link(l).clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(l));
                        }
                    }
                }
            });
        run
    }

    fn installed_tab(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        index: &mut PackageIndexService,
        r_present: bool,
        r_ready: bool,
    ) -> Option<Run> {
        let mut run = None;
        if !r_present {
            ui.label(
                RichText::new("R is not available, so the installed packages cannot be listed.")
                    .color(pal.error),
            );
            return None;
        }
        let own = library_dir();
        let installed = self.installed_map();
        let outdated: Vec<_> = index
            .index()
            .map(|ix| {
                updates(ix, &installed)
                    .into_iter()
                    .filter(|u| {
                        // only packages in RCraft's own library are updated from here
                        self.r.installed.iter().any(|i| {
                            i.name == u.name
                                && own.as_deref().is_some_and(|o| same_folder(&i.library, o))
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.installed_filter)
                    .desired_width(220.0)
                    .hint_text("Filter by name"),
            );
            ui.add_enabled_ui(r_ready && !outdated.is_empty(), |ui| {
                if ui
                    .button(format!("Update all ({})…", outdated.len()))
                    .on_disabled_hover_text(if index.index().is_none() {
                        "Download the package list to see updates"
                    } else {
                        "Nothing to update, or R is busy"
                    })
                    .clicked()
                {
                    let names: Vec<String> = outdated.iter().map(|u| u.name.clone()).collect();
                    self.begin_install_of(
                        index,
                        &names,
                        &installed,
                        InstallFrom::Default,
                        "Packages > Update",
                        None,
                    );
                }
            });
            if ui.button("Look again").clicked() {
                self.r.stale = true;
            }
            if index.index().is_none() {
                ui.label(
                    RichText::new("Updates need the package list (Find on CRAN).").color(pal.dim),
                );
            }
        });
        ui.add_space(4.0);
        ui.columns(2, |cols| {
            if let Some(col) = cols.get_mut(0) {
                egui::ScrollArea::vertical()
                    .id_salt("pkg_installed")
                    .max_height(self.list_height)
                    .auto_shrink([false, false])
                    .show(col, |ui| {
                        if !self.r.known {
                            ui.label(RichText::new("Asking R…").color(pal.dim));
                        }
                        let filter = self.installed_filter.trim().to_lowercase();
                        let update_of: HashMap<&str, &str> = outdated
                            .iter()
                            .map(|u| (u.name.as_str(), u.available.as_str()))
                            .collect();
                        egui::Grid::new("pkg_installed_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                for i in &self.r.installed {
                                    if !filter.is_empty()
                                        && !i.name.to_lowercase().contains(&filter)
                                    {
                                        continue;
                                    }
                                    let loaded = self.r.loaded.iter().find(|(n, _)| *n == i.name);
                                    let on = self.selected.as_deref() == Some(i.name.as_str());
                                    if ui.selectable_label(on, &i.name).clicked() {
                                        self.selected = Some(i.name.clone());
                                        self.confirm = None;
                                    }
                                    ui.label(&i.version);
                                    let mut note = String::new();
                                    if let Some(v) = update_of.get(i.name.as_str()) {
                                        note.push_str(&format!("update to {v}  "));
                                    }
                                    if let Some((_, attached)) = loaded {
                                        note.push_str(if *attached {
                                            "attached"
                                        } else {
                                            "loaded"
                                        });
                                    }
                                    ui.label(RichText::new(note).small().color(pal.dim));
                                    ui.end_row();
                                }
                            });
                    });
            }
            if let Some(col) = cols.get_mut(1) {
                run = self.installed_detail(col, pal, index, own.as_deref(), r_ready);
            }
        });
        run
    }

    fn installed_detail(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        index: &PackageIndexService,
        own_library: Option<&str>,
        r_ready: bool,
    ) -> Option<Run> {
        let mut run = None;
        egui::ScrollArea::vertical()
            .id_salt("pkg_installed_detail")
            .max_height(self.list_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(confirm) = &self.confirm {
                    let (go, cancel) = confirm_panel(ui, pal, confirm, index.mirror(), r_ready);
                    if go {
                        run = confirm.run_for();
                    }
                    if cancel {
                        self.confirm = None;
                    }
                    return;
                }
                if let Some(name) = self.selected.clone() {
                    if let Some(i) = self.r.installed.iter().find(|i| i.name == name).cloned() {
                        ui.label(
                            RichText::new(format!("{} {}", i.name, i.version))
                                .heading()
                                .strong(),
                        );
                        ui.label(RichText::new(&i.library).small().color(pal.dim));
                        let mine = own_library.is_some_and(|o| same_folder(&i.library, o));
                        let comes_with_r = matches!(i.priority.as_str(), "base" | "recommended");
                        if comes_with_r {
                            ui.label("Comes with R.");
                        }
                        let removable = mine && !comes_with_r;
                        ui.add_enabled_ui(removable && r_ready, |ui| {
                            if ui
                                .button("Remove…")
                                .on_disabled_hover_text(if comes_with_r {
                                    "Packages that come with R are not removed here"
                                } else if !mine {
                                    "Only packages in RCraft's own library are removed here"
                                } else {
                                    "R is busy"
                                })
                                .clicked()
                            {
                                self.confirm = Some(Confirm::Remove {
                                    names: vec![i.name.clone()],
                                    code: remove_code(&[i.name.as_str()], own_library),
                                });
                            }
                        });
                        if let Some(d) = index.index().and_then(|ix| describe(ix, &i.name)) {
                            ui.add_space(4.0);
                            ui.label(&d.package.title);
                            if !d.used_by.is_empty() {
                                ui.label(
                                    RichText::new(format!(
                                        "{} packages on CRAN need this one.",
                                        d.used_by.len()
                                    ))
                                    .small()
                                    .color(pal.dim),
                                );
                            }
                        }
                    }
                }
                ui.add_space(10.0);
                ui.separator();
                ui.label(RichText::new("Where does a function come from?").strong());
                ui.horizontal(|ui| {
                    let go = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.where_name)
                                .desired_width(140.0)
                                .hint_text("e.g. median"),
                        )
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let clicked = ui
                        .add_enabled(
                            r_ready && !self.where_name.trim().is_empty(),
                            egui::Button::new("Find"),
                        )
                        .clicked();
                    if (go || clicked) && r_ready {
                        self.where_result = None;
                        // asked through the app's engine on the next frame
                        self.where_pending = Some(self.where_name.trim().to_string());
                    }
                });
                if let Some(r) = &self.where_result {
                    ui.label(r);
                }
            });
        run
    }
}

impl Confirm {
    /// The command to run once the user has said yes.
    fn run_for(&self) -> Option<Run> {
        match self {
            Confirm::Install {
                code: Ok(c), menu, ..
            } => Some(Run {
                menu,
                code: c.clone(),
            }),
            Confirm::Remove { code: Ok(c), .. } => Some(Run {
                menu: "Packages > Remove",
                code: c.clone(),
            }),
            _ => None,
        }
    }
}

/// The list shown before anything is installed, updated or removed. Returns
/// (yes, cancel).
fn confirm_panel(
    ui: &mut egui::Ui,
    pal: &Palette,
    confirm: &Confirm,
    mirror: &str,
    r_ready: bool,
) -> (bool, bool) {
    let mut go = false;
    let mut cancel = false;
    match confirm {
        Confirm::Install {
            plan, code, note, ..
        } => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("This will install").strong());
                let can = plan.can_run() && code.is_ok() && r_ready;
                if ui
                    .add_enabled(can, egui::Button::new("Yes, install"))
                    .clicked()
                {
                    go = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
            ui.label(
                RichText::new(format!(
                    "From CRAN, through {}. Packages are binaries where your system has them.",
                    host_of(mirror)
                ))
                .small()
                .color(pal.dim),
            );
            ui.add_space(4.0);
            egui::Grid::new("pkg_confirm_steps")
                .striped(true)
                .show(ui, |ui| {
                    for s in &plan.steps {
                        ui.label(RichText::new(&s.name).strong());
                        ui.label(match &s.replaces {
                            Some(old) => format!("{old} → {}", s.version),
                            None => s.version.clone(),
                        });
                        ui.label(
                            RichText::new(match &s.why {
                                Why::Asked => "asked for".to_string(),
                                Why::NeededBy(by) => format!("needed by {by}"),
                            })
                            .small()
                            .color(pal.dim),
                        );
                        ui.end_row();
                    }
                });
            for p in &plan.problems {
                ui.label(RichText::new(p.message()).color(pal.error));
            }
            if plan.steps.is_empty() && plan.problems.is_empty() {
                ui.label("Nothing to install: everything is already there.");
            }
            let compile = plan.may_compile();
            if !compile.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "{} may have compiled code. If no binary is offered, R builds it from source, \
                         which needs a compiler (Rtools on Windows).",
                        compile.join(", ")
                    ))
                    .small()
                    .color(pal.warning),
                );
                if let Some(note) = note {
                    ui.label(RichText::new(note).small().color(pal.warning));
                }
            }
            ui.label(
                RichText::new(
                    "Installing a package runs code from its authors. Install only packages you trust.",
                )
                .small()
                .color(pal.dim),
            );
            match code {
                Ok(c) => {
                    ui.add_space(4.0);
                    ui.label(RichText::new("R call").small().color(pal.dim));
                    let mut shown = c.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut shown)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    );
                }
                Err(e) => {
                    ui.label(RichText::new(e).color(pal.error));
                }
            }
        }
        Confirm::Remove { names, code } => {
            ui.label(RichText::new(format!("Remove {}?", names.join(", "))).strong());
            ui.label(
                RichText::new(
                    "This deletes the package from RCraft's library. Your data is not touched.",
                )
                .small()
                .color(pal.dim),
            );
            match code {
                Ok(c) => {
                    ui.label(RichText::new("R call").small().color(pal.dim));
                    let mut shown = c.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut shown)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    );
                }
                Err(e) => {
                    ui.label(RichText::new(e).color(pal.error));
                }
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(code.is_ok() && r_ready, egui::Button::new("Yes, remove"))
                    .clicked()
                {
                    go = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        }
    }
    (go, cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ages_are_said_in_the_largest_unit() {
        assert_eq!(ago(5), "just now");
        assert_eq!(ago(60), "1 minute ago");
        assert_eq!(ago(150), "2 minutes ago");
        assert_eq!(ago(3600), "1 hour ago");
        assert_eq!(ago(86_399), "23 hours ago");
        assert_eq!(ago(86_400 * 3), "3 days ago");
    }

    #[test]
    fn the_mirror_host_and_library_folders_compare_sensibly() {
        assert_eq!(host_of("https://cran.example.edu/CRAN"), "cran.example.edu");
        assert_eq!(
            host_of("https://cloud.r-project.org"),
            "cloud.r-project.org"
        );
        assert!(same_folder(
            "C:\\Users\\Me\\R-library\\",
            "c:/users/me/R-library"
        ));
        assert!(!same_folder("C:/a", "C:/b"));
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;
    use rcraft_rpkg::index::sample;

    fn window_and_service() -> (PackagesWindow, PackageIndexService) {
        (
            PackagesWindow::default(),
            PackageIndexService::in_memory(sample()),
        )
    }

    fn found(w: &PackagesWindow) -> Vec<&str> {
        w.found.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn searching_lists_matches_and_an_empty_search_lists_the_newest() {
        let (mut w, s) = window_and_service();
        w.query = "robust".into();
        w.refresh_results(&s);
        assert_eq!(found(&w), vec!["sandwich"]);
        // the same search is not run again
        let before = w.found_for.clone();
        w.refresh_results(&s);
        assert_eq!(w.found_for, before);
        w.query.clear();
        w.refresh_results(&s);
        assert_eq!(w.found.len(), 6);
        w.compile = Compile::Yes;
        w.refresh_results(&s);
        assert!(w.found.is_empty(), "the sample has no compiled package");
    }

    #[test]
    fn asking_to_install_shows_the_plan_and_the_exact_call_before_anything_runs() {
        let (mut w, s) = window_and_service();
        w.begin_install(&s, "fixest", &HashMap::new());
        let Some(Confirm::Install {
            plan, code, menu, ..
        }) = &w.confirm
        else {
            panic!("no confirmation");
        };
        let steps: Vec<&str> = plan.steps.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(steps, vec!["fixest", "Rcpp"]);
        let code = code.as_ref().map(String::as_str).unwrap_or("");
        assert!(code.contains("install.packages(\"fixest\""), "{code}");
        assert!(
            code.contains("repos = \"https://cloud.r-project.org\""),
            "{code}"
        );
        assert_eq!(*menu, "Packages > Install");
        // only a yes produces a command
        assert!(w.confirm.as_ref().and_then(Confirm::run_for).is_some());
    }

    #[test]
    fn a_package_that_is_already_there_is_still_reinstalled_when_asked() {
        let (mut w, s) = window_and_service();
        let have: HashMap<String, String> = [("fixest", "0.12.1"), ("Rcpp", "1.0.14")]
            .iter()
            .map(|(n, v)| ((*n).to_string(), (*v).to_string()))
            .collect();
        w.begin_install(&s, "fixest", &have);
        let Some(Confirm::Install { plan, .. }) = &w.confirm else {
            panic!("no confirmation");
        };
        assert_eq!(plan.asked(), vec!["fixest"]);
        assert_eq!(plan.steps.len(), 1, "Rcpp is installed and new enough");
    }

    #[test]
    fn a_name_cran_lacks_gives_a_problem_and_no_command() {
        let (mut w, s) = window_and_service();
        w.begin_install(&s, "nonesuch", &HashMap::new());
        let Some(Confirm::Install { plan, code, .. }) = &w.confirm else {
            panic!("no confirmation");
        };
        assert!(!plan.can_run());
        assert!(code.is_err());
        assert!(w.confirm.as_ref().and_then(Confirm::run_for).is_none());
    }

    #[test]
    fn removing_names_the_package_and_gives_the_call_only_for_a_valid_name() {
        let ok = Confirm::Remove {
            names: vec!["zoo".into()],
            code: remove_code(&["zoo"], Some("/lib")),
        };
        let run = ok.run_for();
        assert_eq!(run.as_ref().map(|r| r.menu), Some("Packages > Remove"));
        assert!(run.is_some_and(|r| r.code.contains("remove.packages(\"zoo\"")));
        let bad = Confirm::Remove {
            names: vec!["x y".into()],
            code: remove_code(&["x y"], None),
        };
        assert!(bad.run_for().is_none());
    }
}

#[cfg(test)]
mod retry_tests {
    use super::*;
    use rcraft_rpkg::index::sample;

    #[test]
    fn a_binary_only_retry_asks_r_for_binaries_and_a_source_retry_adds_a_caution() {
        let mut w = PackagesWindow::default();
        let s = PackageIndexService::in_memory(sample());
        let names = vec!["fixest".to_string()];
        w.begin_install_of(
            &s,
            &names,
            &HashMap::new(),
            InstallFrom::Binary,
            "Packages > Install",
            None,
        );
        let Some(Confirm::Install { code, note, .. }) = &w.confirm else {
            panic!("no confirmation");
        };
        let code = code.as_ref().map(String::as_str).unwrap_or("");
        assert!(code.contains(", type = \"binary\")"), "{code}");
        assert!(note.is_none());
        w.begin_install_of(
            &s,
            &names,
            &HashMap::new(),
            InstallFrom::Source,
            "Packages > Install",
            Some("Built from source".into()),
        );
        let Some(Confirm::Install { code, note, .. }) = &w.confirm else {
            panic!("no confirmation");
        };
        assert!(code.as_ref().is_ok_and(|c| c.contains("type = \"source\"")));
        assert!(note.is_some());
    }

    #[test]
    fn an_older_version_comes_from_the_archive_with_a_source_caution_and_bad_versions_are_refused()
    {
        let mut w = PackagesWindow::default();
        let s = PackageIndexService::in_memory(sample());
        w.begin_older_version(&s, "fixest", "0.11.2");
        let Some(Confirm::Install {
            plan, code, note, ..
        }) = &w.confirm
        else {
            panic!("no confirmation");
        };
        let c = code.as_ref().map(String::as_str).unwrap_or("");
        assert!(
            c.contains("/src/contrib/Archive/fixest/fixest_0.11.2.tar.gz"),
            "{c}"
        );
        assert!(c.contains("type = \"source\""));
        assert_eq!(plan.steps.len(), 1);
        assert!(plan.can_run());
        assert!(note.as_deref().is_some_and(|n| n.contains("source")));
        // a version that is not one gives no command
        w.begin_older_version(&s, "fixest", "1.0/../../x");
        let Some(Confirm::Install { code, .. }) = &w.confirm else {
            panic!("no confirmation");
        };
        assert!(code.is_err());
        assert!(w.confirm.as_ref().and_then(Confirm::run_for).is_none());
    }

    #[test]
    fn the_r_version_number_is_taken_from_the_banner_line() {
        assert_eq!(
            r_number("R version 4.4.3 (2025-02-28 ucrt) -- \"Trophy Case\""),
            "4.4.3"
        );
        assert_eq!(r_number("nonsense"), "");
    }
}
