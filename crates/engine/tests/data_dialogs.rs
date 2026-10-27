//! Data menu dialogs (P3.5) end to end against a real R (skipped with a note
//! when none is found): make a small data frame, bring it over, build each
//! dialog's base-R code from its spec and the table's columns, run it, and
//! compare the table R gives back.

use std::time::{Duration, Instant};

use rcraft_catalog::data::{COMPUTE, FILTER, RENAME, SORT};
use rcraft_engine::{Engine, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Spec, Style, VarKind};
use rcraft_rdata::display::cell_text;
use rcraft_rdata::Table;
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

fn fetch(e: &mut Engine, name: &str) -> Option<Table> {
    e.watch(name);
    settle(e);
    let table = e.table(name).cloned();
    e.unwatch(name);
    table
}

fn context(name: &str, table: &Table) -> Context {
    Context {
        dataset: name.into(),
        columns: table
            .columns
            .iter()
            .map(|c| ColumnInfo {
                name: c.name.clone(),
                kind: VarKind::of(&c.data),
            })
            .collect(),
    }
}

fn column(table: &Table, name: &str) -> Vec<String> {
    let Some(col) = table.columns.iter().find(|c| c.name == name) else {
        return vec![format!("no column {name}")];
    };
    (0..table.nrow).map(|r| cell_text(&col.data, r)).collect()
}

const MAKE: &str = "d <- data.frame(id = 1:5, g = factor(c(\"b\", \"a\", \"b\", \"a\", NA)), \
                    x = c(3, 1, 2, NA, 5))";

/// Builds `d` again, runs `spec` with these values on it, and returns the
/// table named `result`.
fn apply(e: &mut Engine, spec: &Spec, set: &[(&str, &str)], result: &str) -> Option<Table> {
    assert_eq!(run(e, Source::Typed, MAKE), Outcome::Ok);
    let table = fetch(e, "d")?;
    let ctx = context("d", &table);
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    assert_eq!(
        run(e, Source::Menu(spec.menu.into()), &code),
        Outcome::Ok,
        "{code}"
    );
    fetch(e, result)
}

#[test]
fn base_r_data_dialogs_change_the_data_as_shown() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    // sort by g, then x largest first: missing values go last at each level
    let sorted = apply(
        &mut e,
        &SORT,
        &[("by1", "g"), ("by2", "x"), ("order2", "down")],
        "d",
    );
    let sorted = sorted.as_ref().map(|t| column(t, "id"));
    assert_eq!(
        sorted,
        Some(
            vec!["2", "4", "1", "3", "5"]
                .into_iter()
                .map(String::from)
                .collect()
        )
    );

    // filter: rows where the condition is NA are left out both ways
    let kept = apply(&mut e, &FILTER, &[("condition", "x > 1")], "d");
    assert_eq!(
        kept.map(|t| column(&t, "id")),
        Some(vec!["1".to_string(), "3".into(), "5".into()])
    );
    let dropped = apply(
        &mut e,
        &FILTER,
        &[
            ("condition", "x > 1"),
            ("keep", "drop"),
            ("result", "small"),
        ],
        "small",
    );
    assert_eq!(
        dropped.map(|t| column(&t, "id")),
        Some(vec!["2".to_string()])
    );

    // new variable into a copy: the original keeps its three columns
    let copy = apply(
        &mut e,
        &COMPUTE,
        &[("name", "x two"), ("value", "x * 2"), ("result", "d2")],
        "d2",
    );
    assert_eq!(
        copy.map(|t| column(&t, "x two")),
        Some(
            vec!["6", "2", "4", "NA", "10"]
                .into_iter()
                .map(String::from)
                .collect()
        )
    );
    assert_eq!(fetch(&mut e, "d").map(|t| t.columns.len()), Some(3));

    // rename in place
    let renamed = apply(&mut e, &RENAME, &[("old", "g"), ("new", "group")], "d");
    let names: Option<Vec<String>> =
        renamed.map(|t| t.columns.iter().map(|c| c.name.clone()).collect());
    assert_eq!(
        names,
        Some(vec!["id".to_string(), "group".into(), "x".into()])
    );
}
