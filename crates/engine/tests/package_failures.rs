//! P1A.7 against a real R: installs that fail on purpose, and what the
//! diagnosis says about R's own messages. The repository is built on the spot
//! (made-up packages), so no network is needed; the unreachable-mirror case
//! uses a host name that cannot exist.
//!
//! The missing-compiler case runs only where R cannot find `make` (a Windows
//! machine without Rtools, such as the one this was written on); elsewhere the
//! package would build and there is nothing to diagnose.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::failure::{diagnose, Cause, Os, Retry};
use rcraft_rcodegen::packages::{install_code, InstallFrom};
use rcraft_rhost::{find_installs, Env, SessionConfig};

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    Engine::start(SessionConfig::new(install)).ok()
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(300);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

/// Runs `code` and returns everything R printed for it, as the window reads it.
fn output_of(e: &mut Engine, code: &str) -> (Outcome, String) {
    let id = e.submit(Source::Typed, code);
    assert!(id.is_ok(), "{id:?}");
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let mut text = String::new();
    if let Some(b) = e.results().blocks().last() {
        for item in &b.items {
            match item {
                Item::Text { text: t, .. } => text.push_str(t),
                Item::Warning(w) => text.push_str(w),
                Item::Error { message, .. } => text.push_str(message),
                Item::Notice(n) => text.push_str(n),
                Item::Plot { .. } => {}
            }
            text.push('\n');
        }
    }
    (outcome, text)
}

struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let dir = std::env::temp_dir().join(format!("rcraft-fail-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(std::fs::create_dir_all(&dir).is_ok());
        Folder(dir)
    }

    fn path(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().replace('\\', "/")
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn this_os() -> Os {
    if cfg!(windows) {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::MacOs
    } else {
        Os::Linux
    }
}

/// R that builds the broken packages and a repository of them under `root`.
fn build_repository(root: &str) -> String {
    format!(
        r##"local({{
  root <- "{root}"
  mk <- function(name, desc = character(), files = list()) {{
    d <- file.path(root, "src", name); dir.create(file.path(d, "R"), recursive = TRUE)
    writeLines(c(paste0("Package: ", name), "Version: 1.0", "Title: Test", "Description: Test.",
                 "License: MIT + file LICENSE",
                 "Authors@R: person('T', 'A', email = 't@e.org', role = c('aut', 'cre'))", desc),
               file.path(d, "DESCRIPTION"))
    writeLines(c("YEAR: 2026", "COPYRIGHT HOLDER: T"), file.path(d, "LICENSE"))
    writeLines("f <- function() 1", file.path(d, "R", "f.R"))
    writeLines("export(f)", file.path(d, "NAMESPACE"))
    for (n in names(files)) {{
      p <- file.path(d, n); dir.create(dirname(p), showWarnings = FALSE, recursive = TRUE)
      writeLines(files[[n]], p)
    }}
  }}
  mk("pfcompiled", character(), list("src/a.c" = c("#include <R.h>", "void a(void) {{}}")))
  mk("pfnewr", "Depends: R (>= 99.0)")
  mk("pfdep", "Imports: pfnothere")
  contrib <- file.path(root, "repo", "src", "contrib"); dir.create(contrib, recursive = TRUE)
  old <- setwd(contrib); on.exit(setwd(old))
  for (n in c("pfcompiled", "pfnewr", "pfdep"))
    system2(file.path(R.home("bin"), "R"), c("CMD", "build", file.path(root, "src", n)),
            stdout = FALSE, stderr = FALSE)
  tools::write_PACKAGES(contrib, type = "source")
  cat(Sys.which("make"))
}})"##
    )
}

fn causes(text: &str, r_version: &str) -> Vec<Cause> {
    diagnose(text, this_os(), r_version)
        .into_iter()
        .map(|d| d.cause)
        .collect()
}

#[test]
fn r_s_own_messages_for_failed_installs_are_diagnosed() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("all");
    let root = dir.path("root");
    let lib = dir.path("lib");
    let repo = format!("file:///{}/repo", root.trim_start_matches('/'));
    let version = e
        .r_version()
        .split_whitespace()
        .skip_while(|w| *w != "version")
        .nth(1)
        .unwrap_or("4.4.3")
        .to_string();

    let (_, made) = output_of(&mut e, &build_repository(&root));
    let make = made.trim().to_string();

    let run = |e: &mut Engine, pkg: &str, repo: &str, from: InstallFrom| {
        let code = install_code(&[pkg], Some(&lib), repo, from).unwrap_or_default();
        output_of(e, &code).1
    };

    // a package that needs a newer R than this one
    let t = run(&mut e, "pfnewr", &repo, InstallFrom::Source);
    assert!(
        causes(&t, &version).contains(&Cause::NeedsNewerR("pfnewr".into(), "99.0".into())),
        "{t}"
    );
    let d = diagnose(&t, this_os(), &version);
    assert_eq!(d[0].retry, vec![Retry::OlderVersion("pfnewr".into())]);

    // a package whose dependency is not offered
    let t = run(&mut e, "pfdep", &repo, InstallFrom::Source);
    assert!(
        causes(&t, &version).contains(&Cause::DependencyUnavailable(
            "pfdep".into(),
            "pfnothere".into()
        )),
        "{t}"
    );

    // a name that is not there
    let t = run(&mut e, "pfabsent", &repo, InstallFrom::Source);
    assert_eq!(
        causes(&t, &version),
        vec![Cause::NotFound("pfabsent".into())],
        "{t}"
    );

    // a mirror that cannot be reached is blamed, not the package name
    let t = run(
        &mut e,
        "ggplot2",
        "https://rcraft-no-such-host.invalid",
        InstallFrom::Default,
    );
    assert_eq!(causes(&t, &version), vec![Cause::Network], "{t}");

    // a library folder that cannot be written: one whose parent is a file, so it cannot
    // be made (a folder in the system area would be writable on a runner with admin rights)
    let blocker = dir.path("blocker");
    assert!(std::fs::write(&blocker, "not a folder").is_ok());
    let code = install_code(
        &["pfcompiled"],
        Some(&format!("{blocker}/lib")),
        &repo,
        InstallFrom::Source,
    )
    .unwrap_or_default();
    let t = output_of(&mut e, &code).1;
    assert!(causes(&t, &version).contains(&Cause::NotWritable), "{t}");

    // a package with compiled code, where R has no `make`
    if make.is_empty() {
        let t = run(&mut e, "pfcompiled", &repo, InstallFrom::Source);
        let d = diagnose(&t, this_os(), &version);
        assert_eq!(
            d.first().map(|d| d.cause.clone()),
            Some(Cause::NoCompiler),
            "{t}"
        );
        if cfg!(windows) {
            let steps = d[0].steps.join("\n");
            assert!(steps.contains("Rtools"), "{steps}");
            assert!(d[0].retry.contains(&Retry::BinaryOnly));
        }
    } else {
        eprintln!("the compiler case was skipped: R found make at {make}");
    }
}

#[test]
fn a_good_install_is_diagnosed_as_nothing() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("good");
    let root = dir.path("root");
    let lib = dir.path("lib");
    let repo = format!("file:///{}/repo", root.trim_start_matches('/'));
    // the fixtures of the other test are not built here: a package that needs only R itself
    let build = format!(
        r##"local({{
  root <- "{root}"; d <- file.path(root, "src", "pfgood"); dir.create(file.path(d, "R"), recursive = TRUE)
  writeLines(c("Package: pfgood", "Version: 1.0", "Title: Test", "Description: Test.", "License: MIT + file LICENSE",
               "Authors@R: person('T', 'A', email = 't@e.org', role = c('aut', 'cre'))"), file.path(d, "DESCRIPTION"))
  writeLines(c("YEAR: 2026", "COPYRIGHT HOLDER: T"), file.path(d, "LICENSE"))
  writeLines("f <- function() 1", file.path(d, "R", "f.R")); writeLines("export(f)", file.path(d, "NAMESPACE"))
  contrib <- file.path(root, "repo", "src", "contrib"); dir.create(contrib, recursive = TRUE)
  old <- setwd(contrib); on.exit(setwd(old))
  system2(file.path(R.home("bin"), "R"), c("CMD", "build", d), stdout = FALSE, stderr = FALSE)
  tools::write_PACKAGES(contrib, type = "source")
}})"##
    );
    let _ = output_of(&mut e, &build);
    let code =
        install_code(&["pfgood"], Some(&lib), &repo, InstallFrom::Source).unwrap_or_default();
    let (outcome, text) = output_of(&mut e, &code);
    assert_eq!(outcome, Outcome::Ok);
    assert!(diagnose(&text, this_os(), "4.4.3").is_empty(), "{text}");
}
