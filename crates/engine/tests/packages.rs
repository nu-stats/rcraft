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
    InstallFrom,
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
    let code =
        install_code(&pl.asked(), Some(&lib), &repo_url, InstallFrom::Source).unwrap_or_default();
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
    let code = install_code(
        &["rcraftnotthere"],
        Some(&lib),
        &repo_url,
        InstallFrom::Source,
    )
    .unwrap_or_default();
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

/// R that builds `pfcur` 1.0 and `pfold` at 1.0 (put in the archive) and 2.0
/// (current) as a repository under `root`, with CRAN's layout.
fn build_archive_repository(root: &str) -> String {
    format!(
        r##"local({{
  root <- "{root}"
  mk <- function(name, version) {{
    d <- file.path(root, "src", paste0(name, "-", version), name); dir.create(file.path(d, "R"), recursive = TRUE)
    writeLines(c(paste0("Package: ", name), paste0("Version: ", version), "Title: Test", "Description: Test.",
                 "License: MIT + file LICENSE",
                 "Authors@R: person('T', 'A', email = 't@e.org', role = c('aut', 'cre'))"), file.path(d, "DESCRIPTION"))
    writeLines(c("YEAR: 2026", "COPYRIGHT HOLDER: T"), file.path(d, "LICENSE"))
    writeLines("f <- function() 1", file.path(d, "R", "f.R")); writeLines("export(f)", file.path(d, "NAMESPACE"))
    d
  }}
  contrib <- file.path(root, "repo", "src", "contrib"); dir.create(contrib, recursive = TRUE)
  archive <- file.path(contrib, "Archive", "pfold"); dir.create(archive, recursive = TRUE)
  build <- function(dir, to) {{
    old <- setwd(to); on.exit(setwd(old))
    system2(file.path(R.home("bin"), "R"), c("CMD", "build", dir), stdout = FALSE, stderr = FALSE)
  }}
  build(mk("pfold", "1.0"), archive)
  build(mk("pfold", "2.0"), contrib)
  build(mk("pfcur", "1.0"), contrib)
  tools::write_PACKAGES(contrib, type = "source")
  cat(sort(c(list.files(contrib), list.files(archive))), sep = " ")
}})"##
    )
}

#[test]
fn a_saved_list_installs_the_current_and_the_archived_versions_it_names() {
    use rcraft_rcodegen::packages::restore_code;
    use rcraft_rpkg::lockfile::{restore_plan, Action, Entry, Lock};
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("restore");
    let root = dir.path("root");
    let lib = dir.path("lib");
    let repo_url = format!("file:///{}/repo", root.trim_start_matches('/'));
    assert_eq!(run(&mut e, &build_archive_repository(&root)), Outcome::Ok);
    let made = e
        .results()
        .blocks()
        .last()
        .map(|b| format!("{:?}", b.items))
        .unwrap_or_default();
    assert!(
        made.contains("pfold_1.0.tar.gz") && made.contains("pfold_2.0.tar.gz"),
        "{made}"
    );

    // the index is the repository's own list: pfold is at 2.0 now
    let gz = std::fs::read(format!("{root}/repo/src/contrib/PACKAGES.gz"))
        .unwrap_or_else(|err| panic!("no PACKAGES.gz: {err}"));
    let table = dcf::read(&gz, DCF_FIELDS).unwrap_or_else(|err| panic!("{err}"));
    let index = Index::from_tables(None, Some(&table), 1);

    // a list made earlier: pfold as it was (1.0), pfcur as it is
    let entry = |n: &str, v: &str| Entry {
        name: n.into(),
        version: v.into(),
        priority: String::new(),
        repository: "CRAN".into(),
    };
    let lock = Lock::from_entries(
        &[
            entry("pfold", "1.0"),
            entry("pfcur", "1.0"),
            entry("stats", "4.4.3"),
        ],
        None,
        "4.4.3",
        "https://cloud.r-project.org",
        "2026-10-28",
    );
    // it survives being written to a file and read back
    let lock = Lock::parse(&lock.to_text()).unwrap_or_else(|err| panic!("{err}"));

    assert_eq!(run(&mut e, &format!("dir.create('{lib}')")), Outcome::Ok);
    assert_eq!(
        run(&mut e, &format!(".libPaths(c('{lib}', .libPaths()))")),
        Outcome::Ok
    );
    let before = installed_versions(&parse_installed(&ask(&mut e, installed_packages_code())));
    let r = restore_plan(Some(&index), &before, &lock, "4.4.3");
    assert_eq!(r.current(), vec!["pfcur"], "{r:?}");
    assert_eq!(r.older(), vec![("pfold", "1.0")], "{r:?}");

    let code = restore_code(&r.current(), &r.older(), Some(&lib), &repo_url)
        .unwrap_or_else(|err| panic!("{err}"));
    // the repository here has no binary folder, so the current ones come from source
    let code = code.replace("repos = \"file:", "type = \"source\", repos = \"file:");
    // R installs an archive file from a web address (https) but not from a file:// one, so
    // here the local copy is named by its path
    let code = code.replace("install.packages(\"file:///", "install.packages(\"");
    assert_eq!(run(&mut e, &code), Outcome::Ok);
    let got = ask(
        &mut e,
        &format!(
            "cat(paste(rownames(installed.packages(lib.loc = '{lib}')), installed.packages(lib.loc = '{lib}')[, 'Version']), sep = ' ')"
        ),
    );
    assert_eq!(got.trim(), "pfcur 1.0 pfold 1.0", "{got}");

    // planning again finds nothing left to do
    let after = installed_versions(&parse_installed(&ask(&mut e, installed_packages_code())));
    let again = restore_plan(Some(&index), &after, &lock, "4.4.3");
    assert!(!again.can_run(), "{again:?}");
    assert!(again
        .steps
        .iter()
        .filter(|s| s.name.starts_with("pf"))
        .all(|s| s.action == Action::AlreadyThere));
}
