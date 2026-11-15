//! P7.4: the session saved as an R script, a Quarto document and an R
//! Markdown document, against a real R (skipped with a note when none is
//! found). Commands are run in one R session (typed, from a menu, one that
//! fails); each saved file is then run in a fresh R session, the documents
//! chunk by chunk, and must leave the same objects. Where Quarto is found
//! (`QUARTO_PATH`, or `quarto` on the PATH) the documents are also rendered,
//! the .qmd by Quarto as a project and the .Rmd by rmarkdown with Quarto's
//! pandoc; without it that test is skipped with a note.

mod pinned;

use std::time::{Duration, Instant};

use rcraft_engine::{DocKind, Engine, Outcome, Source};
use rcraft_rhost::{find_installs, Env, RInstall, SessionConfig};

/// Base R only.
const PACKAGES: &[&str] = &[];

fn install() -> Option<RInstall> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    if install.is_none() {
        eprintln!("skipped: no supported R on this machine");
    }
    install
}

fn engine() -> Option<Engine> {
    let mut e = Engine::start(SessionConfig::new(install()?)).ok()?;
    pinned::record(&mut e, PACKAGES);
    Some(e)
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(120);
    while e.is_working() {
        e.poll(Duration::from_millis(20));
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

fn run(e: &mut Engine, source: Source, code: &str) -> Outcome {
    assert!(e.submit(source, code).is_ok());
    settle(e);
    e.history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error)
}

/// What R prints for `code`, asked out of sight (not part of the session).
fn ask(e: &mut Engine, code: &str) -> String {
    let id = e.query(code);
    settle(e);
    e.query_result(id)
        .map(|r| r.text.clone())
        .unwrap_or_default()
}

/// The R in a document's chunks, in order.
fn chunks(doc: &str) -> String {
    let mut out = String::new();
    let mut fence: Option<String> = None;
    for line in doc.lines() {
        match &fence {
            Some(f) if line == f => fence = None,
            Some(_) => {
                out.push_str(line);
                out.push('\n');
            }
            None if line.starts_with("```") && line.ends_with("{r}") => {
                fence = Some(line.trim_end_matches("{r}").to_string());
            }
            None => {}
        }
    }
    out
}

fn slashes(p: &std::path::Path) -> String {
    p.display().to_string().replace('\\', "/")
}

/// Typed, from a menu, one that fails and code holding backticks.
fn a_session(a: &mut Engine) {
    assert_eq!(
        run(
            a,
            Source::Typed,
            "set.seed(7); d <- data.frame(x = rnorm(30), g = rep(c('a', 'b', 'c'), 10))"
        ),
        Outcome::Ok
    );
    assert_eq!(
        run(
            a,
            Source::Menu("Statistics > Models > Linear model".into()),
            "model <- stats::lm(x ~ g, data = d)\nprint(summary(model))"
        ),
        Outcome::Ok
    );
    assert_eq!(run(a, Source::Typed, "stop('not kept')"), Outcome::Error);
    assert_eq!(
        run(
            a,
            Source::Typed,
            "# a comment with > in it\nd$y <- d$x^2\nlabel <- \"```\""
        ),
        Outcome::Ok
    );
}

#[test]
fn a_saved_session_runs_again_to_the_same_objects() {
    let Some(mut a) = engine() else { return };
    let dir = std::env::temp_dir().join(format!("rcraft-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(std::fs::create_dir_all(&dir).is_ok());

    a_session(&mut a);
    let want = slashes(&dir.join("want.rds"));
    ask(
        &mut a,
        &format!("saveRDS(list(d = d, b = coef(model), label = label), '{want}')"),
    );

    let files = [
        ("session.R", a.script_text(), false),
        (
            "session.qmd",
            a.document_text(DocKind::Quarto, "A session"),
            true,
        ),
        (
            "session.Rmd",
            a.document_text(DocKind::RMarkdown, "A session"),
            true,
        ),
    ];
    for (name, text, document) in &files {
        assert!(!text.contains("not kept"), "{name}: {text}");
        let path = dir.join(name);
        assert!(std::fs::write(&path, text).is_ok());
        let Some(mut b) = engine() else { return };
        let code = if *document {
            chunks(text)
        } else {
            format!("source('{}')", slashes(&path))
        };
        assert_eq!(
            run(&mut b, Source::Typed, &code),
            Outcome::Ok,
            "{name}\n{code}"
        );
        let same = ask(
            &mut b,
            &format!(
                "cat(identical(readRDS('{want}'), list(d = d, b = coef(model), label = label)))"
            ),
        );
        assert_eq!(same.trim(), "TRUE", "{name}");
    }
    // the documents' chunks are the script's commands
    assert_eq!(chunks(&files[1].1), chunks(&files[2].1));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Quarto, from `QUARTO_PATH` or the PATH, if it runs.
fn quarto() -> Option<std::path::PathBuf> {
    let name = if cfg!(windows) {
        "quarto.exe"
    } else {
        "quarto"
    };
    let on_path = || {
        std::env::split_paths(&std::env::var_os("PATH")?)
            .map(|dir| dir.join(name))
            .find(|p| p.is_file())
    };
    let path = match std::env::var_os("QUARTO_PATH") {
        Some(p) => std::path::PathBuf::from(p),
        None => on_path()?,
    };
    let runs = std::process::Command::new(&path)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    runs.then_some(path)
}

/// The pandoc that came with Quarto (in its `bin/tools`, by platform).
fn pandoc_beside(quarto: &std::path::Path) -> Option<std::path::PathBuf> {
    let found = std::process::Command::new(quarto)
        .args(["pandoc", "--version"])
        .output()
        .ok()?;
    if !found.status.success() {
        return None;
    }
    // quarto may be a link (macOS); Windows' canonical form starts \\?\,
    // which R does not read as a folder
    let real = std::fs::canonicalize(quarto).ok()?;
    let real = match real.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(plain) => std::path::PathBuf::from(plain),
        None => real,
    };
    let bin = real.parent()?.join("tools");
    let name = if cfg!(windows) {
        "pandoc.exe"
    } else {
        "pandoc"
    };
    let mut folders = vec![bin];
    while let Some(dir) = folders.pop() {
        if dir.join(name).is_file() {
            return Some(dir);
        }
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            if entry.path().is_dir() {
                folders.push(entry.path());
            }
        }
    }
    None
}

#[test]
fn a_saved_session_renders_with_quarto_and_rmarkdown() {
    let Some(quarto) = quarto() else {
        eprintln!("skipped: no Quarto (set QUARTO_PATH or put quarto on the PATH)");
        return;
    };
    let Some(install) = install() else { return };
    let Some(mut a) = engine() else { return };
    // Quarto runs R chunks through knitr, and rmarkdown renders the .Rmd
    let knits = ask(
        &mut a,
        "cat(all(vapply(c('knitr', 'rmarkdown'), requireNamespace, TRUE, quietly = TRUE)))",
    );
    if knits.trim() != "TRUE" {
        eprintln!("skipped: knitr and rmarkdown are not both installed in this R");
        return;
    }
    a_session(&mut a);
    let dir = std::env::temp_dir().join(format!("rcraft-render-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(std::fs::create_dir_all(&dir).is_ok());

    // as File > Save session as Quarto project writes it
    assert!(std::fs::write(dir.join("_quarto.yml"), "project:\n  type: default\n").is_ok());
    let qmd = a.document_text(DocKind::Quarto, "RCraft session: render");
    assert!(std::fs::write(dir.join("session.qmd"), &qmd).is_ok());
    let rendered = std::process::Command::new(&quarto)
        .arg("render")
        .current_dir(&dir)
        .env("QUARTO_R", &install.rscript)
        .output();
    let Ok(rendered) = rendered else {
        panic!("quarto did not start");
    };
    assert!(
        rendered.status.success(),
        "quarto render: {}",
        String::from_utf8_lossy(&rendered.stderr)
    );
    let html = std::fs::read_to_string(dir.join("session.html")).unwrap_or_default();
    // the menu entry as text, the command's printed output, the backticks kept
    assert!(
        html.contains("Statistics &gt; Models &gt; Linear model"),
        "{html}"
    );
    assert!(html.contains("Residual standard error"), "{html}");
    assert!(!html.contains("not kept"), "{html}");

    let Some(pandoc) = pandoc_beside(&quarto) else {
        panic!("Quarto runs but its pandoc was not found");
    };
    // the .Rmd renders to the same name, so Quarto's page goes first
    assert!(std::fs::remove_file(dir.join("session.html")).is_ok());
    let rmd = dir.join("session.Rmd");
    assert!(std::fs::write(&rmd, a.document_text(DocKind::RMarkdown, "A session")).is_ok());
    let rendered = std::process::Command::new(&install.rscript)
        .args([
            "-e",
            &format!("rmarkdown::render('{}', quiet = TRUE)", slashes(&rmd)),
        ])
        .env("RSTUDIO_PANDOC", &pandoc)
        .output();
    let Ok(rendered) = rendered else {
        panic!("Rscript did not start");
    };
    assert!(
        rendered.status.success(),
        "rmarkdown::render: {}",
        String::from_utf8_lossy(&rendered.stderr)
    );
    let html = std::fs::read_to_string(dir.join("session.html")).unwrap_or_default();
    assert!(
        html.contains("Statistics &gt; Models &gt; Linear model"),
        "{html}"
    );
    assert!(html.contains("Residual standard error"), "{html}");
    assert!(!html.contains("not kept"), "{html}");
    let _ = std::fs::remove_dir_all(&dir);
}
