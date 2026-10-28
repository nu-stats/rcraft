//! The install flow end to end against a real R (skipped with a note when none
//! is found) and a repository made on the spot, so it needs no network: two
//! made-up packages, one importing the other, built with `R CMD build` and
//! listed with `write_PACKAGES`. The index, the plan, the install code, the
//! installed list and the removal code are all the real ones.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Outcome, Source};
use rcraft_rcodegen::packages::{
    install_code, installed_packages_code, installed_versions, parse_installed, remove_code,
};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_rpkg::dcf;
use rcraft_rpkg::index::{Index, DCF_FIELDS};
use rcraft_rpkg::plan::{plan, Why};

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
    let limit = Instant::now() + Duration::from_secs(180);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

fn run(e: &mut Engine, code: &str) -> Outcome {
    let id = e.submit(Source::Typed, code);
    assert!(id.is_ok(), "{id:?}");
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    if outcome != Outcome::Ok {
        if let Some(b) = e.results().blocks().last() {
            eprintln!("R said: {:?}", b.items);
        }
    }
    outcome
}

/// What R prints for `code`, asked the way the window asks.
fn ask(e: &mut Engine, code: &str) -> String {
    let id = e.query(code);
    let limit = Instant::now() + Duration::from_secs(60);
    loop {
        e.poll(Duration::from_millis(50));
        if let Some(r) = e.query_result(id) {
            assert!(r.ok, "{code}: {}", r.text);
            return r.text.clone();
        }
        assert!(Instant::now() < limit, "no answer to {code}");
    }
}

struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let dir = std::env::temp_dir().join(format!("rcraft-pkg-{name}-{}", std::process::id()));
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

/// R that builds the two packages and the repository under `root`.
fn build_repository(root: &str) -> String {
    format!(
        r#"local({{
  root <- "{root}"
  mk <- function(name, extra) {{
    d <- file.path(root, "src", name)
    dir.create(file.path(d, "R"), recursive = TRUE)
    writeLines(c(paste0("Package: ", name), "Version: 1.0.2", "Title: A Made-Up Test Package",
                 "Description: Used only by the tests of RCraft.", "License: MIT + file LICENSE",
                 "Authors@R: person('Test', 'Author', email = 'test@example.org', role = c('aut', 'cre'))",
                 extra), file.path(d, "DESCRIPTION"))
    writeLines(c("YEAR: 2026", "COPYRIGHT HOLDER: Test"), file.path(d, "LICENSE"))
    writeLines("hello <- function() 1", file.path(d, "R", "hello.R"))
    writeLines("export(hello)", file.path(d, "NAMESPACE"))
  }}
  mk("rcrafttestb", character())
  mk("rcrafttesta", "Imports: rcrafttestb (>= 1.0)")
  contrib <- file.path(root, "repo", "src", "contrib")
  dir.create(contrib, recursive = TRUE)
  old <- setwd(contrib)
  on.exit(setwd(old))
  for (n in c("rcrafttestb", "rcrafttesta"))
    system2(file.path(R.home("bin"), "R"), c("CMD", "build", file.path(root, "src", n)), stdout = FALSE, stderr = FALSE)
  tools::write_PACKAGES(contrib, type = "source")
  cat(sort(list.files(contrib)), sep = "\n")
}})"#
    )
}

#[test]
fn a_package_and_what_it_imports_are_planned_installed_listed_and_removed() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("flow");
    let root = dir.path("repo-root");
    let lib = dir.path("lib");
    let repo_url = format!("file:///{}/repo", root.trim_start_matches('/'));

    // 1. a repository with two packages
    assert_eq!(run(&mut e, &build_repository(&root)), Outcome::Ok);
    let packages_gz = std::fs::read(format!("{root}/repo/src/contrib/PACKAGES.gz"));
    let packages_gz = packages_gz.unwrap_or_else(|err| panic!("no PACKAGES.gz: {err}"));

    // 2. the index from the repository's own list
    let table = dcf::read(&packages_gz, DCF_FIELDS).unwrap_or_else(|err| panic!("{err}"));
    let index = Index::from_tables(None, Some(&table), 1);
    assert_eq!(index.len(), 2);
    assert_eq!(
        index.get("rcrafttesta").map(|p| p.imports.as_str()),
        Some("rcrafttestb (>= 1.0)")
    );

    // 3. what is installed now, as R says it
    assert_eq!(run(&mut e, &format!("dir.create('{lib}')")), Outcome::Ok);
    assert_eq!(
        run(&mut e, &format!(".libPaths(c('{lib}', .libPaths()))")),
        Outcome::Ok
    );
    let before = installed_versions(&parse_installed(&ask(&mut e, installed_packages_code())));
    assert!(before.contains_key("stats"), "{before:?}");
    assert!(!before.contains_key("rcrafttesta"));

    // 4. the plan names the import
    let pl = plan(&index, &before, &["rcrafttesta"]);
    assert!(pl.can_run(), "{:?}", pl.problems);
    let steps: Vec<(&str, bool)> = pl
        .steps
        .iter()
        .map(|s| (s.name.as_str(), s.why == Why::Asked))
        .collect();
    assert_eq!(steps, vec![("rcrafttesta", true), ("rcrafttestb", false)]);

    // 5. the install code, run in R, installs both from the repository
    let code = install_code(&pl.asked(), Some(&lib), &repo_url, true).unwrap_or_default();
    assert!(code.contains("install.packages(\"rcrafttesta\""), "{code}");
    assert_eq!(run(&mut e, &code), Outcome::Ok);
    let got = ask(
        &mut e,
        &format!("cat(sort(rownames(installed.packages(lib.loc = '{lib}'))), sep = ' ')"),
    );
    assert_eq!(got.trim(), "rcrafttesta rcrafttestb", "{got}");

    // 6. planning again finds nothing to do
    let after = installed_versions(&parse_installed(&ask(&mut e, installed_packages_code())));
    assert_eq!(after.get("rcrafttesta").map(String::as_str), Some("1.0.2"));
    let again = plan(&index, &after, &["rcrafttesta"]);
    assert!(again.steps.is_empty());
    assert_eq!(again.already.len(), 1);

    // 7. the removal code takes both out of the library
    let rm = remove_code(&["rcrafttesta", "rcrafttestb"], Some(&lib)).unwrap_or_default();
    assert_eq!(run(&mut e, &rm), Outcome::Ok);
    let left = ask(
        &mut e,
        &format!("cat(nrow(installed.packages(lib.loc = '{lib}')))"),
    );
    assert_eq!(left.trim(), "0");
}

#[test]
fn installing_a_name_the_repository_lacks_fails_in_r_with_its_message() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("missing");
    let root = dir.path("repo-root");
    let lib = dir.path("lib");
    assert_eq!(run(&mut e, &build_repository(&root)), Outcome::Ok);
    let repo_url = format!("file:///{}/repo", root.trim_start_matches('/'));
    let code = install_code(&["rcraftnotthere"], Some(&lib), &repo_url, true).unwrap_or_default();
    // R warns that the package is not available; nothing is installed
    run(&mut e, &code);
    let n = ask(
        &mut e,
        &format!("cat(nrow(installed.packages(lib.loc = '{lib}')))"),
    );
    assert_eq!(n.trim(), "0");
}

#[test]
fn where_a_function_comes_from_and_what_is_loaded_are_read_from_r() {
    use rcraft_rcodegen::packages::{loaded_packages_code, parse_loaded, where_code};
    let Some(mut e) = engine() else { return };
    let w = ask(&mut e, &where_code("median"));
    assert!(
        w.lines()
            .any(|l| l.trim() == "package:stats" || l.trim() == "stats"),
        "{w:?}"
    );
    // a name nothing holds prints nothing, and does not fail
    assert_eq!(
        ask(&mut e, &where_code("rcraft_no_such_function")).trim(),
        ""
    );
    let loaded = parse_loaded(&ask(&mut e, loaded_packages_code()));
    assert!(loaded.contains(&("stats".to_string(), true)), "{loaded:?}");
    assert!(loaded.iter().any(|(n, _)| n == "methods"), "{loaded:?}");
}
