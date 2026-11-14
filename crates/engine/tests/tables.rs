//! Rich tables in Results (P2.3) against a real R (skipped with a note when
//! none is found): a command whose value is a data frame, matrix or table (or a
//! named list of them) gives a table with R's own printed text, so the text,
//! the log and the script are as they were; anything else prints as before.

use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, ResultTable, Source};
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

/// Runs `code`; returns its text items, each with its table if it has one.
fn run(e: &mut Engine, code: &str) -> Vec<(String, Option<ResultTable>)> {
    let id = e.submit(Source::Typed, code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e.history().entries().last().and_then(|h| h.outcome);
    assert_eq!(outcome, Some(Outcome::Ok), "{code}");
    e.results()
        .blocks()
        .iter()
        .filter(|b| b.command == id)
        .flat_map(|b| b.items.iter())
        .filter_map(|i| match i {
            Item::Text { text, table, .. } => Some((text.clone(), table.as_deref().cloned())),
            _ => None,
        })
        .collect()
}

/// What R prints for `expr`, as plain text.
fn printed(e: &mut Engine, expr: &str) -> String {
    let items = run(
        e,
        &format!("cat(paste0(utils::capture.output(print({expr})), \"\\n\"), sep = \"\")"),
    );
    items.into_iter().map(|(t, _)| t).collect()
}

#[test]
fn values_that_are_tables_come_as_tables_with_rs_own_text() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    // a data frame
    let items = run(&mut e, "head(mtcars[1:3], 3)");
    assert_eq!(items.len(), 1, "{items:?}");
    let (text, table) = &items[0];
    assert_eq!(*text, printed(&mut e, "head(mtcars[1:3], 3)"));
    let t = table.clone().unwrap_or_default();
    assert_eq!(t.columns, vec!["mpg", "cyl", "disp"]);
    assert_eq!(t.numeric, vec![true, true, true]);
    assert_eq!(t.rows.len(), 3);
    assert_eq!(t.rows[0].0, "Mazda RX4");
    assert_eq!(t.rows[0].1, vec!["21.0", "6", "160"], "R's own formatting");

    // a one-way table: one row per value
    let items = run(&mut e, "table(g = c('a', 'b', 'a', NA), useNA = 'ifany')");
    let t = items[0].1.clone().unwrap_or_default();
    assert_eq!(t.corner, "g");
    assert_eq!(t.columns, vec!["Count"]);
    let rows: Vec<(String, String)> = t
        .rows
        .iter()
        .map(|(n, c)| (n.clone(), c.join(",")))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("a".to_string(), "2".to_string()),
            ("b".to_string(), "1".to_string()),
            ("NA".to_string(), "1".to_string())
        ]
    );

    // a two-way table names both variables in the corner
    let items = run(&mut e, "with(mtcars, table(cyl, gear))");
    let t = items[0].1.clone().unwrap_or_default();
    assert_eq!(t.corner, "cyl \\ gear");
    assert_eq!(t.columns, vec!["3", "4", "5"]);
    assert_eq!(
        t.rows[0],
        ("4".to_string(), vec!["1".into(), "8".into(), "2".into()])
    );

    // a matrix without names is labelled as R labels it
    let items = run(&mut e, "matrix(c(1.5, 2, 3, 4), 2)");
    let t = items[0].1.clone().unwrap_or_default();
    assert_eq!(t.columns, vec!["[,1]", "[,2]"]);
    assert_eq!(t.rows[1].0, "[2,]");
    assert_eq!(t.rows[0].1, vec!["1.5", "3"]);

    // a named list of tables: one table each, all of R's text on the first
    let code = "list(first = matrix(1:4, 2), second = data.frame(x = c('u', 'v')))";
    let items = run(&mut e, code);
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0].1.as_ref().map(|t| t.title.as_str()), Some("first"));
    assert_eq!(
        items[1].1.as_ref().map(|t| t.title.as_str()),
        Some("second")
    );
    assert_eq!(
        items[1].1.as_ref().map(|t| t.numeric.clone()),
        Some(vec![false])
    );
    let all: String = items.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(all, printed(&mut e, code));
}

#[test]
fn other_values_and_large_tables_print_as_before() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    for code in [
        "1:5",
        "summary(c(1, 2, 3))",
        "list(1, 2)",
        "list(a = 1:3)",
        "array(1:8, c(2, 2, 2))",
        "data.frame(x = 1:600)",
        "invisible(mtcars)",
        "x <- mtcars",
        "print(head(mtcars, 2))",
    ] {
        let items = run(&mut e, code);
        assert!(
            items.iter().all(|(_, t)| t.is_none()),
            "{code} should not be a table: {items:?}"
        );
    }
    // turned off, a data frame prints as text
    let items = run(&mut e, "options(rcraft.tables = FALSE); head(mtcars, 2)");
    assert!(items.iter().all(|(_, t)| t.is_none()), "{items:?}");
    assert!(items.iter().any(|(t, _)| t.contains("Mazda RX4")));
    let items = run(&mut e, "options(rcraft.tables = TRUE); head(mtcars, 2)");
    assert!(items.iter().any(|(_, t)| t.is_some()), "{items:?}");
    // inside capture.output, nothing is taken away from what it captures
    let items = run(
        &mut e,
        "x <- capture.output(head(mtcars, 2)); cat(length(x), '\\n')",
    );
    assert!(items.iter().any(|(t, _)| t.trim() == "3"), "{items:?}");
}

#[test]
fn the_log_keeps_rs_text_for_a_table() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let items = run(&mut e, "head(mtcars[1:2], 2)");
    assert!(items[0].1.is_some());
    assert!(
        e.log_text().contains("Mazda RX4 Wag  21   6"),
        "{}",
        e.log_text()
    );
}
