//! Example data (P8.3) against a real R (skipped with a note when none is
//! found): each example's code, run as the menu runs it, makes a data frame
//! equal to the one in R's `datasets` package, with the rows and variables
//! its description says.

mod pinned;

use std::time::{Duration, Instant};

use rcraft_catalog::examples::{EXAMPLES, EXAMPLE_DATA};
use rcraft_engine::{Engine, Outcome, Source};
use rcraft_rcodegen::dialog::{Context, Style};
use rcraft_rhost::{find_installs, Env, SessionConfig};

/// R's own datasets package only.
const PACKAGES: &[&str] = &["datasets"];

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
    let limit = Instant::now() + Duration::from_secs(60);
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

/// The rows and variables a description gives: `(32 rows, 11 variables)`.
fn size_of(label: &str) -> Option<(usize, usize)> {
    let inside = label.rsplit_once('(')?.1.trim_end_matches(')');
    let (rows, vars) = inside.split_once(", ")?;
    Some((
        rows.strip_suffix(" rows")?.parse().ok()?,
        vars.strip_suffix(" variables")?.parse().ok()?,
    ))
}

#[test]
fn every_example_is_rs_own_data_frame_of_the_size_described() {
    let Some(mut e) = engine() else { return };
    let ctx = Context {
        dataset: "(examples)".into(),
        frames: Vec::new(),
        columns: Vec::new(),
    };
    for ex in EXAMPLES {
        let (rows, vars) = size_of(ex.label).unwrap_or_else(|| panic!("{}", ex.label));
        let mut values = EXAMPLE_DATA.defaults(&ctx);
        values.set_text("result", ex.value);
        let code = EXAMPLE_DATA
            .generate(&ctx, &values, Style::Base)
            .unwrap_or_default();
        assert_eq!(
            run(&mut e, Source::Menu(EXAMPLE_DATA.menu.into()), &code),
            Outcome::Ok,
            "{code}"
        );
        let check = format!(
            "stopifnot(is.data.frame({n}), nrow({n}) == {rows}, ncol({n}) == {vars}, \
             isTRUE(all.equal({n}, as.data.frame(get('{n}', envir = asNamespace('datasets'))))))",
            n = ex.value
        );
        assert_eq!(
            run(&mut e, Source::Typed, &check),
            Outcome::Ok,
            "{}",
            ex.value
        );
    }
}
