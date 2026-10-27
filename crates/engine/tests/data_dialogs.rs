//! Data menu dialogs (P3.5) end to end against a real R (skipped with a note
//! when none is found): make a small data frame, bring it over, build each
//! dialog's base-R code from its spec and the table's columns, run it, and
//! compare the table R gives back.

use std::time::{Duration, Instant};

use rcraft_catalog::data::{
    COMPUTE, FILTER, LABELLED_TO_FACTOR, RENAME, SAVE_OBJECT, SORT, VALUE_LABELS, VARIABLE_LABEL,
};
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
    apply_to(e, MAKE, spec, set, result)
}

fn apply_to(
    e: &mut Engine,
    make: &str,
    spec: &Spec,
    set: &[(&str, &str)],
    result: &str,
) -> Option<Table> {
    assert_eq!(run(e, Source::Typed, make), Outcome::Ok);
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

const MAKE_LABELLED: &str = "d <- data.frame(id = 1:4, s = c(1, 2, 1, 9), t = c(\"a\", \"b\", \"a\", \"c\")); \
                             attr(d$s, \"labels\") <- c(Low = 1, High = 2); attr(d$s, \"label\") <- \"Score\"";

fn attr_text(table: &Table, column: &str, key: &str) -> Option<String> {
    let col = table.columns.iter().find(|c| c.name == column)?;
    let a = col.attrs.iter().find(|a| a.key == key)?;
    Some(format!("{:?}", a.value))
}

#[test]
fn label_dialogs_change_the_labels_as_shown() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    // a variable label, set and then removed
    let t = apply_to(
        &mut e,
        MAKE_LABELLED,
        &VARIABLE_LABEL,
        &[("var", "id"), ("label", "Identifier")],
        "d",
    );
    assert!(t.is_some_and(|t| t
        .columns
        .iter()
        .any(|c| c.name == "id" && c.label() == Some("Identifier"))));
    let t = apply_to(
        &mut e,
        MAKE_LABELLED,
        &VARIABLE_LABEL,
        &[("var", "s"), ("label", "")],
        "d",
    );
    assert!(t.is_some_and(|t| t.columns.iter().all(|c| c.label().is_none())));

    // value labels replace the old ones
    let t = apply_to(
        &mut e,
        MAKE_LABELLED,
        &VALUE_LABELS,
        &[("var", "s"), ("labels", "9 = Odd; 1 = One")],
        "d",
    );
    let labels = t.and_then(|t| {
        t.columns
            .iter()
            .find(|c| c.name == "s")
            .map(|c| c.value_labels())
    });
    assert_eq!(
        labels,
        Some(vec![(9.0, "Odd".to_string()), (1.0, "One".to_string())])
    );
    let t = apply_to(
        &mut e,
        MAKE_LABELLED,
        &VALUE_LABELS,
        &[("var", "t"), ("labels", "a = Alpha")],
        "d",
    );
    assert!(t.is_some_and(|t| attr_text(&t, "t", "labels").is_some()));

    // labelled to factor: unlabelled values become missing, the label stays
    let t = apply_to(
        &mut e,
        MAKE_LABELLED,
        &LABELLED_TO_FACTOR,
        &[("var", "s"), ("result", "f")],
        "f",
    );
    assert_eq!(
        t.as_ref().map(|t| column(t, "s")),
        Some(vec![
            "Low".to_string(),
            "High".into(),
            "Low".into(),
            "NA".into()
        ])
    );
    assert!(t.is_some_and(|t| t
        .columns
        .iter()
        .any(|c| c.name == "s" && c.label() == Some("Score"))));

    // a variable without labels is refused by R, and nothing changes
    assert_eq!(run(&mut e, Source::Typed, MAKE_LABELLED), Outcome::Ok);
    let table = fetch(&mut e, "d");
    let ctx = context("d", &table.expect("table"));
    let mut values = LABELLED_TO_FACTOR.defaults(&ctx);
    values.set_text("var", "id");
    let code = LABELLED_TO_FACTOR
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    assert_eq!(
        run(&mut e, Source::Menu(LABELLED_TO_FACTOR.menu.into()), &code),
        Outcome::Error
    );
    assert_eq!(fetch(&mut e, "d").map(|t| t.columns.len()), Some(3));
}

#[test]
fn save_object_writes_an_rds_file_and_will_not_replace_one_unasked() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    assert_eq!(run(&mut e, Source::Typed, MAKE_LABELLED), Outcome::Ok);
    let table = fetch(&mut e, "d").expect("table");
    let ctx = context("d", &table);
    let path = std::env::temp_dir().join(format!("rcraft-save-{}.rds", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let path_text = path.to_string_lossy().into_owned();

    let mut values = SAVE_OBJECT.defaults(&ctx);
    values.set_text("file", &path_text);
    let code = SAVE_OBJECT
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let menu = Source::Menu(SAVE_OBJECT.menu.into());
    assert_eq!(run(&mut e, menu.clone(), &code), Outcome::Ok, "{code}");
    assert!(path.is_file());

    // saved as it is: the labels come back with it
    let back = format!(
        "back <- readRDS({}); stopifnot(identical(back, d))",
        rcraft_rcodegen::r_string(&path_text.replace('\\', "/"))
    );
    assert_eq!(run(&mut e, Source::Typed, &back), Outcome::Ok);

    // a second save stops unless the box is ticked
    assert_eq!(run(&mut e, menu.clone(), &code), Outcome::Error);
    values.set_flag("replace", true);
    let again = SAVE_OBJECT
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    assert_eq!(run(&mut e, menu, &again), Outcome::Ok);
    let _ = std::fs::remove_file(&path);
}

/// Every cell of `table` as shown, column by column, with the column's name.
fn shown(table: &Table) -> Vec<(String, Vec<String>)> {
    table
        .columns
        .iter()
        .map(|c| (c.name.clone(), column(table, &c.name)))
        .collect()
}

/// As [`apply`], in the given code style.
fn apply_in(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    result: &str,
    style: Style,
) -> Option<Table> {
    assert_eq!(run(e, Source::Typed, MAKE), Outcome::Ok);
    let table = fetch(e, "d")?;
    let ctx = context("d", &table);
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    let code = spec.generate(&ctx, &values, style).unwrap_or_default();
    assert_eq!(
        run(e, Source::Menu(spec.menu.into()), &code),
        Outcome::Ok,
        "{code}"
    );
    fetch(e, result)
}

/// The tidyverse style must give the very table the base style gives. Needs
/// dplyr, which CI installs; skipped with a note where it is missing.
#[test]
fn tidyverse_style_gives_the_same_tables_as_base_style() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let have = run(
        &mut e,
        Source::Typed,
        "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: the R package dplyr is not installed");
        return;
    }
    type Case<'a> = (&'a Spec, Vec<(&'a str, &'a str)>, &'a str);
    let cases: [Case; 6] = [
        (
            &SORT,
            vec![("by1", "g"), ("by2", "x"), ("order2", "down")],
            "d",
        ),
        (&SORT, vec![("by1", "x")], "d"),
        (&FILTER, vec![("condition", "x > 1")], "d"),
        (
            &FILTER,
            vec![
                ("condition", "x > 1"),
                ("keep", "drop"),
                ("result", "small"),
            ],
            "small",
        ),
        (
            &COMPUTE,
            vec![("name", "x two"), ("value", "x * 2"), ("result", "d2")],
            "d2",
        ),
        (&RENAME, vec![("old", "g"), ("new", "group")], "d"),
    ];
    for (spec, set, result) in cases {
        let base = apply_in(&mut e, spec, &set, result, Style::Base);
        let tidy = apply_in(&mut e, spec, &set, result, Style::Tidyverse);
        assert!(base.is_some(), "{} gave no table in base style", spec.entry);
        assert_eq!(
            tidy.as_ref().map(shown),
            base.as_ref().map(shown),
            "{} differs between the styles: {set:?}",
            spec.entry
        );
    }
}
