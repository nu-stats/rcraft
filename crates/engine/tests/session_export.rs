//! P7.4: the session saved as an R script, a Quarto document and an R
//! Markdown document, against a real R (skipped with a note when none is
//! found). Commands are run in one R session (typed, from a menu, one that
//! fails); each saved file is then run in a fresh R session, the documents
//! chunk by chunk, and must leave the same objects. Rendering the documents
//! is left to Quarto or rmarkdown, which these tests do not need.

mod pinned;

use std::time::{Duration, Instant};

use rcraft_engine::{DocKind, Engine, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};

/// Base R only.
const PACKAGES: &[&str] = &[];

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    let mut e = Engine::start(SessionConfig::new(install)).ok()?;
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

#[test]
fn a_saved_session_runs_again_to_the_same_objects() {
    let Some(mut a) = engine() else { return };
    let dir = std::env::temp_dir().join(format!("rcraft-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(std::fs::create_dir_all(&dir).is_ok());

    assert_eq!(
        run(
            &mut a,
            Source::Typed,
            "set.seed(7); d <- data.frame(x = rnorm(30), g = rep(c('a', 'b', 'c'), 10))"
        ),
        Outcome::Ok
    );
    assert_eq!(
        run(
            &mut a,
            Source::Menu("Statistics > Models > Linear model".into()),
            "model <- stats::lm(x ~ g, data = d)\nprint(summary(model))"
        ),
        Outcome::Ok
    );
    assert_eq!(
        run(&mut a, Source::Typed, "stop('not kept')"),
        Outcome::Error
    );
    assert_eq!(
        run(
            &mut a,
            Source::Typed,
            "# a comment with > in it\nd$y <- d$x^2\nlabel <- \"```\""
        ),
        Outcome::Ok
    );
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
