//! The Packages window: find a package on CRAN, read about it, install it;
//! see what is installed, update it, remove it, and find where a function comes
//! from. The list of CRAN packages is searched on this computer; it is
//! downloaded only when the user asks (the button says from where), and
//! nothing is installed, updated or removed without a clear yes to a list that
//! shows the source and the exact R call (AGENTS.md section 1).

use std::collections::HashMap;

use eframe::egui::{self, RichText};
use rcraft_engine::sources::{Found, Lookup, LookupState, SourceLookups};
use rcraft_engine::{Engine, IndexStatus, PackageIndexService, QueryId};
use rcraft_rcodegen::failure::{diagnose, Diagnosis, Os, Retry};
use rcraft_rcodegen::packages::{
    install_code, install_version_code, installed_packages_code, installed_versions,
    loaded_packages_code, parse_installed, parse_loaded, remove_code, restore_code, where_code,
    InstallFrom, Installed,
};
use rcraft_rcodegen::rc::{Rc, RcSource};
use rcraft_rcodegen::sources::{
    check_path, describe_local_code, install_bioc_code, install_github_code, install_local_code,
    install_universe_code, LocalKind,
};
use rcraft_rhost::Platform;
use rcraft_rpkg::describe::{describe, Source};
use rcraft_rpkg::index::Package;
use rcraft_rpkg::lockfile::{restore_plan, Action, Entry, Lock, Restore};
use rcraft_rpkg::plan::{compare_versions, plan, updates, Plan, Step, Why};
use rcraft_rpkg::provider::{is_universe, parse_github, Provider};
use rcraft_rpkg::refresh::Stage;
use rcraft_rpkg::search::Filters;

use crate::look::Palette;
use crate::packages::library_dir;

/// Most results listed for a search.
const RESULT_LIMIT: usize = 100;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Find,
    /// The CRAN task views, field by field.
    Browse,
    Installed,
    /// The package list of a project: save it, open it, install from it.
    List,
    /// Bioconductor, r-universe, GitHub and local files.
    Sources,
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
    /// Installing what a saved package list names.
    Restore {
        restore: Restore,
        code: Result<String, String>,
    },
    /// One package from a source other than CRAN, after the CRAN packages it needs.
    Source {
        provider: Provider,
        name: String,
        version: String,
        title: String,
        deps: Vec<Step>,
        problems: Vec<String>,
        note: Option<String>,
        may_compile: bool,
        code: Result<String, String>,
        menu: &'static str,
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
    /// Only packages of this task view are searched.
    view_filter: Option<String>,
    /// The task view open in the Browse tab.
    view_open: Option<String>,
    selected: Option<String>,
    installed_filter: String,
    r: RState,
    confirm: Option<Confirm>,
    /// The packages of the install now running (set when the user said yes).
    last_install: Option<Vec<(String, String)>>,
    failure: Option<Failure>,
    /// The package list opened in the Project list tab, and a message about it.
    lock: Option<Lock>,
    lock_note: Option<String>,
    /// Save only what RCraft's own library holds, not every loaded package.
    lock_library_only: bool,
    /// An `rc` command waiting for R to have listed its packages.
    pending_rc: Option<Rc>,
    /// The Other sources tab.
    source_kind: SourceKind,
    source_text: String,
    source_query: String,
    source_selected: Option<String>,
    /// The tick an unreviewed source needs before the yes.
    source_understood: bool,
    lookups: SourceLookups,
    /// A local file to read (when R is free), R reading it, and what it found.
    peek_pending: Option<String>,
    peek_query: Option<QueryId>,
    peek_found: Option<Result<Package, String>>,
    /// For an `rc` command: the package to pick once the list is in, a
    /// Bioconductor lookup waiting for the R version, and whether to show the
    /// list to say yes to as soon as it can be.
    pending_source_pick: Option<String>,
    pending_bioc_lookup: bool,
    auto_confirm: bool,
    /// What the last `rc` command could not do, in words.
    rc_note: Option<String>,
    /// This R's version number, such as 4.4.3, once the engine has said.
    r_version_number: String,
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
    found_for: Option<(String, String, Compile, u64, usize, Option<String>)>,
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
            view_filter: None,
            view_open: None,
            selected: None,
            installed_filter: String::new(),
            r: RState::default(),
            confirm: None,
            last_install: None,
            failure: None,
            lock: None,
            lock_note: None,
            lock_library_only: false,
            pending_rc: None,
            source_kind: SourceKind::default(),
            source_text: String::new(),
            source_query: String::new(),
            source_selected: None,
            source_understood: false,
            lookups: SourceLookups::for_machine(),
            peek_pending: None,
            peek_query: None,
            peek_found: None,
            pending_source_pick: None,
            pending_bioc_lookup: false,
            auto_confirm: false,
            rc_note: None,
            r_version_number: String::new(),
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
    /// Uses another way of looking sources up (tests use a stand-in for the network).
    #[cfg(test)]
    pub(crate) fn set_lookups(&mut self, lookups: SourceLookups) {
        self.lookups = lookups;
    }

    /// Whether a list is waiting for the user's yes.
    #[cfg(test)]
    pub(crate) fn has_confirmation(&self) -> bool {
        self.confirm.is_some()
    }

    /// The R code of the install list waiting for a yes.
    #[cfg(test)]
    pub(crate) fn confirmation_code(&self) -> Option<String> {
        match &self.confirm {
            Some(Confirm::Install { code: Ok(c), .. }) => Some(c.clone()),
            _ => None,
        }
    }

    /// The names of the installed packages R last listed.
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

    /// An `rc install`, `rc update` or `rc remove` typed in the Command window:
    /// opens this window and, once R has said what is installed, shows the list
    /// to say yes to. Nothing runs before that.
    pub fn start_rc(&mut self, rc: Rc) {
        self.open = true;
        self.tab = match rc {
            Rc::Install { .. } => Tab::Find,
            _ => Tab::Installed,
        };
        self.rc_note = None;
        self.confirm = None;
        self.r.stale = true;
        self.pending_rc = Some(rc);
    }

    /// Carries out a waiting `rc` command when what it needs is known.
    fn apply_pending_rc(&mut self, index: &PackageIndexService, r_present: bool) {
        if self.pending_rc.is_none() {
            return;
        }
        // the saved package list is still being read, or R has not listed its packages
        if matches!(index.status(), IndexStatus::Loading)
            || (r_present && (!self.r.known || self.r.stale || self.r.asked.is_some()))
        {
            return;
        }
        let Some(rc) = self.pending_rc.take() else {
            return;
        };
        if index.index().is_none() {
            self.rc_note = Some(
                "The CRAN package list has not been downloaded yet. Press \"Download the \
                 package list\" under Find on CRAN, then type the command again."
                    .to_string(),
            );
            return;
        }
        let installed = self.installed_map();
        let own = library_dir();
        match rc {
            Rc::Install {
                packages,
                version,
                from,
            } => {
                self.selected = packages.first().cloned();
                if let (Some(v), [one]) = (&version, packages.as_slice()) {
                    self.begin_older_version(index, one, v);
                } else {
                    let note = (from == InstallFrom::Source).then(|| {
                        "Built from source: packages with compiled code need a compiler."
                            .to_string()
                    });
                    self.begin_install_of(
                        index,
                        &packages,
                        &installed,
                        from,
                        "Packages > Install",
                        note,
                    );
                }
            }
            Rc::Update { packages } => {
                let ours: Vec<String> = self
                    .r
                    .installed
                    .iter()
                    .filter(|i| own.as_deref().is_some_and(|o| same_folder(&i.library, o)))
                    .map(|i| i.name.clone())
                    .collect();
                let names: Vec<String> = if packages.is_empty() {
                    index
                        .index()
                        .map(|ix| {
                            updates(ix, &installed)
                                .into_iter()
                                .map(|u| u.name)
                                .filter(|n| ours.contains(n))
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    packages
                };
                if names.is_empty() {
                    self.rc_note = Some(
                        "Nothing to update: no package in RCraft's library has a newer version \
                         on CRAN."
                            .to_string(),
                    );
                    return;
                }
                let not_ours: Vec<&String> = names.iter().filter(|n| !ours.contains(n)).collect();
                if !not_ours.is_empty() {
                    self.rc_note = Some(format!(
                        "Left out, because they are not in RCraft's own library: {}.",
                        not_ours
                            .iter()
                            .map(|n| n.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                let names: Vec<String> = names.into_iter().filter(|n| ours.contains(n)).collect();
                if names.is_empty() {
                    return;
                }
                self.begin_install_of(
                    index,
                    &names,
                    &installed,
                    InstallFrom::Default,
                    "Packages > Update",
                    None,
                );
            }
            Rc::Remove { packages } => {
                let mine: Vec<&String> = packages
                    .iter()
                    .filter(|p| {
                        self.r.installed.iter().any(|i| {
                            &i.name == *p
                                && own.as_deref().is_some_and(|o| same_folder(&i.library, o))
                                && !matches!(i.priority.as_str(), "base" | "recommended")
                        })
                    })
                    .collect();
                if mine.len() != packages.len() {
                    let left: Vec<&str> = packages
                        .iter()
                        .filter(|p| !mine.contains(p))
                        .map(String::as_str)
                        .collect();
                    self.rc_note = Some(format!(
                        "Not removed here, because they are not in RCraft's own library or come \
                         with R: {}.",
                        left.join(", ")
                    ));
                }
                if mine.is_empty() {
                    return;
                }
                let names: Vec<String> = mine.into_iter().cloned().collect();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                self.confirm = Some(Confirm::Remove {
                    code: remove_code(&refs, own.as_deref()),
                    names,
                });
            }
            // the other forms are answered in the Command window
            _ => {}
        }
    }

    /// For pictures: the list to say yes to for a made-up GitHub package that
    /// needs two CRAN packages, as it would look before the tick.
    pub fn preset_source_demo(&mut self, index: &PackageIndexService) {
        self.tab = Tab::Sources;
        self.source_kind = SourceKind::GitHub;
        self.source_text = "someone/madeup".into();
        let package = Package {
            name: "madeup".into(),
            version: "0.3.0.9000".into(),
            title: "A Made-Up Package on GitHub".into(),
            imports: "dplyr, ggplot2".into(),
            needs_compilation: Some(false),
            ..Package::default()
        };
        if let Ok(gh) = parse_github("someone/madeup") {
            self.begin_source_install(index, gh, &package);
        }
    }

    /// Opens a task view in the Browse tab and filters Find by it, for pictures.
    pub fn preset_view(&mut self, view: Option<&str>) {
        self.view_open = view.map(str::to_string);
        self.view_filter = view.map(str::to_string);
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
        if self.r_version_number.is_empty() {
            self.r_version_number = r_number(engine.r_version());
        }
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
        // a local package file to read: R prints its DESCRIPTION, nothing is installed
        if let Some(path) = self.peek_pending.take() {
            if engine.is_busy() {
                self.peek_pending = Some(path);
            } else {
                match describe_local_code(&path) {
                    Ok(code) => self.peek_query = Some(engine.query(&code)),
                    Err(e) => self.peek_found = Some(Err(e)),
                }
            }
        }
        if let Some(q) = self.peek_query {
            if let Some(r) = engine.query_result(q) {
                self.peek_found = Some(if r.ok {
                    package_from_dcf(&r.text)
                } else {
                    Err(format!("R could not read it: {}", r.text.trim()))
                });
                self.peek_query = None;
            }
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
        self.apply_pending_rc(index, r_present);
        self.lookups.poll();
        if self.pending_bioc_lookup && !self.r_version_number.is_empty() {
            self.pending_bioc_lookup = false;
            self.lookups.start(Lookup::Bioconductor {
                r_version: self.r_version_number.clone(),
            });
        }
        self.apply_pending_source(index, r_present);
        if self.lookups.is_busy() || self.peek_query.is_some() || self.peek_pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
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
                if let Some(note) = &self.rc_note {
                    ui.label(RichText::new(note).small().color(pal.warning));
                }
                if index.index().is_none() && self.tab == Tab::Find {
                    self.no_index(ui, pal, index);
                    return;
                }
                match self.tab {
                    Tab::Find => run = self.find_tab(ui, pal, index, r_present, r_ready),
                    Tab::Browse => self.browse_tab(ui, pal, index),
                    Tab::Installed => run = self.installed_tab(ui, pal, index, r_present, r_ready),
                    Tab::List => run = self.list_tab(ui, pal, index, r_present, r_ready),
                    Tab::Sources => run = self.sources_tab(ui, pal, index, r_present, r_ready),
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
                Some(Confirm::Source { name, version, .. }) => {
                    Some(vec![(name.clone(), version.clone())])
                }
                Some(Confirm::Restore { restore, .. }) => Some(
                    restore
                        .steps
                        .iter()
                        .filter(|s| {
                            matches!(s.action, Action::InstallCurrent | Action::InstallOlder)
                        })
                        .map(|s| (s.name.clone(), s.wanted.clone()))
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
            ui.selectable_value(&mut self.tab, Tab::Browse, "Task views");
            ui.selectable_value(&mut self.tab, Tab::Installed, "Installed");
            ui.selectable_value(&mut self.tab, Tab::Sources, "Other sources");
            ui.selectable_value(&mut self.tab, Tab::List, "Project list");
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
            self.view_filter.clone(),
        );
        if self.found_for.as_ref() == Some(&key) {
            return;
        }
        let only = self
            .view_filter
            .as_deref()
            .zip(index.views())
            .map(|(name, views)| views.members_of(name));
        let filters = Filters {
            licence: Some(self.licence.trim().to_string()).filter(|l| !l.is_empty()),
            needs_compilation: match self.compile {
                Compile::Any => None,
                Compile::Yes => Some(true),
                Compile::No => Some(false),
            },
            only,
            ..Filters::default()
        };
        let compile_ok = |p: &Package| match self.compile {
            Compile::Any => true,
            Compile::Yes => p.needs_compilation == Some(true),
            Compile::No => p.needs_compilation == Some(false),
        };
        let nothing_typed = self.query.trim().is_empty() && filters.licence.is_none();
        self.found = if nothing_typed && self.view_filter.is_some() {
            // a task view chosen and no words: its packages, the core ones first
            let order: Vec<String> = self
                .view_filter
                .as_deref()
                .and_then(|n| index.views().and_then(|v| v.get(n)))
                .map(|v| v.packages.iter().map(|p| p.name.clone()).collect())
                .unwrap_or_default();
            order
                .iter()
                .filter_map(|n| ix.get(n))
                .filter(|p| compile_ok(p))
                .take(RESULT_LIMIT)
                .cloned()
                .collect()
        } else if nothing_typed {
            // nothing asked yet: the newest packages
            ix.recently_published(RESULT_LIMIT)
                .into_iter()
                .filter(|p| compile_ok(p))
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
            if let Some(views) = index.views() {
                let shown = self
                    .view_filter
                    .clone()
                    .unwrap_or_else(|| "Any task view".to_string());
                egui::ComboBox::from_id_salt("pkg_view")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.view_filter, None, "Any task view");
                        for v in views.views() {
                            ui.selectable_value(
                                &mut self.view_filter,
                                Some(v.name.clone()),
                                &v.name,
                            );
                        }
                    });
            }
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
                    let (go, cancel) = confirm_panel(
                        ui,
                        pal,
                        confirm,
                        index.mirror(),
                        r_ready,
                        &mut self.source_understood,
                    );
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
if let Some(of) = index.views().map(|v| v.views_of(&p.name)) {                    if !of.is_empty() {                        ui.label(RichText::new("In task views").small().color(pal.dim));                        let names: Vec<&str> = of.iter().map(|v| v.topic.as_str()).collect();                        ui.label(names.join(", "));                    }                }
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
                    let (go, cancel) = confirm_panel(
                        ui,
                        pal,
                        confirm,
                        index.mirror(),
                        r_ready,
                        &mut self.source_understood,
                    );
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
            Confirm::Restore { code: Ok(c), .. } => Some(Run {
                menu: "Packages > Restore",
                code: c.clone(),
            }),
            Confirm::Source {
                code: Ok(c), menu, ..
            } => Some(Run {
                menu,
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
    understood: &mut bool,
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
        Confirm::Restore { restore, code } => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("This will install").strong());
                let can = restore.can_run() && code.is_ok() && r_ready;
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
                    "From CRAN, through {}. The list's versions are installed, not the newest.",
                    host_of(mirror)
                ))
                .small()
                .color(pal.dim),
            );
            ui.add_space(4.0);
            egui::Grid::new("pkg_restore_steps")
                .striped(true)
                .show(ui, |ui| {
                    for s in &restore.steps {
                        let what = match &s.action {
                            Action::InstallCurrent => "current version".to_string(),
                            Action::InstallOlder => "older version, built from source".to_string(),
                            _ => continue,
                        };
                        ui.label(RichText::new(&s.name).strong());
                        ui.label(&s.wanted);
                        ui.label(RichText::new(what).small().color(pal.dim));
                        ui.end_row();
                    }
                });
            for s in &restore.steps {
                if let Action::Skip(why) = &s.action {
                    ui.label(
                        RichText::new(format!("{} {}: left alone.", s.name, why))
                            .small()
                            .color(pal.warning),
                    );
                }
            }
            if let Some(n) = &restore.r_note {
                ui.label(RichText::new(n).small().color(pal.warning));
            }
            if !restore.older().is_empty() {
                ui.label(
                    RichText::new(
                        "Older versions come from CRAN's archive and are built from source: packages \
                         with compiled code need a compiler. What they need that is missing is \
                         installed at its newest version.",
                    )
                    .small()
                    .color(pal.warning),
                );
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
                    ui.label(RichText::new("R calls").small().color(pal.dim));
                    let mut shown = c.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut shown)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    );
                }
                Err(e) => {
                    ui.label(RichText::new(e).color(pal.error));
                }
            }
        }
        Confirm::Source {
            provider,
            name,
            version,
            title,
            deps,
            problems,
            note,
            may_compile,
            code,
            ..
        } => {
            let reviewed = provider.reviewed();
            ui.horizontal(|ui| {
                ui.label(RichText::new("This will install").strong());
                let can =
                    problems.is_empty() && code.is_ok() && r_ready && (reviewed || *understood);
                if ui
                    .add_enabled(can, egui::Button::new("Yes, install"))
                    .on_disabled_hover_text(if !reviewed && !*understood {
                        "Tick the box below first"
                    } else {
                        "Not possible yet: see below"
                    })
                    .clicked()
                {
                    go = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
            ui.label(RichText::new(provider.label()).small().color(if reviewed {
                pal.dim
            } else {
                pal.warning
            }));
            ui.add_space(4.0);
            egui::Grid::new("pkg_source_steps")
                .striped(true)
                .show(ui, |ui| {
                    ui.label(RichText::new(name).strong());
                    ui.label(version);
                    ui.label(
                        RichText::new(format!("from {}", provider.name()))
                            .small()
                            .color(pal.dim),
                    );
                    ui.end_row();
                    for s in deps {
                        ui.label(RichText::new(&s.name).strong());
                        ui.label(&s.version);
                        ui.label(
                            RichText::new(match &s.why {
                                Why::Asked => format!("needed by {name}, from CRAN"),
                                Why::NeededBy(by) => format!("needed by {by}, from CRAN"),
                            })
                            .small()
                            .color(pal.dim),
                        );
                        ui.end_row();
                    }
                });
            if !title.is_empty() {
                ui.label(RichText::new(title).small());
            }
            for p in problems {
                ui.label(RichText::new(p).color(pal.error));
            }
            if let Some(n) = note {
                ui.label(RichText::new(n).small().color(pal.dim));
            }
            if *may_compile {
                ui.label(
                    RichText::new(
                        "This is built from source here. If it has compiled code that needs a \
                         compiler (Rtools on Windows, the command line tools on a Mac).",
                    )
                    .small()
                    .color(pal.warning),
                );
            }
            if !reviewed {
                ui.checkbox(understood, "I understand this code has not been reviewed");
            }
            match code {
                Ok(c) => {
                    ui.label(RichText::new("R calls").small().color(pal.dim));
                    let mut shown = c.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut shown)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(4)
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

impl PackagesWindow {
    /// The entries of the packages R lists, as the package list wants them.
    fn lock_entries(&self) -> Vec<Entry> {
        self.r
            .installed
            .iter()
            .map(|i| Entry {
                name: i.name.clone(),
                version: i.version.clone(),
                priority: i.priority.clone(),
                repository: i.repository.clone(),
            })
            .collect()
    }

    /// The list for what this session has, as the options say.
    fn make_lock(&self, index: &PackageIndexService, r_version: &str) -> Lock {
        let own = library_dir();
        let mut entries = self.lock_entries();
        if self.lock_library_only {
            let in_own: Vec<String> = self
                .r
                .installed
                .iter()
                .filter(|i| own.as_deref().is_some_and(|o| same_folder(&i.library, o)))
                .map(|i| i.name.clone())
                .collect();
            entries.retain(|e| in_own.contains(&e.name));
        }
        let loaded: Vec<String> = self.r.loaded.iter().map(|(n, _)| n.clone()).collect();
        let only = (!self.lock_library_only).then_some(loaded.as_slice());
        Lock::from_entries(
            &entries,
            only,
            r_version,
            index.mirror(),
            &rcraft_engine::package_index::today(),
        )
    }

    /// Reads list text into the tab, or says what is wrong with it.
    pub(crate) fn load_lock_text(&mut self, text: &str) {
        match Lock::parse(text) {
            Ok(lock) => {
                self.lock_note = Some(format!(
                    "Opened a list of {} packages.",
                    lock.packages.len()
                ));
                self.lock = Some(lock);
            }
            Err(e) => {
                self.lock = None;
                self.lock_note = Some(e);
            }
        }
    }

    fn save_list(&mut self, index: &PackageIndexService, r_version: &str) {
        let lock = self.make_lock(index, r_version);
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("rcraft-packages.txt")
            .add_filter("Package list", &["txt"])
            .save_file()
        else {
            return;
        };
        self.lock_note = Some(match std::fs::write(&path, lock.to_text()) {
            Ok(()) => format!(
                "Saved {} packages to {}.",
                lock.packages.len(),
                path.display()
            ),
            Err(e) => format!("Could not save the list: {e}"),
        });
    }

    fn open_list(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Package list", &["txt"])
            .pick_file()
        else {
            return;
        };
        let too_big = std::fs::metadata(&path)
            .map(|m| m.len() > rcraft_rpkg::lockfile::MAX_BYTES as u64)
            .unwrap_or(false);
        if too_big {
            self.lock = None;
            self.lock_note = Some("That file is too large to be a package list.".to_string());
            return;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => self.load_lock_text(&text),
            Err(e) => {
                self.lock = None;
                self.lock_note = Some(format!("Could not read the file: {e}"));
            }
        }
    }

    /// Works out what installing the opened list would do and asks for the yes.
    fn begin_restore(&mut self, restore: Restore, index: &PackageIndexService) {
        let code = restore_code(
            &restore.current(),
            &restore.older(),
            library_dir().as_deref(),
            index.mirror(),
        );
        self.confirm = Some(Confirm::Restore { restore, code });
    }

    fn list_tab(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        index: &mut PackageIndexService,
        r_present: bool,
        r_ready: bool,
    ) -> Option<Run> {
        let mut run = None;
        ui.label(
            "A package list records which packages a project uses and their versions, in a small \
             text file you can keep with the project. Open it later, or on another computer, to \
             install the same packages.",
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut self.lock_library_only,
                "Only packages in RCraft's own library",
            )
            .on_hover_text("Unticked, the list holds the packages loaded in this session");
            ui.add_enabled_ui(r_present && self.r.known, |ui| {
                if ui
                    .button("Save list…")
                    .on_disabled_hover_text("R has not listed its packages yet")
                    .clicked()
                {
                    let version = self.r_version_number.clone();
                    self.save_list(index, &version);
                }
            });
            if ui.button("Open list…").clicked() {
                self.open_list();
            }
        });
        if let Some(n) = &self.lock_note {
            ui.label(RichText::new(n).small().color(pal.dim));
        }
        ui.separator();
        let Some(lock) = self.lock.clone() else {
            ui.label(RichText::new("No list opened.").color(pal.dim));
            return None;
        };
        let installed = self.installed_map();
        let restore = restore_plan(index.index(), &installed, &lock, &self.r_version_number);
        ui.label(
            RichText::new(format!(
                "Made with R {} on {} from {}",
                if lock.r_version.is_empty() {
                    "?"
                } else {
                    &lock.r_version
                },
                if lock.saved.is_empty() {
                    "an unknown day"
                } else {
                    &lock.saved
                },
                if lock.mirror.is_empty() {
                    "an unknown mirror"
                } else {
                    &lock.mirror
                },
            ))
            .small()
            .color(pal.dim),
        );
        if let Some(note) = &restore.r_note {
            ui.label(RichText::new(note).small().color(pal.warning));
        }
        if index.index().is_none() {
            ui.label(
                RichText::new(
                    "Download the package list (Find on CRAN) to see what can be installed.",
                )
                .small()
                .color(pal.warning),
            );
        }
        ui.horizontal(|ui| {
            ui.add_enabled_ui(restore.can_run() && r_ready, |ui| {
                if ui
                    .button("Install what is missing…")
                    .on_disabled_hover_text("Nothing to install, or R is busy")
                    .clicked()
                {
                    self.begin_restore(restore.clone(), index);
                }
            });
        });
        if let Some(Confirm::Restore { .. }) = &self.confirm {
            if let Some(confirm) = &self.confirm {
                let (go, cancel) = confirm_panel(
                    ui,
                    pal,
                    confirm,
                    index.mirror(),
                    r_ready,
                    &mut self.source_understood,
                );
                if go {
                    run = confirm.run_for();
                }
                if cancel {
                    self.confirm = None;
                }
            }
            return run;
        }
        egui::ScrollArea::vertical()
            .id_salt("pkg_list")
            .max_height(self.list_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("pkg_list_grid")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Package").small().color(pal.dim));
                        ui.label(RichText::new("Listed").small().color(pal.dim));
                        ui.label(RichText::new("Installed").small().color(pal.dim));
                        ui.label(RichText::new("What happens").small().color(pal.dim));
                        ui.end_row();
                        for s in &restore.steps {
                            ui.label(&s.name);
                            ui.label(&s.wanted);
                            ui.label(s.have.clone().unwrap_or_else(|| "-".to_string()));
                            let (text, color) = match &s.action {
                                Action::AlreadyThere => ("already installed".to_string(), pal.dim),
                                Action::InstallCurrent => ("install".to_string(), pal.accent),
                                Action::InstallOlder => {
                                    ("install this older version".to_string(), pal.accent)
                                }
                                Action::Skip(why) => (format!("left alone: {why}"), pal.warning),
                            };
                            ui.label(RichText::new(text).small().color(color));
                            ui.end_row();
                        }
                    });
            });
        run
    }
}

impl PackagesWindow {
    /// The CRAN task views: the fields CRAN keeps curated lists for, each with
    /// its packages (the core ones first). Clicking a package goes to its page.
    fn browse_tab(&mut self, ui: &mut egui::Ui, pal: &Palette, index: &PackageIndexService) {
        let Some(views) = index.views() else {
            ui.add_space(8.0);
            ui.label("The task views have not been downloaded yet.");
            ui.label(
                RichText::new(
                    "They come with the package list: press \"Refresh list\" (a few more kilobytes).",
                )
                .color(pal.dim),
            );
            return;
        };
        let mut go: Option<(String, Option<String>)> = None;
        ui.columns(2, |cols| {
            if let Some(col) = cols.get_mut(0) {
                egui::ScrollArea::vertical()
                    .id_salt("pkg_views")
                    .max_height(self.list_height)
                    .auto_shrink([false, false])
                    .show(col, |ui| {
                        for v in views.views() {
                            let on = self.view_open.as_deref() == Some(v.name.as_str());
                            if ui
                                .selectable_label(
                                    on,
                                    RichText::new(if v.topic.is_empty() {
                                        &v.name
                                    } else {
                                        &v.topic
                                    })
                                    .strong(),
                                )
                                .clicked()
                            {
                                self.view_open = Some(v.name.clone());
                            }
                            ui.label(
                                RichText::new(format!("{} packages", v.packages.len()))
                                    .small()
                                    .color(pal.dim),
                            );
                            ui.add_space(2.0);
                        }
                    });
            }
            if let Some(col) = cols.get_mut(1) {
                let Some(view) = self.view_open.as_deref().and_then(|n| views.get(n)) else {
                    col.label(RichText::new("Pick a field to see its packages.").color(pal.dim));
                    return;
                };
                col.label(RichText::new(&view.topic).heading().strong());
                col.label(
                    RichText::new(format!(
                        "Kept by {}, version of {}",
                        if view.maintainer.is_empty() {
                            "its maintainers"
                        } else {
                            &view.maintainer
                        },
                        if view.updated.is_empty() {
                            "an unknown day"
                        } else {
                            &view.updated
                        }
                    ))
                    .small()
                    .color(pal.dim),
                );
                col.horizontal(|ui| {
                    if ui
                        .button("Search inside this view")
                        .on_hover_text("Goes to Find on CRAN with this view as the filter")
                        .clicked()
                    {
                        go = Some((String::new(), Some(view.name.clone())));
                    }
                    if !view.url.is_empty() && ui.link("Its web page").clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(&view.url));
                    }
                });
                egui::ScrollArea::vertical()
                    .id_salt("pkg_view_packages")
                    .max_height(self.list_height)
                    .auto_shrink([false, false])
                    .show(col, |ui| {
                        for p in &view.packages {
                            let known = index.index().and_then(|ix| ix.get(&p.name));
                            ui.horizontal(|ui| {
                                if ui.link(RichText::new(&p.name).strong()).clicked() {
                                    go = Some((p.name.clone(), None));
                                }
                                if p.core {
                                    ui.label(RichText::new("core").small().color(pal.accent));
                                }
                                if known.is_none() {
                                    ui.label(
                                        RichText::new("not on CRAN").small().color(pal.warning),
                                    );
                                }
                            });
                            if let Some(k) = known {
                                ui.label(RichText::new(&k.title).small().color(pal.dim));
                            }
                        }
                    });
            }
        });
        if let Some((words, view)) = go {
            self.tab = Tab::Find;
            self.query = words.clone();
            self.view_filter = view;
            self.selected = (!words.is_empty()).then_some(words);
            self.confirm = None;
        }
    }
}

/// The source picked in the Other sources tab.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SourceKind {
    #[default]
    Bioconductor,
    Universe,
    GitHub,
    Local,
}

/// The first package of a DESCRIPTION printed by R or fetched from GitHub.
fn package_from_dcf(text: &str) -> Result<Package, String> {
    let table =
        rcraft_rpkg::dcf::table(text, rcraft_rpkg::index::DB_COLUMNS).map_err(|e| e.to_string())?;
    rcraft_rpkg::index::Index::from_tables(Some(&table), None, 0)
        .packages()
        .first()
        .cloned()
        .ok_or_else(|| "no package DESCRIPTION was found".to_string())
}

impl PackagesWindow {
    /// An `rc install ..., provider(...)` or a path typed in the Command window:
    /// opens Other sources, looks the source up (the typed command is the
    /// request), and shows the list to say yes to once it is known.
    pub fn start_source(&mut self, source: RcSource, what: String) {
        self.open = true;
        self.tab = Tab::Sources;
        self.confirm = None;
        self.rc_note = None;
        self.source_query.clear();
        self.source_selected = None;
        self.auto_confirm = true;
        self.r.stale = true;
        match source {
            RcSource::Bioconductor => {
                self.source_kind = SourceKind::Bioconductor;
                self.pending_source_pick = Some(what);
                self.pending_bioc_lookup = true;
            }
            RcSource::Universe(u) => {
                self.source_kind = SourceKind::Universe;
                self.source_text = u.clone();
                self.pending_source_pick = Some(what);
                self.lookups.start(Lookup::Universe(u));
            }
            RcSource::GitHub => {
                self.source_kind = SourceKind::GitHub;
                self.source_text = what.clone();
                if let Ok(Provider::GitHub {
                    owner,
                    repo,
                    reference,
                }) = parse_github(&what)
                {
                    self.lookups.start(Lookup::GitHub {
                        owner,
                        repo,
                        reference,
                    });
                }
            }
            RcSource::Local => {
                self.source_kind = SourceKind::Local;
                self.source_text = what.clone();
                self.peek_found = None;
                self.peek_pending = Some(what);
            }
        }
    }

    /// Builds the list to say yes to for `package` from `provider`, with the
    /// CRAN packages it needs when it is a GitHub repository or a local file.
    fn begin_source_install(
        &mut self,
        index: &PackageIndexService,
        provider: Provider,
        package: &Package,
    ) {
        let installed = self.installed_map();
        let library = library_dir();
        let mirror = index.mirror().to_string();
        let mut deps: Vec<Step> = Vec::new();
        let mut problems: Vec<String> = Vec::new();
        let builds_here = match &provider {
            Provider::GitHub { .. } => true,
            Provider::Local(path) => LocalKind::of(path).is_source(),
            _ => false,
        };
        let plans_needs = matches!(provider, Provider::GitHub { .. } | Provider::Local(_));
        if plans_needs {
            // what it needs that is not installed already at a version it accepts
            let needs: Vec<String> = package
                .hard_dependencies()
                .into_iter()
                .filter(|d| !rcraft_rpkg::describe::is_base_package(&d.name))
                .filter(|d| {
                    installed.get(&d.name).is_none_or(|have| {
                        d.constraint
                            .as_deref()
                            .is_some_and(|c| !rcraft_rpkg::plan::satisfies(have, c))
                    })
                })
                .map(|d| d.name)
                .collect();
            let missing: Vec<&str> = needs
                .iter()
                .filter(|n| !installed.contains_key(*n))
                .map(String::as_str)
                .collect();
            match index.index() {
                Some(ix) if !needs.is_empty() => {
                    let refs: Vec<&str> = needs.iter().map(String::as_str).collect();
                    let pl = plan(ix, &installed, &refs);
                    for p in &pl.problems {
                        problems.push(match p {
                            rcraft_rpkg::plan::Problem::NotFound { name, .. } => format!(
                                "{} needs \"{name}\", which is not on CRAN. Install it first from its own source.",
                                package.name
                            ),
                            other => other.message(),
                        });
                    }
                    deps = pl.steps;
                }
                None if !missing.is_empty() => problems.push(format!(
                    "{} needs {}. Download the CRAN package list (Find on CRAN) so RCraft can \
                     install those first.",
                    package.name,
                    missing.join(", ")
                )),
                _ => {}
            }
        }
        let asked: Vec<&str> = deps
            .iter()
            .filter(|s| s.why == Why::Asked)
            .map(|s| s.name.as_str())
            .collect();
        let (code, menu, note) = match &provider {
            Provider::GitHub {
                owner,
                repo,
                reference,
            } => (
                install_github_code(owner, repo, reference, &asked, library.as_deref(), &mirror),
                "Packages > Install from GitHub",
                Some(
                    "R downloads the archive from github.com and builds it here. Packages in a \
                     subfolder of a repository are not handled."
                        .to_string(),
                ),
            ),
            Provider::Local(path) => (
                install_local_code(path, &asked, library.as_deref(), &mirror),
                "Packages > Install from a file",
                None,
            ),
            Provider::Universe(u) => (
                install_universe_code(u, &[package.name.as_str()], library.as_deref(), &mirror),
                "Packages > Install from r-universe",
                Some(
                    "R installs what it needs from the universe or, failing that, from CRAN."
                        .to_string(),
                ),
            ),
            Provider::Bioconductor => (
                install_bioc_code(&[package.name.as_str()], library.as_deref(), &mirror),
                "Packages > Install from Bioconductor",
                Some(
                    "BiocManager installs what it needs from Bioconductor and CRAN. If BiocManager \
                     is missing it is installed from CRAN first."
                        .to_string(),
                ),
            ),
            Provider::Cran => (
                Err("CRAN packages are installed from Find on CRAN".to_string()),
                "Packages > Install",
                None,
            ),
        };
        self.source_understood = false;
        self.confirm = Some(Confirm::Source {
            provider,
            name: package.name.clone(),
            version: package.version.clone(),
            title: package.title.clone(),
            deps,
            problems,
            note,
            may_compile: builds_here && package.needs_compilation != Some(false),
            code,
            menu,
        });
    }

    /// One package from another source, with an Install button. Returns
    /// whether it was pressed.
    fn source_detail(&self, ui: &mut egui::Ui, pal: &Palette, p: &Package, r_ready: bool) -> bool {
        ui.label(
            RichText::new(format!("{} {}", p.name, p.version))
                .heading()
                .strong(),
        );
        if !p.title.is_empty() {
            ui.label(&p.title);
        }
        if let Some(v) = self.installed_map().get(&p.name) {
            ui.label(RichText::new(format!("Installed: version {v}")).color(pal.accent));
        }
        let pressed = ui
            .add_enabled(r_ready, egui::Button::new("Install…"))
            .on_disabled_hover_text("R is busy or not available")
            .clicked();
        for (label, value) in [
            ("Depends", p.depends.as_str()),
            ("Imports", p.imports.as_str()),
            ("LinkingTo", p.linking_to.as_str()),
            ("Licence", p.license.as_str()),
        ] {
            if !value.is_empty() {
                ui.label(RichText::new(label).small().color(pal.dim));
                ui.label(value);
            }
        }
        if !p.description.is_empty() {
            ui.add_space(4.0);
            ui.label(&p.description);
        }
        pressed
    }

    fn sources_tab(
        &mut self,
        ui: &mut egui::Ui,
        pal: &Palette,
        index: &PackageIndexService,
        r_present: bool,
        r_ready: bool,
    ) -> Option<Run> {
        let mut run = None;
        ui.horizontal(|ui| {
            for (kind, label) in [
                (SourceKind::Bioconductor, "Bioconductor"),
                (SourceKind::Universe, "r-universe"),
                (SourceKind::GitHub, "GitHub"),
                (SourceKind::Local, "A file on this computer"),
            ] {
                if ui.radio_value(&mut self.source_kind, kind, label).changed() {
                    self.confirm = None;
                    self.source_selected = None;
                    self.source_query.clear();
                    self.peek_found = None;
                    self.lookups.clear();
                }
            }
        });
        let busy = self.lookups.is_busy();
        let mut start: Option<Lookup> = None;
        ui.horizontal(|ui| match self.source_kind {
            SourceKind::Bioconductor => {
                let r = self.r_version_number.clone();
                ui.label(format!(
                    "Bioconductor's software packages for R {}",
                    if r.is_empty() { "?" } else { &r }
                ));
                let l = Lookup::Bioconductor {
                    r_version: r.clone(),
                };
                if ui
                    .add_enabled(!busy && !r.is_empty(), egui::Button::new("Look up"))
                    .on_hover_text(format!("Asks {}", l.addresses().join(" and ")))
                    .clicked()
                {
                    start = Some(l);
                }
            }
            SourceKind::Universe => {
                ui.add(
                    egui::TextEdit::singleline(&mut self.source_text)
                        .desired_width(200.0)
                        .hint_text("universe, e.g. ropensci"),
                );
                let name = self.source_text.trim().to_string();
                let ok = is_universe(&name);
                let l = Lookup::Universe(name);
                if ui
                    .add_enabled(!busy && ok, egui::Button::new("Look up"))
                    .on_hover_text(if ok {
                        format!("Asks {}", l.addresses().join(" and "))
                    } else {
                        "Type the universe's name (its owner on GitHub)".to_string()
                    })
                    .clicked()
                {
                    start = Some(l);
                }
            }
            SourceKind::GitHub => {
                ui.add(
                    egui::TextEdit::singleline(&mut self.source_text)
                        .desired_width(260.0)
                        .hint_text("owner/repo or owner/repo@tag"),
                );
                let parsed = parse_github(&self.source_text);
                let lookup = match &parsed {
                    Ok(Provider::GitHub {
                        owner,
                        repo,
                        reference,
                    }) => Some(Lookup::GitHub {
                        owner: owner.clone(),
                        repo: repo.clone(),
                        reference: reference.clone(),
                    }),
                    _ => None,
                };
                let hover = match (&lookup, &parsed) {
                    (Some(l), _) => format!("Asks {}", l.addresses().join(" and ")),
                    (None, Err(e)) => e.clone(),
                    (None, Ok(_)) => String::new(),
                };
                if ui
                    .add_enabled(!busy && lookup.is_some(), egui::Button::new("Look up"))
                    .on_hover_text(hover)
                    .clicked()
                {
                    start = lookup;
                }
            }
            SourceKind::Local => {
                ui.add(
                    egui::TextEdit::singleline(&mut self.source_text)
                        .desired_width(300.0)
                        .hint_text("a .tar.gz, .zip or .tgz file, or a package folder"),
                );
                if ui.button("File…").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("R package", &["gz", "zip", "tgz"])
                        .pick_file()
                    {
                        self.source_text = p.to_string_lossy().into_owned();
                    }
                }
                if ui.button("Folder…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        self.source_text = p.to_string_lossy().into_owned();
                    }
                }
                let ok = check_path(&self.source_text).is_ok() && r_present;
                if ui
                    .add_enabled(
                        ok && self.peek_query.is_none(),
                        egui::Button::new("Read it"),
                    )
                    .on_hover_text("R reads the package's DESCRIPTION; nothing is installed")
                    .clicked()
                {
                    self.peek_found = None;
                    self.confirm = None;
                    self.peek_pending = Some(self.source_text.trim().to_string());
                }
            }
        });
        if let Some(l) = start {
            self.confirm = None;
            self.source_selected = None;
            self.lookups.start(l);
        }
        match self.lookups.state() {
            LookupState::Running(l) => {
                let a = l.addresses().join(" and ");
                let mut stop = false;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Asking {a}…")).color(pal.dim));
                    stop = ui.button("Cancel").clicked();
                });
                if stop {
                    self.lookups.cancel();
                }
            }
            LookupState::Failed(m) => {
                ui.label(RichText::new(m).color(pal.error));
            }
            _ => {}
        }
        if self.source_kind == SourceKind::Local {
            if self.peek_query.is_some() || self.peek_pending.is_some() {
                ui.label(RichText::new("R is reading the file…").color(pal.dim));
            }
            if let Some(Err(e)) = &self.peek_found {
                ui.label(RichText::new(e).color(pal.error));
            }
        }
        ui.separator();

        // a list to say yes to takes the place of the details
        if matches!(self.confirm, Some(Confirm::Source { .. })) {
            egui::ScrollArea::vertical()
                .id_salt("pkg_source_confirm")
                .max_height(self.list_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(confirm) = &self.confirm {
                        let (go, cancel) = confirm_panel(
                            ui,
                            pal,
                            confirm,
                            index.mirror(),
                            r_ready,
                            &mut self.source_understood,
                        );
                        if go {
                            run = confirm.run_for();
                        }
                        if cancel {
                            self.confirm = None;
                        }
                    }
                });
            return run;
        }

        let mut chosen: Option<(Provider, Package)> = None;
        match (self.source_kind, self.lookups.state()) {
            (SourceKind::Local, _) => {
                if let Some(Ok(p)) = &self.peek_found {
                    let p = p.clone();
                    if self.source_detail(ui, pal, &p, r_ready) {
                        chosen = Some((Provider::Local(self.source_text.trim().to_string()), p));
                    }
                }
            }
            (_, LookupState::Done(found)) => match &**found {
                Found::Description { provider, package } => {
                    let p = (**package).clone();
                    let pr = provider.clone();
                    if self.source_detail(ui, pal, &p, r_ready) {
                        chosen = Some((pr, p));
                    }
                }
                Found::List {
                    provider,
                    release,
                    index: list,
                    searcher,
                } => {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.source_query)
                                .desired_width(260.0)
                                .hint_text("Search this list"),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} packages from {}{}",
                                list.len(),
                                provider.name(),
                                release
                                    .as_deref()
                                    .map(|r| format!(" {r}"))
                                    .unwrap_or_default()
                            ))
                            .small()
                            .color(pal.dim),
                        );
                    });
                    let hits: Vec<Package> = if self.source_query.trim().is_empty() {
                        list.packages().iter().take(RESULT_LIMIT).cloned().collect()
                    } else {
                        searcher
                            .search(list, &self.source_query, &Filters::default(), RESULT_LIMIT)
                            .iter()
                            .filter_map(|h| list.packages().get(h.position))
                            .cloned()
                            .collect()
                    };
                    let selected = self
                        .source_selected
                        .as_deref()
                        .and_then(|n| list.get(n))
                        .cloned();
                    let provider = provider.clone();
                    let mut pick: Option<String> = None;
                    ui.columns(2, |cols| {
                        if let Some(col) = cols.get_mut(0) {
                            egui::ScrollArea::vertical()
                                .id_salt("pkg_source_list")
                                .max_height(self.list_height)
                                .auto_shrink([false, false])
                                .show(col, |ui| {
                                    for p in &hits {
                                        let on = self.source_selected.as_deref() == Some(&p.name);
                                        if ui
                                            .selectable_label(
                                                on,
                                                RichText::new(format!("{}  {}", p.name, p.version))
                                                    .strong(),
                                            )
                                            .clicked()
                                        {
                                            pick = Some(p.name.clone());
                                        }
                                        if !p.title.is_empty() {
                                            ui.label(RichText::new(&p.title).small());
                                        }
                                    }
                                });
                        }
                        if let Some(col) = cols.get_mut(1) {
                            if let Some(p) = &selected {
                                egui::ScrollArea::vertical()
                                    .id_salt("pkg_source_detail")
                                    .max_height(self.list_height)
                                    .auto_shrink([false, false])
                                    .show(col, |ui| {
                                        if self.source_detail(ui, pal, p, r_ready) {
                                            chosen = Some((provider.clone(), p.clone()));
                                        }
                                    });
                            } else {
                                col.label(
                                    RichText::new("Pick a package to read about it.")
                                        .color(pal.dim),
                                );
                            }
                        }
                    });
                    if let Some(n) = pick {
                        self.source_selected = Some(n);
                    }
                }
            },
            _ => {
                ui.label(
                    RichText::new(match self.source_kind {
                        SourceKind::Bioconductor => {
                            "Look up Bioconductor's list to search it. Its packages are reviewed by the Bioconductor project."
                        }
                        SourceKind::Universe => {
                            "Type a universe and look it up. r-universe builds packages from their owners' repositories; nobody reviews them."
                        }
                        SourceKind::GitHub => {
                            "Type owner/repository and look it up. Code from GitHub is not reviewed by anyone."
                        }
                        SourceKind::Local => {
                            "Pick a package file or folder and press Read it. Nothing is installed until you say yes."
                        }
                    })
                    .color(pal.dim),
                );
            }
        }
        if let Some((provider, package)) = chosen {
            self.begin_source_install(index, provider, &package);
        }
        run
    }

    /// After a lookup an `rc` command started: picks the package it named and
    /// shows the list to say yes to.
    fn apply_pending_source(&mut self, index: &PackageIndexService, r_present: bool) {
        if !self.auto_confirm || self.confirm.is_some() || self.lookups.is_busy() {
            return;
        }
        if r_present && (!self.r.known || self.r.stale || self.r.asked.is_some()) {
            return;
        }
        let chosen: Option<(Provider, Package)> = match (self.source_kind, self.lookups.state()) {
            (SourceKind::Local, _) => match &self.peek_found {
                Some(Ok(p)) => Some((
                    Provider::Local(self.source_text.trim().to_string()),
                    p.clone(),
                )),
                Some(Err(_)) => {
                    self.auto_confirm = false;
                    None
                }
                None => None,
            },
            (_, LookupState::Done(found)) => match &**found {
                Found::Description { provider, package } => {
                    Some((provider.clone(), (**package).clone()))
                }
                Found::List {
                    provider,
                    index: list,
                    ..
                } => {
                    let want = self.pending_source_pick.clone().unwrap_or_default();
                    match list.get(&want) {
                        Some(p) => Some((provider.clone(), p.clone())),
                        None => {
                            self.rc_note = Some(format!(
                                "There is no package \"{want}\" in {}.",
                                provider.name()
                            ));
                            self.auto_confirm = false;
                            None
                        }
                    }
                }
            },
            (_, LookupState::Failed(_)) => {
                self.auto_confirm = false;
                None
            }
            _ => None,
        };
        if let Some((provider, package)) = chosen {
            self.auto_confirm = false;
            self.source_selected = Some(package.name.clone());
            self.begin_source_install(index, provider, &package);
        }
    }
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

#[cfg(test)]
mod list_tests {
    use super::*;
    use rcraft_rpkg::index::sample;

    fn installed(name: &str, version: &str, priority: &str, repository: &str) -> Installed {
        Installed {
            name: name.into(),
            version: version.into(),
            library: "C:/lib".into(),
            priority: priority.into(),
            repository: repository.into(),
        }
    }

    fn window() -> PackagesWindow {
        let mut w = PackagesWindow::default();
        w.r.installed = vec![
            installed("stats", "4.4.3", "base", ""),
            installed("fixest", "0.12.1", "", "CRAN"),
            installed("zoo", "1.8-12", "", "CRAN"),
            installed("mine", "0.1", "", ""),
        ];
        w.r.loaded = vec![
            ("stats".to_string(), true),
            ("fixest".to_string(), true),
            ("mine".to_string(), false),
        ];
        w
    }

    #[test]
    fn the_saved_list_holds_the_loaded_packages_with_where_each_came_from() {
        let w = window();
        let s = PackageIndexService::in_memory(sample());
        let lock = w.make_lock(&s, "4.4.3");
        let got: Vec<(&str, &str, &str)> = lock
            .packages
            .iter()
            .map(|p| {
                (
                    p.name.as_str(),
                    p.version.as_str(),
                    match p.source {
                        rcraft_rpkg::lockfile::LockSource::Cran => "cran",
                        rcraft_rpkg::lockfile::LockSource::Base => "base",
                        rcraft_rpkg::lockfile::LockSource::Other => "other",
                    },
                )
            })
            .collect();
        // zoo is installed but not loaded, so it is not in the list
        assert_eq!(
            got,
            vec![
                ("fixest", "0.12.1", "cran"),
                ("mine", "0.1", "other"),
                ("stats", "4.4.3", "base")
            ]
        );
        assert_eq!(lock.r_version, "4.4.3");
        assert_eq!(lock.mirror, "https://cloud.r-project.org");
        assert_eq!(lock.saved.len(), 10);
    }

    #[test]
    fn an_opened_list_is_read_or_refused_with_a_reason() {
        let mut w = window();
        w.load_lock_text(
            "# RCraft package list (version 1)\nr\t4.4.3\npackage\tzoo\t1.8-12\tcran\n",
        );
        assert_eq!(w.lock.as_ref().map(|l| l.packages.len()), Some(1));
        assert!(w
            .lock_note
            .as_deref()
            .is_some_and(|n| n.contains("1 packages")));
        w.load_lock_text("not a list");
        assert!(w.lock.is_none());
        assert!(w
            .lock_note
            .as_deref()
            .is_some_and(|n| n.contains("not an RCraft package list")));
    }

    #[test]
    fn restoring_a_list_asks_for_a_yes_with_both_kinds_of_install() {
        let mut w = window();
        let s = PackageIndexService::in_memory(sample());
        let lock = Lock::parse(
            "# RCraft package list (version 1)\nr\t4.4.3\npackage\tzoo\t1.8-12\tcran\npackage\tfixest\t0.11.0\tcran\npackage\tmine\t0.1\tother\n",
        )
        .unwrap_or_default();
        // an older zoo is installed, so the list's zoo has to be installed again
        w.r.installed.retain(|i| i.name != "zoo");
        w.r.installed.push(installed("zoo", "1.8-10", "", "CRAN"));
        let have = w.installed_map();
        let restore = restore_plan(s.index(), &have, &lock, "4.4.3");
        // zoo 1.8-12 is what the made-up index offers: current; fixest 0.11.0 is older than its 0.12.1
        assert_eq!(restore.current(), vec!["zoo"]);
        assert_eq!(restore.older(), vec![("fixest", "0.11.0")]);
        w.begin_restore(restore, &s);
        let Some(Confirm::Restore { code, .. }) = &w.confirm else {
            panic!("no confirmation");
        };
        let code = code.as_ref().map(String::as_str).unwrap_or("");
        assert!(
            code.contains("Archive/fixest/fixest_0.11.0.tar.gz"),
            "{code}"
        );
        assert!(code.contains("install.packages(\"zoo\""), "{code}");
        let run = w.confirm.as_ref().and_then(Confirm::run_for);
        assert_eq!(run.map(|r| r.menu), Some("Packages > Restore"));
    }
}

#[cfg(test)]
mod view_tests {
    use super::*;
    use rcraft_rpkg::index::sample;
    use rcraft_rpkg::views::{TaskView, ViewPackage, Views};

    fn service() -> PackageIndexService {
        let mut s = PackageIndexService::in_memory(sample());
        let vp = |n: &str, core: bool| ViewPackage {
            name: n.into(),
            core,
        };
        s.set_views(Views::new(vec![
            TaskView {
                name: "Econ".into(),
                topic: "Econometrics".into(),
                packages: vec![
                    vp("zoo", false),
                    vp("AER", true),
                    vp("sandwich", true),
                    vp("gone", false),
                ],
                ..TaskView::default()
            },
            TaskView {
                name: "Other".into(),
                topic: "Other".into(),
                packages: vec![vp("fixest", true)],
                ..TaskView::default()
            },
        ]));
        s
    }

    fn found(w: &PackagesWindow) -> Vec<&str> {
        w.found.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn a_chosen_task_view_with_no_words_lists_its_packages_core_first_and_skips_unlisted_ones() {
        let s = service();
        let mut w = PackagesWindow {
            view_filter: Some("Econ".into()),
            ..PackagesWindow::default()
        };
        w.refresh_results(&s);
        // core packages first, then the rest; "gone" is not in the index so it is left out
        assert_eq!(found(&w), vec!["AER", "sandwich", "zoo"]);
    }

    #[test]
    fn words_search_only_inside_the_chosen_view() {
        let s = service();
        let mut w = PackagesWindow {
            query: "robust".into(),
            view_filter: Some("Econ".into()),
            ..PackagesWindow::default()
        };
        w.refresh_results(&s);
        assert_eq!(found(&w), vec!["sandwich"]);
        w.view_filter = Some("Other".into());
        w.refresh_results(&s);
        assert!(w.found.is_empty(), "sandwich is not in the Other view");
        w.view_filter = None;
        w.refresh_results(&s);
        assert_eq!(found(&w), vec!["sandwich"]);
    }

    #[test]
    fn picture_presets_open_and_filter_a_view() {
        let mut w = PackagesWindow::default();
        w.preset_view(Some("Econ"));
        assert_eq!(
            (w.view_open.as_deref(), w.view_filter.as_deref()),
            (Some("Econ"), Some("Econ"))
        );
        w.preset_view(None);
        assert_eq!((w.view_open, w.view_filter), (None, None));
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;
    use rcraft_rpkg::index::sample;

    fn pkg(name: &str, imports: &str, linking: &str) -> Package {
        Package {
            name: name.into(),
            version: "0.1.0".into(),
            title: "Made up".into(),
            imports: imports.into(),
            linking_to: linking.into(),
            ..Package::default()
        }
    }

    fn source_confirm(
        w: &PackagesWindow,
    ) -> (
        &Provider,
        &[Step],
        &[String],
        &Result<String, String>,
        &'static str,
    ) {
        match &w.confirm {
            Some(Confirm::Source {
                provider,
                deps,
                problems,
                code,
                menu,
                ..
            }) => (provider, deps.as_slice(), problems.as_slice(), code, *menu),
            _ => panic!("no source confirmation"),
        }
    }

    #[test]
    fn a_github_package_installs_its_cran_needs_first_and_needs_the_tick() {
        let s = PackageIndexService::in_memory(sample());
        let mut w = PackagesWindow::default();
        let gh = parse_github("me/madeup@v1").unwrap_or(Provider::Cran);
        w.begin_source_install(&s, gh, &pkg("madeup", "sandwich, stats", "Rcpp"));
        let (provider, deps, problems, code, menu) = source_confirm(&w);
        assert!(!provider.reviewed());
        let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["Rcpp", "sandwich"], "stats comes with R");
        assert!(problems.is_empty(), "{problems:?}");
        let code = code.as_ref().map(String::as_str).unwrap_or("");
        assert!(
            code.contains("install.packages(c(\"Rcpp\", \"sandwich\")"),
            "{code}"
        );
        assert!(
            code.contains("https://github.com/me/madeup/archive/v1.tar.gz"),
            "{code}"
        );
        assert_eq!(menu, "Packages > Install from GitHub");
        assert!(!w.source_understood, "the tick starts off");
    }

    #[test]
    fn a_need_that_is_not_on_cran_stops_the_install_with_a_reason() {
        let s = PackageIndexService::in_memory(sample());
        let mut w = PackagesWindow::default();
        w.begin_source_install(
            &s,
            Provider::Local("C:/x_1.0.tar.gz".into()),
            &pkg("x", "limma", ""),
        );
        let (_, _, problems, _, menu) = source_confirm(&w);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("\"limma\", which is not on CRAN")),
            "{problems:?}"
        );
        assert_eq!(menu, "Packages > Install from a file");
        // with no CRAN list at all, the needs cannot be installed either
        let none = PackageIndexService::new(
            None,
            None,
            std::sync::Arc::new(rcraft_rpkg::refresh::Offline),
            rcraft_engine::package_index::unix_now,
        );
        w.begin_source_install(
            &none,
            Provider::Local("C:/x_1.0.tar.gz".into()),
            &pkg("x", "cli", ""),
        );
        let (_, _, problems, _, _) = source_confirm(&w);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("Download the CRAN package list")),
            "{problems:?}"
        );
    }

    #[test]
    fn bioconductor_and_universe_packages_leave_their_needs_to_r() {
        let s = PackageIndexService::in_memory(sample());
        let mut w = PackagesWindow::default();
        w.begin_source_install(&s, Provider::Bioconductor, &pkg("limma", "", ""));
        let (provider, deps, problems, code, menu) = source_confirm(&w);
        assert!(provider.reviewed());
        assert!(deps.is_empty() && problems.is_empty());
        assert!(code
            .as_ref()
            .is_ok_and(|c| c.contains("BiocManager::install(c(\"limma\")")));
        assert_eq!(menu, "Packages > Install from Bioconductor");
        w.begin_source_install(
            &s,
            Provider::Universe("ropensci".into()),
            &pkg("rtweet", "", ""),
        );
        let (provider, _, _, code, _) = source_confirm(&w);
        assert!(!provider.reviewed());
        assert!(code
            .as_ref()
            .is_ok_and(|c| c.contains("https://ropensci.r-universe.dev")));
    }

    #[test]
    fn a_dcf_printed_by_r_becomes_a_package() {
        let p = package_from_dcf("Package: x\nVersion: 1.0\nTitle: X\nImports: cli\n");
        assert_eq!(
            p.map(|p| (p.name, p.imports)),
            Ok(("x".to_string(), "cli".to_string()))
        );
        assert!(package_from_dcf("").is_err());
    }

    /// Serves one GitHub DESCRIPTION.
    struct OneDescription;

    impl rcraft_rpkg::refresh::Fetcher for OneDescription {
        fn get(
            &self,
            url: &str,
            _max: u64,
            _p: &mut dyn FnMut(u64, Option<u64>) -> rcraft_rpkg::refresh::Control,
        ) -> rcraft_rpkg::Result<Vec<u8>> {
            if url == "https://raw.githubusercontent.com/me/madeup/HEAD/DESCRIPTION" {
                Ok(b"Package: madeup\nVersion: 0.2\nTitle: Made Up\nImports: zoo\n".to_vec())
            } else {
                Err(rcraft_rpkg::Error::Fetch(format!("{url}: 404")))
            }
        }
    }

    #[test]
    fn an_rc_github_install_looks_the_repository_up_and_then_shows_the_list() {
        let s = PackageIndexService::in_memory(sample());
        let mut w = PackagesWindow::default();
        w.set_lookups(SourceLookups::new(std::sync::Arc::new(OneDescription)));
        w.start_source(RcSource::GitHub, "me/madeup".into());
        assert_eq!(w.tab, Tab::Sources);
        let started = std::time::Instant::now();
        while w.lookups.is_busy() {
            w.lookups.poll();
            assert!(started.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        // with no R in this test, the list is made as soon as the lookup is in
        w.apply_pending_source(&s, false);
        let (provider, deps, _, code, _) = source_confirm(&w);
        assert!(matches!(provider, Provider::GitHub { repo, .. } if repo == "madeup"));
        assert_eq!(
            deps.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            vec!["zoo"]
        );
        assert!(code
            .as_ref()
            .is_ok_and(|c| c.contains("archive/HEAD.tar.gz")));
        // only once
        w.confirm = None;
        w.apply_pending_source(&s, false);
        assert!(w.confirm.is_none());
    }
}

#[cfg(test)]
mod source_needs_tests {
    use super::*;
    use rcraft_rpkg::index::sample;

    #[test]
    fn needs_already_installed_at_an_accepted_version_are_left_alone() {
        let s = PackageIndexService::in_memory(sample());
        let mut w = PackagesWindow::default();
        w.r.installed = vec![Installed {
            name: "sandwich".into(),
            version: "2.0".into(),
            library: "C:/lib".into(),
            priority: String::new(),
            repository: "CRAN".into(),
        }];
        let p = Package {
            name: "madeup".into(),
            version: "0.1".into(),
            imports: "sandwich, Rcpp (>= 1.0)".into(),
            ..Package::default()
        };
        w.begin_source_install(&s, Provider::Local("C:/m.tar.gz".into()), &p);
        let Some(Confirm::Source { deps, .. }) = &w.confirm else {
            panic!("no confirmation");
        };
        let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["Rcpp"], "sandwich 2.0 is there and accepted");
        // a version that is too old is installed again
        let strict = Package {
            imports: "sandwich (>= 3.0)".into(),
            ..p
        };
        w.begin_source_install(&s, Provider::Local("C:/m.tar.gz".into()), &strict);
        let Some(Confirm::Source { deps, .. }) = &w.confirm else {
            panic!("no confirmation");
        };
        assert_eq!(
            deps.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            vec!["sandwich"]
        );
    }
}
