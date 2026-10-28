//! The File menu's save dialogs end to end against a real R (skipped with a
//! note when none is found): build each dialog's code from its spec, run it, and
//! read the file back in R to see that it holds what was saved. Formats that
//! need a package (writexl, haven, arrow) are skipped when R lacks it.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rcraft_catalog::save::{
    SAVE_ARROW, SAVE_EXCEL, SAVE_R, SAVE_STATISTICS, SAVE_TEXT, SAVE_WORKSPACE,
};
use rcraft_engine::{Engine, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Spec, Style, VarKind};
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
    let limit = Instant::now() + Duration::from_secs(60);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

fn run(e: &mut Engine, source: Source, code: &str) -> Outcome {
    let id = e.submit(source, code);
    assert!(id.is_ok(), "{id:?}");
    settle(e);
    e.history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error)
}

/// A folder of its own for one test's files, removed when it goes out of scope.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let dir = std::env::temp_dir().join(format!("rcraft-save-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(std::fs::create_dir_all(&dir).is_ok());
        Folder(dir)
    }

    /// A path in the folder, with forward slashes as R and the dialogs take it.
    fn file(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().replace('\\', "/")
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const MAKE: &str = "d <- data.frame(id = 1:4, \
     name = c('a,b', 'say \"hi\"', NA, 'caf\\u00e9'), x = c(1.5, NA, 3, 4), \
     stringsAsFactors = FALSE)";

/// Makes `d` and runs `spec` on it with these values; the outcome of the run.
fn save(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    flags: &[(&str, bool)],
) -> Option<Outcome> {
    assert_eq!(run(e, Source::Typed, MAKE), Outcome::Ok);
    e.watch("d");
    settle(e);
    let columns: Vec<ColumnInfo> = e
        .table("d")?
        .columns
        .iter()
        .map(|c| ColumnInfo {
            name: c.name.clone(),
            kind: VarKind::of(&c.data),
        })
        .collect();
    let ctx = Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns,
    };
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, on) in flags {
        values.set_flag(id, *on);
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    Some(run(e, Source::Menu(spec.menu.into()), &code))
}

/// Whether R has the package; a note is printed when it does not.
fn have(e: &mut Engine, package: &str) -> bool {
    let code = format!("stopifnot(requireNamespace(\"{package}\", quietly = TRUE))");
    let found = run(e, Source::Typed, &code) == Outcome::Ok;
    if !found {
        eprintln!("skipped: the R package {package} is not installed");
    }
    found
}

fn check(e: &mut Engine, code: &str) {
    assert_eq!(run(e, Source::Typed, code), Outcome::Ok, "{code}");
}

#[test]
fn text_files_read_back_as_saved() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("text");
    let cases = [
        ("comma", "NA", false, "a.csv", ",", "NA"),
        ("tab", "NA", false, "a.tsv", "\\t", "NA"),
        ("semicolon", "empty", false, "b.csv", ";", ""),
        ("comma", "NA", true, "c.csv", ",", "NA"),
    ];
    for (sep, na, rows, name, r_sep, r_na) in cases {
        let file = dir.file(name);
        let out = save(
            &mut e,
            &SAVE_TEXT,
            &[("file", &file), ("sep", sep), ("na", na)],
            &[("rownames", rows)],
        );
        assert_eq!(out, Some(Outcome::Ok), "{sep} {na} {rows}");
        let row_names = if rows { ", row.names = 1" } else { "" };
        check(
            &mut e,
            &format!(
                "back <- read.table('{file}', sep = '{r_sep}', header = TRUE, \
                 na.strings = '{r_na}', quote = '\"', comment.char = '', \
                 fileEncoding = 'UTF-8', stringsAsFactors = FALSE{row_names}); \
                 stopifnot(all.equal(as.list(back), as.list(d), check.attributes = FALSE))"
            ),
        );
    }
}

#[test]
fn a_file_is_not_written_over_unless_asked() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("guard");
    let file = dir.file("keep.csv");
    assert_eq!(
        save(&mut e, &SAVE_TEXT, &[("file", &file)], &[]),
        Some(Outcome::Ok)
    );
    let first = std::fs::read(dir.0.join("keep.csv")).unwrap_or_default();
    assert!(!first.is_empty());
    // different data, same file, no Replace: R stops and the file is untouched
    assert_eq!(
        run(&mut e, Source::Typed, "d <- data.frame(z = 9)"),
        Outcome::Ok
    );
    let refused = run(
        &mut e,
        Source::Menu(SAVE_TEXT.menu.into()),
        &format!(
            "if (file.exists('{file}')) stop('the file already exists'); \
             write.csv(d, '{file}')"
        ),
    );
    assert_eq!(refused, Outcome::Error);
    assert_eq!(
        std::fs::read(dir.0.join("keep.csv")).unwrap_or_default(),
        first
    );
    // the dialog's own code does the same, and with Replace it writes
    let again = save(&mut e, &SAVE_TEXT, &[("file", &file)], &[]);
    assert_eq!(again, Some(Outcome::Error));
    assert_eq!(
        std::fs::read(dir.0.join("keep.csv")).unwrap_or_default(),
        first
    );
    let replaced = save(&mut e, &SAVE_TEXT, &[("file", &file)], &[("replace", true)]);
    assert_eq!(replaced, Some(Outcome::Ok));
}

#[test]
fn r_data_files_hold_the_data_frame_exactly() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("r");
    let rds = dir.file("d.rds");
    assert_eq!(
        save(&mut e, &SAVE_R, &[("file", &rds), ("format", "rds")], &[]),
        Some(Outcome::Ok)
    );
    check(
        &mut e,
        &format!("stopifnot(identical(readRDS('{rds}'), d))"),
    );
    let rdata = dir.file("d.RData");
    assert_eq!(
        save(
            &mut e,
            &SAVE_R,
            &[("file", &rdata), ("format", "rdata")],
            &[]
        ),
        Some(Outcome::Ok)
    );
    check(
        &mut e,
        &format!(
            "env <- new.env(); loaded <- load('{rdata}', envir = env); \
             stopifnot(identical(loaded, 'd'), identical(env$d, d))"
        ),
    );
}

#[test]
fn the_workspace_comes_back_whole() {
    let Some(mut e) = engine() else { return };
    let dir = Folder::new("workspace");
    let file = dir.file("all.RData");
    assert_eq!(
        run(&mut e, Source::Typed, "other <- list(a = 1, b = 'two')"),
        Outcome::Ok
    );
    assert_eq!(
        save(&mut e, &SAVE_WORKSPACE, &[("file", &file)], &[]),
        Some(Outcome::Ok)
    );
    // clear the workspace, then bring it back from the file
    check(
        &mut e,
        &format!(
            "rm(list = ls()); loaded <- load('{file}'); \
             stopifnot(setequal(loaded, c('d', 'other')), \
             identical(other, list(a = 1, b = 'two')), nrow(d) == 4)"
        ),
    );
}

#[test]
fn stata_spss_and_sas_files_keep_values_and_labels() {
    let Some(mut e) = engine() else { return };
    if !have(&mut e, "haven") {
        return;
    }
    let dir = Folder::new("haven");
    for (format, reader) in [
        ("dta", "haven::read_dta"),
        ("sav", "haven::read_sav"),
        ("zsav", "haven::read_sav"),
        ("xpt", "haven::read_xpt"),
    ] {
        let file = dir.file(&format!("d.{format}"));
        let out = save(
            &mut e,
            &SAVE_STATISTICS,
            &[("file", &file), ("format", format)],
            &[],
        );
        assert_eq!(out, Some(Outcome::Ok), "{format}");
        check(
            &mut e,
            &format!(
                "back <- {reader}('{file}'); \
                 stopifnot(identical(names(back), names(d)), \
                 all.equal(as.numeric(back$x), d$x), \
                 identical(as.vector(back$name), ifelse(is.na(d$name), '', d$name)))"
            ),
        );
    }
}

#[test]
fn excel_workbooks_read_back() {
    let Some(mut e) = engine() else { return };
    if !have(&mut e, "writexl") || !have(&mut e, "readxl") {
        return;
    }
    let dir = Folder::new("excel");
    let file = dir.file("d.xlsx");
    let out = save(
        &mut e,
        &SAVE_EXCEL,
        &[("file", &file), ("sheet", "Q1 data")],
        &[],
    );
    assert_eq!(out, Some(Outcome::Ok));
    check(
        &mut e,
        &format!(
            "stopifnot(identical(readxl::excel_sheets('{file}'), 'Q1 data')); \
             back <- as.data.frame(readxl::read_excel('{file}')); \
             stopifnot(all.equal(back$x, d$x), identical(back$name, d$name))"
        ),
    );
}

#[test]
fn parquet_and_feather_files_read_back() {
    let Some(mut e) = engine() else { return };
    if !have(&mut e, "arrow") {
        return;
    }
    let dir = Folder::new("arrow");
    for (format, reader) in [
        ("parquet", "arrow::read_parquet"),
        ("feather", "arrow::read_feather"),
    ] {
        let file = dir.file(&format!("d.{format}"));
        let out = save(
            &mut e,
            &SAVE_ARROW,
            &[("file", &file), ("format", format)],
            &[],
        );
        assert_eq!(out, Some(Outcome::Ok), "{format}");
        check(
            &mut e,
            &format!(
                "back <- as.data.frame({reader}('{file}')); \
                 stopifnot(all.equal(back, d, check.attributes = FALSE))"
            ),
        );
    }
}
