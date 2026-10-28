//! The do-file front end taken through its three steps on a small do-file:
//! read it into command lines, take a command line apart, parse its
//! expression. This is what P2C.3 promises: the reader, the command syntax
//! and the expression parser, usable from RCraft.

use rcraft_statatr::lang::expr::{self, BinaryOp, Expr};
use rcraft_statatr::lang::script;
use rcraft_statatr::lang::syntax::{self, WeightKind};
use rcraft_statatr::values::missing;

const DO_FILE: &str = "\
* a small analysis
use \"data.dta\", clear // load
regress price mpg weight if foreign == 1 & !missing(rep78) ///
    [aweight = weight], robust vce(cluster id)
generate big = inrange(price, 5000, .) in 1/10
";

#[test]
fn a_do_file_is_read_into_command_lines_with_continuations_joined() {
    let lines = script::read(DO_FILE);
    let commands: Vec<&str> = lines
        .iter()
        .filter(|l| !l.is_blank())
        .map(|l| l.text.trim())
        .collect();
    assert_eq!(commands.len(), 3, "{commands:?}");
    assert_eq!(commands[0], "use \"data.dta\", clear");
    assert!(commands[1].starts_with("regress price mpg weight if foreign == 1"));
    assert!(
        commands[1].contains("[aweight = weight], robust"),
        "{}",
        commands[1]
    );
    // the physical lines the long command came from are kept for the log
    let long = lines
        .iter()
        .find(|l| l.text.contains("regress"))
        .map(|l| l.physical.len());
    assert_eq!(long, Some(2));
}

#[test]
fn a_command_line_comes_apart_into_its_clauses() {
    let lines = script::read(DO_FILE);
    let regress = lines
        .iter()
        .find(|l| l.text.contains("regress"))
        .map(|l| l.text.clone());
    let text = regress.unwrap_or_default();
    // the command name comes off first
    let rest = text
        .trim()
        .strip_prefix("regress")
        .unwrap_or("")
        .trim()
        .to_string();
    let parts = syntax::parts(&rest);
    assert_eq!(parts.main.trim(), "price mpg weight");
    assert_eq!(
        parts.if_exp.as_deref().map(str::trim),
        Some("foreign == 1 & !missing(rep78)")
    );
    let weight = parts.weight.as_ref();
    assert_eq!(weight.map(|w| w.kind), Some(WeightKind::Analytic));
    assert_eq!(weight.map(|w| w.exp.as_str()), Some("weight"));
    let opts = syntax::options(&parts.options);
    assert!(opts.iter().any(|o| o.is("robust", 3)));
    assert_eq!(
        opts.iter().find(|o| o.is("vce", 3)).map(|o| o.arg()),
        Some("cluster id".to_string())
    );
}

#[test]
fn an_if_expression_parses_into_a_tree_and_bad_input_is_an_error_not_a_crash() {
    let e = expr::parse("foreign == 1 & !missing(rep78)");
    assert!(matches!(e, Ok(Expr::Binary(BinaryOp::And, _, _))), "{e:?}");
    // Stata's missing values are numbers that sort high
    let dot = expr::parse(".");
    assert!(
        matches!(dot, Ok(Expr::Number(v)) if missing::is_missing(v)),
        "{dot:?}"
    );
    // a date literal becomes a call to date() with its mask, for the translator to handle
    let d = expr::parse("td(01jan1960)");
    assert!(
        matches!(&d, Ok(Expr::Call(name, args)) if name == "date" && args.len() == 2),
        "{d:?}"
    );
    for bad in ["", "(", "a +", "x ==== 1", "f(,", &"(".repeat(500)] {
        assert!(expr::parse(bad).is_err(), "{bad:?} should be refused");
    }
}
