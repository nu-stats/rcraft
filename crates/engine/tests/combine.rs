//! The Data menu's combine and reshape dialogs end to end against a real R
//! (skipped with a note when none is found): build each dialog's code from its
//! spec and the table's columns, run it, and compare what R gives back.

use std::time::{Duration, Instant};

use rcraft_catalog::combine::{
    AGGREGATE, APPEND, DROP_MISSING, DUPLICATES, JOIN, LONG_TO_WIDE, SAMPLE, WIDE_TO_LONG,
};
use rcraft_engine::{Engine, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Frame, Spec, Style, VarKind};
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

fn columns_of(table: &Table) -> Vec<ColumnInfo> {
    table
        .columns
        .iter()
        .map(|c| ColumnInfo {
            name: c.name.clone(),
            kind: VarKind::of(&c.data),
        })
        .collect()
}

fn context(name: &str, table: &Table, frames: Vec<Frame>) -> Context {
    Context {
        dataset: name.into(),
        frames,
        columns: columns_of(table),
    }
}

fn column(table: &Table, name: &str) -> Vec<String> {
    let Some(col) = table.columns.iter().find(|c| c.name == name) else {
        return vec![format!("no column {name}")];
    };
    (0..table.nrow).map(|r| cell_text(&col.data, r)).collect()
}

fn names(table: &Table) -> Vec<String> {
    table.columns.iter().map(|c| c.name.clone()).collect()
}

fn list(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Builds the data frames in `make`, runs `spec` on `d` with these values and
/// returns the table it leaves in `result`.
fn apply(
    e: &mut Engine,
    make: &str,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
) -> Option<Table> {
    assert_eq!(run(e, Source::Typed, make), Outcome::Ok, "{make}");
    let table = fetch(e, "d");
    assert!(table.is_some(), "R has no data frame d");
    // the other data frames, as the window offers them: `e`, when R has one
    let frames: Vec<Frame> = fetch(e, "e")
        .map(|t| Frame {
            name: "e".into(),
            columns: columns_of(&t),
        })
        .into_iter()
        .collect();
    let ctx = context("d", &table?, frames);
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, items) in lists {
        values.set_list(id, list(items));
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    assert_eq!(
        run(e, Source::Menu(spec.menu.into()), &code),
        Outcome::Ok,
        "{code}"
    );
    let out = values.text("result").trim().to_string();
    fetch(e, &out)
}

const TWO: &str = "d <- data.frame(k = c(1, 2, 3, 4), x = c('a', 'b', 'c', 'd')); \
     e <- data.frame(k = c(2, 3, 5), y = c(20, 30, 50))";

#[test]
fn join_keeps_the_rows_asked_for() {
    let Some(mut e) = engine() else { return };
    let keys: &[(&str, &[&str])] = &[("keys", &["k"])];
    let kept = |e: &mut Engine, keep: &str| {
        apply(
            e,
            TWO,
            &JOIN,
            &[("other", "e"), ("keep", keep), ("result", "j")],
            keys,
        )
    };
    let inner = kept(&mut e, "inner");
    assert_eq!(inner.as_ref().map(column_k), Some(list(&["2", "3"])));
    assert_eq!(inner.map(|t| column(&t, "y")), Some(list(&["20", "30"])));
    let left = kept(&mut e, "left");
    assert_eq!(
        left.map(|t| column(&t, "y")),
        Some(list(&["NA", "20", "30", "NA"]))
    );
    let full = kept(&mut e, "full");
    assert_eq!(
        full.map(|t| column(&t, "k")),
        Some(list(&["1", "2", "3", "4", "5"]))
    );
}

fn column_k(t: &Table) -> Vec<String> {
    column(t, "k")
}

#[test]
fn append_fills_what_one_side_lacks() {
    let Some(mut e) = engine() else { return };
    let make = "d <- data.frame(a = c(1, 2), b = c('x', 'y')); \
                e <- data.frame(b = c('z'), c = c(TRUE))";
    let t = apply(
        &mut e,
        make,
        &APPEND,
        &[("other", "e"), ("result", "a")],
        &[],
    );
    assert_eq!(t.as_ref().map(names), Some(list(&["a", "b", "c"])));
    assert_eq!(
        t.as_ref().map(|t| column(t, "a")),
        Some(list(&["1", "2", "NA"]))
    );
    assert_eq!(t.map(|t| column(&t, "b")), Some(list(&["x", "y", "z"])));
}

#[test]
fn aggregate_summarises_each_group() {
    let Some(mut e) = engine() else { return };
    let make = "d <- data.frame(g = c('a', 'a', 'b', 'b', 'b'), \
                x = c(1, 3, 10, NA, 20))";
    let lists: &[(&str, &[&str])] = &[("groups", &["g"]), ("values", &["x"])];
    let mean = apply(&mut e, make, &AGGREGATE, &[("result", "s")], lists);
    assert_eq!(
        mean.as_ref().map(|t| column(t, "g")),
        Some(list(&["a", "b"]))
    );
    assert_eq!(mean.map(|t| column(&t, "x")), Some(list(&["2", "15"])));
    let count = apply(
        &mut e,
        make,
        &AGGREGATE,
        &[("how", "count"), ("result", "s")],
        lists,
    );
    assert_eq!(count.map(|t| column(&t, "x")), Some(list(&["2", "2"])));
}

#[test]
fn reshape_goes_long_and_back() {
    let Some(mut e) = engine() else { return };
    let make = "d <- data.frame(who = c('p', 'q'), inc1 = c(1, 2), inc2 = c(10, 20))";
    let long = apply(
        &mut e,
        make,
        &WIDE_TO_LONG,
        &[("result", "l")],
        &[("stack", &["inc1", "inc2"]), ("ids", &["who"])],
    );
    assert_eq!(
        long.as_ref().map(names),
        Some(list(&["who", "which", "value"]))
    );
    assert_eq!(
        long.as_ref().map(|t| column(t, "which")),
        Some(list(&["inc1", "inc1", "inc2", "inc2"]))
    );
    assert_eq!(
        long.map(|t| column(&t, "value")),
        Some(list(&["1", "2", "10", "20"]))
    );

    let long_data = "d <- data.frame(who = c('p', 'q', 'p', 'q'), yr = c(1, 1, 2, 2), \
                     inc = c(1, 2, 10, 20))";
    let wide = apply(
        &mut e,
        long_data,
        &LONG_TO_WIDE,
        &[("when", "yr"), ("result", "w")],
        &[("ids", &["who"]), ("values", &["inc"])],
    );
    assert_eq!(
        wide.as_ref().map(names),
        Some(list(&["who", "inc_1", "inc_2"]))
    );
    assert_eq!(
        wide.as_ref().map(|t| column(t, "inc_1")),
        Some(list(&["1", "2"]))
    );
    assert_eq!(wide.map(|t| column(&t, "inc_2")), Some(list(&["10", "20"])));
}

#[test]
fn reshape_without_ids_numbers_the_cases() {
    let Some(mut e) = engine() else { return };
    let make = "d <- data.frame(a = c(1, 2), b = c(3, 4))";
    let long = apply(
        &mut e,
        make,
        &WIDE_TO_LONG,
        &[("result", "l")],
        &[("stack", &["a", "b"])],
    );
    assert_eq!(
        long.as_ref().map(names),
        Some(list(&["which", "value", "id"]))
    );
    assert_eq!(
        long.map(|t| column(&t, "id")),
        Some(list(&["1", "2", "1", "2"]))
    );
}

const DUPS: &str = "d <- data.frame(k = c(1, 2, 2, 3, 3, 3), v = c('a', 'b', 'b', 'c', 'd', 'c'))";

#[test]
fn duplicates_drop_keep_and_mark() {
    let Some(mut e) = engine() else { return };
    let whole = |e: &mut Engine, action: &str| {
        apply(
            e,
            DUPS,
            &DUPLICATES,
            &[("action", action), ("result", "r")],
            &[],
        )
    };
    assert_eq!(
        whole(&mut e, "drop").map(|t| column(&t, "v")),
        Some(list(&["a", "b", "c", "d"]))
    );
    assert_eq!(
        whole(&mut e, "only").map(|t| column(&t, "v")),
        Some(list(&["b", "b", "c", "c"]))
    );
    let by_k = apply(
        &mut e,
        DUPS,
        &DUPLICATES,
        &[("action", "mark"), ("result", "r")],
        &[("vars", &["k"])],
    );
    assert_eq!(
        by_k.map(|t| column(&t, "duplicated")),
        Some(list(&["FALSE", "TRUE", "TRUE", "TRUE", "TRUE", "TRUE"]))
    );
}

#[test]
fn sample_has_the_size_and_repeats_with_a_seed() {
    let Some(mut e) = engine() else { return };
    let make = "d <- data.frame(n = 1:10)";
    let sized = |e: &mut Engine, how: &str, amount: &str| {
        apply(
            e,
            make,
            &SAMPLE,
            &[
                ("how", how),
                ("amount", amount),
                ("seed", "5"),
                ("result", "s"),
            ],
            &[],
        )
    };
    assert_eq!(sized(&mut e, "count", "3").map(|t| t.nrow), Some(3));
    assert_eq!(sized(&mut e, "count", "50").map(|t| t.nrow), Some(10));
    let first = sized(&mut e, "percent", "40").map(|t| column(&t, "n"));
    assert_eq!(first.as_ref().map(Vec::len), Some(4));
    assert_eq!(
        sized(&mut e, "percent", "40").map(|t| column(&t, "n")),
        first
    );
}

#[test]
fn drop_missing_drops_or_keeps_rows() {
    let Some(mut e) = engine() else { return };
    let make = "d <- data.frame(a = c(1, NA, 3, 4), b = c('x', 'y', NA, 'w'))";
    let dropped = apply(&mut e, make, &DROP_MISSING, &[("result", "r")], &[]);
    assert_eq!(dropped.map(|t| column(&t, "a")), Some(list(&["1", "4"])));
    let only_b = apply(
        &mut e,
        make,
        &DROP_MISSING,
        &[("result", "r")],
        &[("vars", &["b"])],
    );
    assert_eq!(
        only_b.map(|t| column(&t, "a")),
        Some(list(&["1", "NA", "4"]))
    );
    let kept = apply(
        &mut e,
        make,
        &DROP_MISSING,
        &[("which", "keep"), ("result", "r")],
        &[],
    );
    assert_eq!(kept.map(|t| column(&t, "a")), Some(list(&["NA", "3"])));
}
