//! Installing from a local file or folder and from a GitHub-style archive,
//! against a real R (skipped with a note when none is found). The packages are
//! made up and built on the spot; the "GitHub" archive is laid out the way
//! GitHub serves one (a single top folder named after the repository and the
//! reference) and fetched from a file address, so no network is needed.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Outcome, Source};
use rcraft_rcodegen::sources::{describe_local_code, install_archive_code, install_local_code};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_rpkg::dcf;
use rcraft_rpkg::index::{Index, DB_COLUMNS};

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

/// What R prints for `code` (`Err` with R's message when it fails).
fn ask(e: &mut Engine, code: &str) -> Result<String, String> {
    let id = e.query(code);
    let limit = Instant::now() + Duration::from_secs(120);
    loop {
        e.poll(Duration::from_millis(50));
        if let Some(r) = e.query_result(id) {
            return if r.ok {
                Ok(r.text.clone())
            } else {
                Err(r.text.clone())
            };
        }
        assert!(Instant::now() < limit, "no answer");
    }
}

struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let dir = std::env::temp_dir().join(format!("rcraft-src-{name}-{}", std::process::id()));
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

/// R that makes a source folder `rcraftlocal`, its `.tar.gz`, on Windows its
/// binary `.zip`, and a GitHub-style archive `rcraftgh-main.tar.gz`, under `root`.
fn make_packages(root: &str) -> String {
    format!(
        r##"local({{
  root <- "{root}"
  mk <- function(dir, name) {{
    dir.create(file.path(dir, "R"), recursive = TRUE)
    writeLines(c(paste0("Package: ", name), "Version: 1.0", "Title: A Made-Up Package",
                 "Description: Used only by the tests of RCraft.", "License: MIT + file LICENSE",
                 "Imports: stats", "Authors@R: person('T', 'A', email = 't@e.org', role = c('aut', 'cre'))"),
               file.path(dir, "DESCRIPTION"))
    writeLines(c("YEAR: 2026", "COPYRIGHT HOLDER: T"), file.path(dir, "LICENSE"))
    writeLines("f <- function() 1", file.path(dir, "R", "f.R"))
    writeLines("export(f)", file.path(dir, "NAMESPACE"))
  }}
  mk(file.path(root, "rcraftlocal"), "rcraftlocal")
  old <- setwd(root); on.exit(setwd(old))
  r <- file.path(R.home("bin"), "R")
  system2(r, c("CMD", "build", "rcraftlocal"), stdout = FALSE, stderr = FALSE)
  # the binary is built through a library of its own, so the source folder is left as it is
  if (.Platform$OS.type == "windows")
    {{ dir.create("binlib"); system2(r, c("CMD", "INSTALL", "--build", "--library=binlib", "rcraftlocal_1.0.tar.gz"), stdout = FALSE, stderr = FALSE) }}
  mk(file.path(root, "rcraftgh-main"), "rcraftgh")
  utils::tar("rcraftgh-main.tar.gz", files = "rcraftgh-main", compression = "gzip")
  cat(sort(list.files(root)), sep = " ")
}})"##
    )
}

#[test]
fn local_files_and_folders_are_described_and_installed() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("local");
    let root = dir.path("root");
    assert!(std::fs::create_dir_all(&root).is_ok());
    assert_eq!(run(&mut e, &make_packages(&root)), Outcome::Ok);

    let mut kinds = vec![
        format!("{root}/rcraftlocal_1.0.tar.gz"),
        format!("{root}/rcraftlocal"),
    ];
    // building a Windows binary needs the zip tool from Rtools; where it is missing, no .zip
    let zip = format!("{root}/rcraftlocal_1.0.zip");
    if std::path::Path::new(&zip).exists() {
        kinds.push(zip);
    } else {
        eprintln!("the .zip case was skipped: R could not build a binary here");
    }
    for (i, path) in kinds.iter().enumerate() {
        // what is inside, read without installing anything
        let text = ask(&mut e, &describe_local_code(path).unwrap_or_default())
            .unwrap_or_else(|err| panic!("{path}: {err}"));
        let table = dcf::table(&text, DB_COLUMNS).unwrap_or_else(|err| panic!("{err}"));
        let ix = Index::from_tables(Some(&table), None, 0);
        let p = ix
            .get("rcraftlocal")
            .unwrap_or_else(|| panic!("{path}: {text}"));
        assert_eq!(p.version, "1.0", "{path}");
        assert_eq!(p.title, "A Made-Up Package", "{path}");
        assert_eq!(p.imports, "stats", "{path}");

        // installed into a library of its own
        let lib = dir.path(&format!("lib{i}"));
        let code = install_local_code(path, &[], Some(&lib), "https://cloud.r-project.org")
            .unwrap_or_default();
        assert_eq!(run(&mut e, &code), Outcome::Ok, "{path}");
        let got = ask(
            &mut e,
            &format!("cat(rownames(installed.packages(lib.loc = '{lib}')))"),
        );
        assert_eq!(
            got.map(|g| g.trim().to_string()),
            Ok("rcraftlocal".to_string()),
            "{path}"
        );
    }

    // a folder that is not there, and a file that is not a package, say so
    let missing = ask(
        &mut e,
        &describe_local_code(&dir.path("nope")).unwrap_or_default(),
    );
    assert!(missing.is_err_and(|m| m.contains("no folder")));
    let junk = dir.path("junk.tar.gz");
    assert!(std::fs::write(&junk, b"not an archive").is_ok());
    assert!(ask(&mut e, &describe_local_code(&junk).unwrap_or_default()).is_err());
}

#[test]
fn a_github_style_archive_is_fetched_and_built() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("gh");
    let root = dir.path("root");
    assert!(std::fs::create_dir_all(&root).is_ok());
    assert_eq!(run(&mut e, &make_packages(&root)), Outcome::Ok);
    let lib = dir.path("lib");
    let url = format!(
        "file:///{}/rcraftgh-main.tar.gz",
        root.trim_start_matches('/')
    );
    let code = install_archive_code(&url, &[], Some(&lib), "https://cloud.r-project.org")
        .unwrap_or_default();
    assert_eq!(run(&mut e, &code), Outcome::Ok, "{code}");
    let got = ask(
        &mut e,
        &format!("cat(rownames(installed.packages(lib.loc = '{lib}')))"),
    );
    // the archive's folder is named rcraftgh-main; the package is still rcraftgh
    assert_eq!(
        got.map(|g| g.trim().to_string()),
        Ok("rcraftgh".to_string())
    );
}
