//! Packages a dialog's code needs (the tidyverse style needs dplyr): asking R
//! whether they can be loaded, and offering to install the ones that cannot.
//! The install goes into RCraft's own library and runs as an ordinary command,
//! so it shows in Results and in the log.

use eframe::egui::{self, RichText};
use rcraft_engine::{Engine, QueryId};
use rcraft_rcodegen::{missing_packages_code, r_string};

use crate::look::Palette;

/// RCraft's own package library, as a path R understands.
pub(crate) fn library_dir() -> Option<String> {
    rcraft_rhost::library::user_library_dir(&rcraft_rhost::Env::from_process())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}

/// The CRAN mirror the user chose in the Packages window (the default when
/// none was chosen).
pub(crate) fn saved_mirror() -> String {
    rcraft_engine::package_index::saved_mirror(&rcraft_rhost::Env::from_process())
}

/// R code that installs `packages` into RCraft's library, from the saved mirror.
pub(crate) fn install_code(packages: &[&str], library: Option<&str>) -> String {
    install_code_from(packages, library, &saved_mirror())
}

/// R code that installs `packages` into `library` from `mirror`.
pub(crate) fn install_code_from(packages: &[&str], library: Option<&str>, mirror: &str) -> String {
    rcraft_rcodegen::packages::install_code(
        packages,
        library,
        mirror,
        rcraft_rcodegen::packages::InstallFrom::Default,
    )
    .unwrap_or_else(|why| format!("stop({})", r_string(&why)))
}

#[derive(Default)]
enum Check {
    #[default]
    NotAsked,
    Waiting(QueryId),
    /// The packages that cannot be loaded.
    Got(Vec<String>),
}

/// What is known about the packages a dialog needs right now.
#[derive(Default)]
pub struct PackageCheck {
    /// The packages the answer is about.
    needed: Vec<&'static str>,
    check: Check,
    /// An install was started from here; look again when R is free.
    installing: bool,
    seen_busy: bool,
}

impl PackageCheck {
    /// Forgets what was learned (a dialog opened afresh).
    pub fn reset(&mut self) {
        *self = PackageCheck::default();
    }

    /// Asks R, and takes in its answer. Call it every frame while the dialog is
    /// open, with the packages its present choices need (none for base R).
    pub fn service(&mut self, engine: &mut Engine, needed: &[&'static str]) {
        if self.installing {
            if engine.is_busy() {
                self.seen_busy = true;
            } else if self.seen_busy {
                self.installing = false;
                self.seen_busy = false;
                self.check = Check::NotAsked;
            }
        }
        if self.needed != needed {
            self.needed = needed.to_vec();
            self.check = Check::NotAsked;
        }
        if needed.is_empty() {
            return;
        }
        match &self.check {
            Check::NotAsked => {
                self.check = Check::Waiting(engine.query(&missing_packages_code(needed)));
            }
            Check::Waiting(id) => {
                if let Some(r) = engine.query_result(*id) {
                    // an answer that did not come means the packages are not known to be there
                    self.check = Check::Got(if r.ok {
                        r.text
                            .lines()
                            .map(str::trim)
                            .filter(|l| !l.is_empty())
                            .map(str::to_string)
                            .collect()
                    } else {
                        needed.iter().map(|p| p.to_string()).collect()
                    });
                }
            }
            Check::Got(_) => {}
        }
    }

    /// Whether R is still answering.
    pub fn is_waiting(&self) -> bool {
        matches!(self.check, Check::Waiting(_)) || self.installing
    }

    /// Whether the code can run: nothing is needed, or R says it is all there.
    pub fn ready(&self, needed: &[&str]) -> bool {
        needed.is_empty()
            || (self.needed == needed && matches!(&self.check, Check::Got(m) if m.is_empty()))
    }

    /// Tells the user which packages are missing and offers to install them.
    /// Draws nothing when the code can run. Returns the install code when the
    /// user asks for it.
    pub fn panel(&mut self, ui: &mut egui::Ui, pal: &Palette, needed: &[&str]) -> Option<String> {
        if self.ready(needed) {
            return None;
        }
        let missing: Vec<String> = match &self.check {
            Check::Got(m) if self.needed == needed => m.clone(),
            _ => {
                let what = needed.join(", ");
                let text = if self.installing {
                    format!("Installing {what}…")
                } else {
                    format!("Checking for {what}…")
                };
                ui.label(RichText::new(text).color(pal.dim));
                return None;
            }
        };
        let list: Vec<&str> = missing.iter().map(String::as_str).collect();
        let names = list.join(", ");
        ui.label(
            RichText::new(format!(
                "This dialog needs the R package {names}, which is not installed."
            ))
            .color(pal.warning),
        );
        let code = install_code(&list, library_dir().as_deref());
        ui.label(
            RichText::new(
                "RCraft can install it for you. It is downloaded from CRAN (with the packages it \
                 needs) into RCraft's own library, not into any other app's R:",
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
        let mut ask = None;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.installing,
                    egui::Button::new(format!("Install {names}")),
                )
                .clicked()
            {
                self.installing = true;
                self.seen_busy = false;
                ask = Some(code.clone());
            }
            if ui.button("Check again").clicked() {
                self.check = Check::NotAsked;
            }
        });
        ask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installing_goes_to_rcrafts_own_library() {
        let code = install_code_from(
            &["haven"],
            Some("C:/Users/me/AppData/Local/RCraft/R-library"),
            "https://cloud.r-project.org",
        );
        assert!(code.contains("lib = \"C:/Users/me/AppData/Local/RCraft/R-library\""));
        assert!(code.contains("https://cloud.r-project.org"));
        assert!(code.starts_with("dir.create("));
        assert!(code.contains("install.packages(\"haven\","));
        assert!(
            !install_code_from(&["haven"], None, "https://cloud.r-project.org").contains("lib =")
        );
    }

    #[test]
    fn several_packages_install_together() {
        let code = install_code_from(&["dplyr", "tidyr"], None, "https://cloud.r-project.org");
        assert!(
            code.contains("install.packages(c(\"dplyr\", \"tidyr\"),"),
            "{code}"
        );
    }

    #[test]
    fn nothing_needed_is_always_ready_and_an_unanswered_check_is_not() {
        let check = PackageCheck::default();
        assert!(check.ready(&[]));
        assert!(!check.ready(&["dplyr"]));
    }

    /// Asks a real R about `needed` until it answers (skipped without R).
    fn ask_r(needed: &[&'static str]) -> Option<PackageCheck> {
        use rcraft_rhost::{find_installs, Env, SessionConfig};
        use std::time::{Duration, Instant};
        let install = find_installs(&Env::from_process())
            .into_iter()
            .find(|i| i.is_supported())?;
        let mut engine = Engine::start(SessionConfig::new(install)).ok()?;
        let mut check = PackageCheck::default();
        let limit = Instant::now() + Duration::from_secs(60);
        while !check.ready(needed) && !matches!(check.check, Check::Got(_)) {
            engine.poll(Duration::from_millis(50));
            check.service(&mut engine, needed);
            assert!(Instant::now() < limit, "R did not answer");
        }
        Some(check)
    }

    #[test]
    fn r_says_which_packages_are_missing() {
        let Some(check) = ask_r(&["stats", "definitelyNotAPackage"]) else {
            eprintln!("skipped: no supported R on this machine");
            return;
        };
        assert!(!check.ready(&["stats", "definitelyNotAPackage"]));
        assert!(matches!(&check.check, Check::Got(m) if m == &["definitelyNotAPackage"]));
        let Some(check) = ask_r(&["stats", "utils"]) else {
            return;
        };
        assert!(check.ready(&["stats", "utils"]));
    }

    #[test]
    fn an_answer_is_only_good_for_the_packages_it_was_asked_about() {
        let check = PackageCheck {
            needed: vec!["dplyr"],
            check: Check::Got(Vec::new()),
            ..PackageCheck::default()
        };
        assert!(check.ready(&["dplyr"]));
        assert!(!check.ready(&["dplyr", "tidyr"]));
        let missing = PackageCheck {
            needed: vec!["dplyr"],
            check: Check::Got(vec!["dplyr".into()]),
            ..PackageCheck::default()
        };
        assert!(!missing.ready(&["dplyr"]));
    }
}
