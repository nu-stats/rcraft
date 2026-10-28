//! P2B.2: Stata expressions translated to R give Stata's answers when R runs
//! them. Each expression is translated, run with `dplyr::mutate()` on data with
//! missing values, and compared with the values Stata's rules give (worked out
//! by hand below). Needs a real R with dplyr; skipped with a note otherwise.

use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_statatr::expr_r::{translate, Kind, Use};

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

fn run(e: &mut Engine, code: &str) -> Outcome {
    let id = e.submit(Source::Typed, code);
    assert!(id.is_ok(), "{id:?}");
    settle(e);
    e.history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error)
}

fn kind_of(name: &str) -> Option<Kind> {
    match name {
        "s" => Some(Kind::Text),
        "x" | "y" | "r" | "g" => Some(Kind::Number),
        _ => None,
    }
}

#[test]
fn translated_expressions_give_statas_answers_in_r() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    if run(
        &mut e,
        "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE))",
    ) != Outcome::Ok
    {
        eprintln!("skipped: the R package dplyr is not installed");
        return;
    }
    let made = run(
        &mut e,
        "d <- data.frame(x = c(1, NA, 5, 7, 0), y = c(2, 3, NA, 0, 1), \
         r = c(2.345, -2.5, 0.5, 1.5, NA), g = c(1, 1, 2, 2, 2), \
         s = c('ab', '', 'b a', 'abc', 'x'))",
    );
    assert_eq!(made, Outcome::Ok);

    // (expression, use, grouped by g, the values Stata gives as R)
    let cases: &[(&str, Use, bool, &str)] = &[
        // a missing number is larger than every number
        ("x > 5", Use::Value, false, "c(0, 1, 0, 1, 0)"),
        ("x < 5", Use::Value, false, "c(1, 0, 0, 0, 1)"),
        ("x == .", Use::Value, false, "c(0, 1, 0, 0, 0)"),
        ("x > y", Use::Value, false, "c(0, 1, 0, 1, 0)"),
        ("x != y", Use::Value, false, "c(1, 1, 1, 1, 1)"),
        ("x >= 5 & y < 3", Use::Value, false, "c(0, 0, 0, 1, 0)"),
        ("missing(x) | y == 0", Use::Value, false, "c(0, 1, 0, 1, 0)"),
        // anything not zero is true, missing included
        ("!x", Use::Value, false, "c(0, 0, 0, 0, 1)"),
        (
            "x > 5",
            Use::Condition,
            false,
            "c(FALSE, TRUE, FALSE, TRUE, FALSE)",
        ),
        (
            "x",
            Use::Condition,
            false,
            "c(TRUE, TRUE, TRUE, TRUE, FALSE)",
        ),
        // x / 0 is missing
        ("x / y", Use::Value, false, "c(0.5, NA, NA, NA, 0)"),
        ("max(x, y)", Use::Value, false, "c(2, 3, 5, 7, 1)"),
        ("min(x, y, .)", Use::Value, false, "c(1, 3, 5, 0, 0)"),
        ("mod(x, 3)", Use::Value, false, "c(1, NA, 2, 1, 0)"),
        ("mod(x, y)", Use::Value, false, "c(1, NA, NA, NA, 0)"),
        // halfway goes up, with Stata's allowance (2.345 to 2.35)
        (
            "round(r, 0.01)",
            Use::Value,
            false,
            "c(2.35, -2.5, 0.5, 1.5, NA)",
        ),
        ("round(r)", Use::Value, false, "c(2, -2, 1, 2, NA)"),
        ("inrange(x, 1, 5)", Use::Value, false, "c(1, 0, 1, 0, 0)"),
        ("inlist(x, 1, 7, .)", Use::Value, false, "c(1, 1, 0, 1, 0)"),
        (
            "cond(x > 1, 10, 20)",
            Use::Value,
            false,
            "c(20, 10, 10, 10, 20)",
        ),
        ("cond(x, 1, 2, 3)", Use::Value, false, "c(1, 3, 1, 1, 2)"),
        ("sum(x)", Use::Value, false, "c(1, 1, 6, 13, 13)"),
        ("x[_n-1]", Use::Value, false, "c(NA, 1, NA, 5, 7)"),
        ("x[_n+1]", Use::Value, false, "c(NA, 5, 7, 0, NA)"),
        ("_n * 10", Use::Value, false, "c(10, 20, 30, 40, 50)"),
        // by g: _n, _N, lags and running sums start again in each group
        ("_n", Use::Value, true, "c(1, 2, 1, 2, 3)"),
        ("_N", Use::Value, true, "c(2, 2, 3, 3, 3)"),
        ("x[_n-1]", Use::Value, true, "c(NA, 1, NA, 5, 7)"),
        ("x[1]", Use::Value, true, "c(1, 1, 5, 5, 5)"),
        ("x[_N]", Use::Value, true, "c(NA, NA, 0, 0, 0)"),
        ("sum(x)", Use::Value, true, "c(1, 1, 5, 12, 12)"),
        // strings: the empty string is Stata's missing string
        (
            "s + \"!\"",
            Use::Value,
            false,
            "c('ab!', '!', 'b a!', 'abc!', 'x!')",
        ),
        ("s == \"ab\"", Use::Value, false, "c(1, 0, 0, 0, 0)"),
        ("strlen(s)", Use::Value, false, "c(2, 0, 3, 3, 1)"),
        (
            "substr(s, 2, 2)",
            Use::Value,
            false,
            "c('b', '', ' a', 'bc', '')",
        ),
        ("strpos(s, \"b\")", Use::Value, false, "c(2, 0, 1, 2, 0)"),
        (
            "subinstr(s, \"a\", \"z\", .)",
            Use::Value,
            false,
            "c('zb', '', 'b z', 'zbc', 'x')",
        ),
        (
            "upper(s)",
            Use::Value,
            false,
            "c('AB', '', 'B A', 'ABC', 'X')",
        ),
        ("missing(s)", Use::Value, false, "c(0, 1, 0, 0, 0)"),
        // dates are days since 1 January 1960
        (
            "mdy(1, 2, 1960) + 0 * x",
            Use::Value,
            false,
            "c(1, NA, 1, 1, 1)",
        ),
        (
            "year(td(15mar2001)) + 0 * _n",
            Use::Value,
            false,
            "rep(2001, 5)",
        ),
        (
            "dow(mdy(1, 1, 1960)) + 0 * _n",
            Use::Value,
            false,
            "rep(5, 5)",
        ),
    ];
    let mut wrong = Vec::new();
    for (text, use_, grouped, expected) in cases {
        let translated = match translate(text, &kind_of, *use_) {
            Ok(t) => t,
            Err(why) => {
                wrong.push(format!("{text}: not translated ({})", why.reason));
                continue;
            }
        };
        let data = if *grouped {
            "dplyr::group_by(d, g)"
        } else {
            "d"
        };
        let check = if *use_ == Use::Condition {
            "identical(v, e)"
        } else {
            "isTRUE(all.equal(v, e))"
        };
        let code = format!(
            "local({{ v <- dplyr::mutate({data}, .v = {})$.v; e <- {expected}; \
             if (!{check}) stop(paste(format(v), collapse = ' ')) }})",
            translated.code
        );
        if run(&mut e, &code) != Outcome::Ok {
            wrong.push(format!(
                "{text} (as {}): R did not give {expected}",
                translated.code
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
