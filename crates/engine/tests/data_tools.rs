//! The Data menu's value-changing dialogs (recode, convert, factor levels,
//! text, dates) end to end against a real R (skipped with a note when none is
//! found): build each dialog's base-R code from its spec and the table's
//! columns, run it, and compare what R gives back.

use std::time::{Duration, Instant};

use rcraft_catalog::data_tools::{
    COMBINE_LEVELS, CONVERT_TYPE, DATE_PART, EXTRACT_TEXT, FIND_REPLACE, RECODE, REFERENCE_LEVEL,
    REORDER_LEVELS, TEXT_CASE, TEXT_TO_DATE,
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
        frames: Vec::new(),
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

fn strings(items: &[&str]) -> Option<Vec<String>> {
    Some(items.iter().map(|s| s.to_string()).collect())
}

const MAKE: &str = "d <- data.frame(n = c(1, 2, 3, NA, 9), \
     t = c(' Ab c ', 'x', NA, 'ab', 'NA'), \
     g = factor(c('a', 'b', 'c', 'a', 'b')), \
     f = factor(c('10', '20', '10', '30', '20')), \
     u = c('a.b', 'a.b', NA, 'A.B', 'x'), \
     s = c('2026-03-27', '27/03/2026', '', '2026-12-01', NA)); \
     attr(d$n, 'label') <- 'Score'";

/// The code `spec` makes for the current `d` with these values.
fn code_for(e: &mut Engine, spec: &Spec, set: &[(&str, &str)], flags: &[&str]) -> String {
    let table = fetch(e, "d");
    assert!(table.is_some(), "R has no data frame d");
    let Some(table) = table else {
        return String::new();
    };
    let ctx = context("d", &table);
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for id in flags {
        values.set_flag(id, true);
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    code.unwrap_or_default()
}

/// Builds `d` afresh, runs `spec` on it and returns the outcome.
fn try_on(
    e: &mut Engine,
    make: &str,
    spec: &Spec,
    set: &[(&str, &str)],
    flags: &[&str],
) -> Outcome {
    assert_eq!(run(e, Source::Typed, make), Outcome::Ok);
    let code = code_for(e, spec, set, flags);
    run(e, Source::Menu(spec.menu.into()), &code)
}

/// As [`try_on`], expecting success, and returns the table `d`.
fn apply(e: &mut Engine, spec: &Spec, set: &[(&str, &str)]) -> Option<Table> {
    assert_eq!(try_on(e, MAKE, spec, set, &[]), Outcome::Ok, "{set:?}");
    fetch(e, "d")
}

/// Runs `spec` on the `d` as it is now.
fn apply_again(e: &mut Engine, spec: &Spec, set: &[(&str, &str)]) -> Option<Table> {
    let code = code_for(e, spec, set, &[]);
    assert_eq!(
        run(e, Source::Menu(spec.menu.into()), &code),
        Outcome::Ok,
        "{code}"
    );
    fetch(e, "d")
}

/// Runs R that stops unless every one of `exprs` holds.
fn holds(e: &mut Engine, exprs: &str) {
    assert_eq!(
        run(e, Source::Typed, &format!("stopifnot({exprs})")),
        Outcome::Ok,
        "{exprs}"
    );
}

#[test]
fn recode_and_convert_change_values_and_keep_labels() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    // text news make the variable text; the first matching rule wins
    let t = apply(
        &mut e,
        &RECODE,
        &[("var", "n"), ("rules", "1, 2 = Low; 3 to 9 = High; NA = 0")],
    );
    assert_eq!(
        t.as_ref().map(|t| column(t, "n")),
        strings(&["Low", "Low", "High", "0", "High"])
    );
    assert!(t.is_some_and(|t| t
        .columns
        .iter()
        .any(|c| c.name == "n" && c.label() == Some("Score"))));

    // numbers only, others made missing, into a new variable
    let t = apply(
        &mut e,
        &RECODE,
        &[
            ("var", "n"),
            ("rules", "9 = 90; 3 to 5 = 30"),
            ("others", "missing"),
            ("into", "n2"),
        ],
    );
    assert_eq!(
        t.map(|t| column(&t, "n2")),
        strings(&["NA", "NA", "30", "NA", "90"])
    );
    holds(&mut e, "is.numeric(d$n2), identical(d$n[1:2], c(1, 2))");

    // text: a quoted NA is the text, an unquoted one is missing
    let t = apply(
        &mut e,
        &RECODE,
        &[("var", "t"), ("rules", "\"NA\" = Missing; NA = None")],
    );
    assert_eq!(
        t.map(|t| column(&t, "t")),
        strings(&[" Ab c ", "x", "None", "ab", "Missing"])
    );

    // factors come out as text
    let t = apply(&mut e, &RECODE, &[("var", "g"), ("rules", "a, b = ab")]);
    assert_eq!(
        t.map(|t| column(&t, "g")),
        strings(&["ab", "ab", "c", "ab", "ab"])
    );

    // a factor of numbers converts by what it says, not by its codes
    let t = apply(&mut e, &CONVERT_TYPE, &[("var", "f"), ("to", "number")]);
    assert_eq!(
        t.map(|t| column(&t, "f")),
        strings(&["10", "20", "10", "30", "20"])
    );
    holds(&mut e, "is.numeric(d$f)");
    let t = apply(
        &mut e,
        &CONVERT_TYPE,
        &[("var", "n"), ("to", "text"), ("into", "ntext")],
    );
    assert_eq!(
        t.map(|t| column(&t, "ntext")),
        strings(&["1", "2", "3", "NA", "9"])
    );
    holds(
        &mut e,
        "is.character(d$ntext), identical(attr(d$ntext, 'label'), 'Score')",
    );
    apply(&mut e, &CONVERT_TYPE, &[("var", "t"), ("to", "factor")]);
    holds(&mut e, "is.factor(d$t), sum(is.na(d$t)) == 1");
}

#[test]
fn factor_dialogs_change_the_levels_as_shown() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    apply(&mut e, &REFERENCE_LEVEL, &[("var", "g"), ("level", "c")]);
    holds(&mut e, "identical(levels(d$g), c('c', 'a', 'b'))");
    assert_eq!(
        try_on(
            &mut e,
            MAKE,
            &REFERENCE_LEVEL,
            &[("var", "g"), ("level", "zzz")],
            &[]
        ),
        Outcome::Error
    );

    apply(
        &mut e,
        &COMBINE_LEVELS,
        &[("var", "g"), ("groups", "a, b = ab")],
    );
    holds(
        &mut e,
        "identical(levels(d$g), c('ab', 'c')), \
         identical(as.character(d$g), c('ab', 'ab', 'c', 'ab', 'ab'))",
    );
    assert_eq!(
        try_on(
            &mut e,
            MAKE,
            &COMBINE_LEVELS,
            &[("var", "g"), ("groups", "a, zzz = ab")],
            &[]
        ),
        Outcome::Error
    );

    apply(
        &mut e,
        &REORDER_LEVELS,
        &[("var", "g"), ("levels", "c, b, a"), ("into", "g2")],
    );
    holds(
        &mut e,
        "identical(levels(d$g2), c('c', 'b', 'a')), identical(levels(d$g), c('a', 'b', 'c'))",
    );
    // leaving a level out would turn its values missing: R refuses
    assert_eq!(
        try_on(
            &mut e,
            MAKE,
            &REORDER_LEVELS,
            &[("var", "g"), ("levels", "c, b")],
            &[]
        ),
        Outcome::Error
    );
}

#[test]
fn text_dialogs_change_the_text_as_shown() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    let t = apply(&mut e, &TEXT_CASE, &[("var", "t"), ("how", "squish")]);
    assert_eq!(
        t.map(|t| column(&t, "t")),
        strings(&["Ab c", "x", "NA", "ab", "NA"])
    );
    holds(&mut e, "is.na(d$t[3])");
    apply(&mut e, &TEXT_CASE, &[("var", "t"), ("how", "title")]);
    holds(&mut e, "d$t[4] == 'Ab', is.na(d$t[3])");
    apply(&mut e, &TEXT_CASE, &[("var", "g"), ("how", "upper")]);
    holds(&mut e, "identical(d$g, c('A', 'B', 'C', 'A', 'B'))");

    // literal by default: the dot is not "any character"
    let t = apply(
        &mut e,
        &FIND_REPLACE,
        &[("var", "u"), ("find", "."), ("with", "-")],
    );
    assert_eq!(
        t.map(|t| column(&t, "u")),
        strings(&["a-b", "a-b", "NA", "A-B", "x"])
    );
    // a backslash in the replacement stays a backslash
    let t = apply(
        &mut e,
        &FIND_REPLACE,
        &[
            ("var", "u"),
            ("find", "a.b"),
            ("with", "\\1"),
            ("into", "u2"),
        ],
    );
    assert_eq!(
        t.map(|t| column(&t, "u2")),
        strings(&["\\1", "\\1", "NA", "A.B", "x"])
    );
    // ignoring capitals is still literal, and so is the replacement
    assert_eq!(
        try_on(
            &mut e,
            MAKE,
            &FIND_REPLACE,
            &[("var", "u"), ("find", "A.B"), ("with", "\\1")],
            &["ignore"]
        ),
        Outcome::Ok
    );
    holds(
        &mut e,
        "identical(d$u[c(1, 2, 4, 5)], c('\\\\1', '\\\\1', '\\\\1', 'x'))",
    );
    // a pattern
    assert_eq!(
        try_on(
            &mut e,
            MAKE,
            &FIND_REPLACE,
            &[("var", "u"), ("find", "^[aA]"), ("with", "z")],
            &["pattern"]
        ),
        Outcome::Ok
    );
    holds(&mut e, "identical(d$u[c(1, 4)], c('z.b', 'z.B'))");

    let t = apply(
        &mut e,
        &EXTRACT_TEXT,
        &[("var", "s"), ("from", "1"), ("to", "4"), ("into", "year")],
    );
    assert_eq!(
        t.map(|t| column(&t, "year")),
        strings(&["2026", "27/0", "", "2026", "NA"])
    );
    let t = apply(
        &mut e,
        &EXTRACT_TEXT,
        &[("var", "s"), ("from", "6"), ("into", "rest")],
    );
    assert_eq!(
        t.map(|t| column(&t, "rest")),
        strings(&["03-27", "/2026", "", "12-01", "NA"])
    );
}

#[test]
fn date_dialogs_make_dates_and_take_them_apart() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    // one value does not fit the layout: it becomes missing
    let t = apply(
        &mut e,
        &TEXT_TO_DATE,
        &[("var", "s"), ("layout", "%Y-%m-%d")],
    );
    assert_eq!(
        t.map(|t| column(&t, "s")),
        strings(&["2026-03-27", "NA", "NA", "2026-12-01", "NA"])
    );
    holds(&mut e, "inherits(d$s, 'Date')");

    for (part, want) in [
        ("year", strings(&["2026", "NA", "NA", "2026", "NA"])),
        ("month", strings(&["3", "NA", "NA", "12", "NA"])),
        ("day", strings(&["27", "NA", "NA", "1", "NA"])),
        ("quarter", strings(&["1", "NA", "NA", "4", "NA"])),
    ] {
        let t = apply_again(
            &mut e,
            &DATE_PART,
            &[("var", "s"), ("part", part), ("into", "p")],
        );
        assert_eq!(t.map(|t| column(&t, "p")), want, "{part}");
    }
    // weekday names follow R's language, so only that they were made is checked
    apply_again(
        &mut e,
        &DATE_PART,
        &[("var", "s"), ("part", "weekday"), ("into", "w")],
    );
    holds(&mut e, "is.character(d$w), sum(!is.na(d$w)) == 2");

    // another layout
    assert_eq!(
        try_on(
            &mut e,
            "d <- data.frame(s = c('27/03/2026', '03/27/2026', '20260327'))",
            &TEXT_TO_DATE,
            &[("var", "s"), ("layout", "%d/%m/%Y")],
            &[]
        ),
        Outcome::Ok
    );
    holds(&mut e, "identical(format(d$s), c('2026-03-27', NA, NA))");
}
