//! Graphs menu dialogs (P5.3) end to end against a real R (skipped with a note
//! when none is found): each dialog's code is built from its spec, run, and must
//! draw exactly the plots it should, as real PNG files, without an error.

mod pinned;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rcraft_catalog::graphs::{
    BAR_CHART, BOX_PLOT, DENSITY, HEATMAP, HISTOGRAM, LINE_PLOT, QQ_PLOT, SCATTER, SCATTER_MATRIX,
};
use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rcodegen::dialog::{ColumnInfo, Context, Spec, Style, VarKind, Widget};
use rcraft_rhost::{find_installs, Env, SessionConfig};

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
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

/// Runs code; returns how it ended, the plots it drew and its messages.
fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, Vec<PathBuf>, String) {
    let id = e.submit(source, code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let mut plots = Vec::new();
    let mut text = String::new();
    for block in e.results().blocks().iter().filter(|b| b.command == id) {
        for item in &block.items {
            match item {
                Item::Plot { path } => plots.push(path.clone()),
                Item::Text { text: t, .. } => text.push_str(t),
                Item::Warning(w) => text.push_str(&format!("warning: {w}\n")),
                Item::Error { message, .. } => text.push_str(&format!("error: {message}\n")),
                Item::Notice(n) => text.push_str(n),
            }
        }
    }
    (outcome, plots, text)
}

const MAKE: &str = "d <- data.frame(\
     x = c(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12), \
     y = c(2.1, 3.9, 6.2, 7.8, 10.1, 12.2, 13.8, 16.1, 18.0, 20.2, 21.9, 24.1), \
     z = c(5, 3, 8, 1, 9, 4, 7, 2, 6, 10, 3, 5), \
     g = factor(rep(c('a', 'b', 'c'), 4)), \
     t = c('u', 'v', 'u', 'v', 'u', 'v', 'u', 'v', 'u', 'v', 'u', NA), \
     day = as.Date('2026-01-01') + 0:11)";

fn context() -> Context {
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: [
            ("x", VarKind::Number),
            ("y", VarKind::Number),
            ("z", VarKind::Number),
            ("g", VarKind::Factor),
            ("t", VarKind::Text),
            ("day", VarKind::Date),
        ]
        .into_iter()
        .map(|(name, kind)| ColumnInfo {
            name: name.into(),
            kind,
        })
        .collect(),
    }
}

/// Builds the dialog's code with these values and runs it on made-up data.
fn draw(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) -> (Outcome, Vec<PathBuf>, String) {
    let (made, _, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, flags.contains(id));
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    let (outcome, plots, text) = run(e, Source::Menu(spec.menu.into()), &code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    assert!(!text.contains("error:"), "{code}\n{text}");
    (outcome, plots, text)
}

fn is_png(path: &std::path::Path) -> bool {
    std::fs::read(path).is_ok_and(|b| {
        b.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) && b.len() > 2000
    })
}

/// One dialog draws exactly one plot, a real picture.
fn one_plot(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) {
    let (_, plots, text) = draw(e, spec, set, lists, flags);
    assert_eq!(plots.len(), 1, "{}: {plots:?}\n{text}", spec.entry);
    assert!(
        plots.first().is_some_and(|p| is_png(p)),
        "{}: {plots:?}",
        spec.entry
    );
}

#[test]
fn every_graph_dialog_draws_one_real_picture() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    one_plot(&mut e, &HISTOGRAM, &[("x", "y")], &[], &[]);
    one_plot(
        &mut e,
        &HISTOGRAM,
        &[("x", "y"), ("breaks", "FD"), ("title", "My bars")],
        &[],
        &["density"],
    );
    one_plot(&mut e, &HISTOGRAM, &[("x", "y"), ("bins", "4")], &[], &[]);
    one_plot(&mut e, &DENSITY, &[("x", "y")], &[], &[]);
    one_plot(&mut e, &DENSITY, &[("x", "y"), ("g", "g")], &[], &[]);
    one_plot(&mut e, &BOX_PLOT, &[("x", "y")], &[], &[]);
    one_plot(&mut e, &BOX_PLOT, &[("x", "y"), ("g", "g")], &[], &[]);
    one_plot(&mut e, &BAR_CHART, &[("x", "g")], &[], &[]);
    one_plot(&mut e, &BAR_CHART, &[("x", "t")], &[], &["percent"]);
    one_plot(&mut e, &QQ_PLOT, &[("x", "y")], &[], &[]);
    one_plot(&mut e, &SCATTER, &[("x", "x"), ("y", "y")], &[], &[]);
    one_plot(
        &mut e,
        &SCATTER,
        &[("x", "x"), ("y", "z"), ("g", "g")],
        &[],
        &["fit", "smooth"],
    );
    one_plot(&mut e, &LINE_PLOT, &[("x", "day"), ("y", "y")], &[], &[]);
    one_plot(
        &mut e,
        &LINE_PLOT,
        &[("x", "x"), ("y", "z")],
        &[],
        &["points"],
    );
    one_plot(
        &mut e,
        &SCATTER_MATRIX,
        &[],
        &[("vars", &["x", "y", "z"])],
        &[],
    );
}

#[test]
fn the_heatmap_draws_one_picture_each_way_and_a_constant_variable_does_not_stop_it() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let xyz: &[&str] = &["x", "y", "z"];
    one_plot(&mut e, &HEATMAP, &[], &[("vars", xyz)], &["numbers"]);
    one_plot(
        &mut e,
        &HEATMAP,
        &[("method", "kendall")],
        &[("vars", xyz)],
        &["cluster"],
    );
    one_plot(
        &mut e,
        &HEATMAP,
        &[("show", "values")],
        &[("vars", xyz)],
        &[],
    );
    one_plot(
        &mut e,
        &HEATMAP,
        &[("show", "values")],
        &[("vars", xyz)],
        &["cluster"],
    );
    // a variable that never changes has no correlation: the cells stay blank and
    // the order asked for falls back to the chosen one, with a message
    let (made, _, _) = run(&mut e, Source::Typed, "d$k <- 1; 0");
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
    let mut ctx = ctx;
    ctx.columns.push(ColumnInfo {
        name: "k".into(),
        kind: VarKind::Number,
    });
    let mut values = HEATMAP.defaults(&ctx);
    values.set_list("vars", vec!["x".into(), "k".into(), "z".into()]);
    values.set_flag("cluster", true);
    let code = HEATMAP
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, plots, text) = run(&mut e, Source::Menu(HEATMAP.menu.into()), &code);
    assert_eq!(
        outcome,
        Outcome::Ok,
        "{code}
{text}"
    );
    assert_eq!(plots.len(), 1, "{text}");
    assert!(text.contains("left in the order chosen"), "{text}");
}

#[test]
fn a_plot_dialog_run_again_draws_another_and_a_typed_command_can_add_to_it() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let (_, first, _) = draw(&mut e, &SCATTER, &[("x", "x"), ("y", "y")], &[], &[]);
    assert_eq!(first.len(), 1);
    // abline() in a typed command finds the scatter plot: the device stayed open
    let (outcome, second, text) = run(
        &mut e,
        Source::Typed,
        "abline(lm(y ~ x, data = d), col = 'red')",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(second.len(), 1, "{second:?}");
    assert_ne!(
        std::fs::read(&first[0]).unwrap_or_default(),
        std::fs::read(&second[0]).unwrap_or_default()
    );
}

#[test]
fn missing_values_do_not_stop_a_plot() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    // t has a missing value; a bar chart and a scatter with a missing y still draw
    one_plot(&mut e, &BAR_CHART, &[("x", "t")], &[], &[]);
    let (made, _, _) = run(&mut e, Source::Typed, "d$y[3] <- NA; d$x[5] <- NA; 0");
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
    let mut values = SCATTER.defaults(&ctx);
    values.set_text("x", "x");
    values.set_text("y", "y");
    values.set_flag("fit", true);
    values.set_flag("smooth", true);
    let code = SCATTER
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, plots, text) = run(&mut e, Source::Menu(SCATTER.menu.into()), &code);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert_eq!(plots.len(), 1);
}

/// Base R's graphics only.
const PACKAGES: &[&str] = &[];

/// Runs R that stops unless `exprs` holds.
fn holds(e: &mut Engine, exprs: &str) {
    let (outcome, _, text) = run(e, Source::Typed, &format!("stopifnot({exprs})"));
    assert_eq!(outcome, Outcome::Ok, "{exprs}\n{text}");
}

/// Draws the dialog's code on a recording device and keeps what it drew as
/// `s` (see `pinned::SHAPES`).
fn shapes_of(
    e: &mut Engine,
    spec: &Spec,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    flags: &[&str],
) {
    let (made, _, _) = run(e, Source::Typed, MAKE);
    assert_eq!(made, Outcome::Ok);
    let ctx = context();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            values.set_flag(id, flags.contains(id));
        }
    }
    let code = spec
        .generate(&ctx, &values, Style::Base)
        .unwrap_or_default();
    let (outcome, _, text) = run(e, Source::Menu(spec.menu.into()), &pinned::drawn(&code));
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
}

/// What each graph draws, read back from R's display list, against values
/// worked out by hand: the histogram's bars from Sturges' number of classes
/// and counts made row by row, densities from a Gaussian kernel written out
/// (density() bins first, so within 1e-3 of its largest value), Tukey's five
/// numbers in each box, the bars' counts and shares, the Q-Q points from the
/// normal scores and its line through the quartiles, the points and the
/// least-squares line of a scatter plot, a line plot in the order of x, every
/// panel of a scatter plot matrix, and a heatmap's colours and printed values
/// from correlations and standardised values computed by hand.
#[test]
fn what_each_graph_draws_is_what_the_data_give() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);

    shapes_of(&mut e, &HISTOGRAM, &[("x", "x")], &[], &[]);
    holds(
        &mut e,
        &pinned::all_of(
            "v <- d$x; k <- ceiling(log2(length(v)) + 1); br <- pretty(range(v), k); \
             n <- vapply(seq_len(length(br) - 1), function(i) \
               sum(v > br[i] & v <= br[i + 1]) + (i == 1) * sum(v == br[1]), 0); \
             r <- pick(s, 'C_rect')[[1]]",
            &[
                ("left", "same(r[[1]], head(br, -1))"),
                ("right", "same(r[[3]], br[-1])"),
                ("counts", "same(r[[4]], n)"),
            ],
        ),
    );
    shapes_of(&mut e, &HISTOGRAM, &[("x", "z")], &[], &["density"]);
    holds(
        &mut e,
        &pinned::all_of(
            "v <- d$z; k <- ceiling(log2(length(v)) + 1); br <- pretty(range(v), k); \
             n <- vapply(seq_len(length(br) - 1), function(i) \
               sum(v > br[i] & v <= br[i + 1]) + (i == 1) * sum(v == br[1]), 0); \
             r <- pick(s, 'C_rect')[[1]]; \
             curve <- Filter(function(a) length(a[[1]]$x) == 512, pick(s, 'C_plotXY'))[[1]][[1]]; \
             f <- kde(v, curve$x)",
            &[
                ("density", "same(r[[4]], n / (length(v) * diff(br)))"),
                ("curve", "max(abs(curve$y - f)) < 1e-3 * max(f)"),
            ],
        ),
    );

    shapes_of(&mut e, &DENSITY, &[("x", "y")], &[], &[]);
    holds(
        &mut e,
        &pinned::all_of(
            "p <- pick(s, 'C_polygon')[[1]]; f <- kde(d$y, p[[1]])",
            &[("curve", "max(abs(p[[2]] - f)) < 1e-3 * max(f)")],
        ),
    );
    shapes_of(&mut e, &DENSITY, &[("x", "y"), ("g", "g")], &[], &[]);
    holds(
        &mut e,
        &pinned::all_of(
            "curves <- Filter(function(a) length(a[[1]]$x) == 512, pick(s, 'C_plotXY')); \
             ok <- vapply(seq_along(levels(d$g)), function(i) { c <- curves[[i]][[1]]; \
               f <- kde(d$y[d$g == levels(d$g)[i]], c$x); max(abs(c$y - f)) < 1e-3 * max(f) }, TRUE)",
            &[("groups", "length(curves) == 3 && all(ok)")],
        ),
    );

    shapes_of(&mut e, &BOX_PLOT, &[("x", "y"), ("g", "g")], &[], &[]);
    holds(
        &mut e,
        &pinned::all_of(
            "five <- function(v) { v <- sort(v); n <- length(v); n4 <- floor((n + 3) / 2) / 2; \
               i <- c(1, n4, (n + 1) / 2, n + 1 - n4, n); 0.5 * (v[floor(i)] + v[ceiling(i)]) }; \
             want <- unlist(lapply(levels(d$g), function(l) five(d$y[d$g == l]))); \
             got <- c(unlist(lapply(pick(s, 'C_polygon'), `[[`, 2)), \
               unlist(lapply(pick(s, 'C_segments'), function(a) c(a[[2]], a[[4]])))) ",
            &[("five", "all(round(want, 10) %in% round(got, 10))")],
        ),
    );

    shapes_of(&mut e, &BAR_CHART, &[("x", "t")], &[], &[]);
    holds(
        &mut e,
        &pinned::all_of(
            "u <- sum(d$t == 'u', na.rm = TRUE); v <- sum(d$t == 'v', na.rm = TRUE)",
            &[("counts", "same(pick(s, 'C_rect')[[1]][[4]], c(u, v))")],
        ),
    );
    shapes_of(&mut e, &BAR_CHART, &[("x", "t")], &[], &["percent"]);
    holds(
        &mut e,
        &pinned::all_of(
            "u <- sum(d$t == 'u', na.rm = TRUE); v <- sum(d$t == 'v', na.rm = TRUE)",
            &[(
                "shares",
                "same(pick(s, 'C_rect')[[1]][[4]], 100 * c(u, v) / (u + v))",
            )],
        ),
    );

    shapes_of(&mut e, &QQ_PLOT, &[("x", "z")], &[], &[]);
    holds(
        &mut e,
        &pinned::all_of(
            "v <- d$z; n <- length(v); a <- if (n <= 10) 3 / 8 else 1 / 2; \
             sc <- qnorm((seq_len(n) - a) / (n + 1 - 2 * a))[rank(v, ties.method = 'first')]; \
             yq <- c(q7(v, 0.25), q7(v, 0.75)); xq <- qnorm(c(0.25, 0.75)); \
             slope <- diff(yq) / diff(xq); line <- pick(s, 'C_abline')[[1]]",
            &[
                ("points", "has_xy(s, sc, v)"),
                (
                    "line",
                    "same(c(line[[1]], line[[2]]), c(yq[1] - slope * xq[1], slope))",
                ),
            ],
        ),
    );

    shapes_of(&mut e, &SCATTER, &[("x", "x"), ("y", "y")], &[], &["fit"]);
    holds(
        &mut e,
        &pinned::all_of(
            "X <- cbind(1, d$x); b <- solve(crossprod(X), crossprod(X, d$y)); \
             line <- pick(s, 'C_abline')[[1]]",
            &[
                ("points", "has_xy(s, d$x, d$y)"),
                ("line", "same(c(line[[1]], line[[2]]), b)"),
            ],
        ),
    );

    shapes_of(&mut e, &LINE_PLOT, &[("x", "z"), ("y", "y")], &[], &[]);
    holds(
        &mut e,
        &pinned::all_of("o <- order(d$z)", &[("line", "has_xy(s, d$z[o], d$y[o])")]),
    );

    shapes_of(
        &mut e,
        &SCATTER_MATRIX,
        &[],
        &[("vars", &["x", "y", "z"])],
        &[],
    );
    holds(
        &mut e,
        &pinned::all_of(
            "v <- c('x', 'y', 'z'); \
             ok <- outer(1:3, 1:3, Vectorize(function(i, j) i == j || has_xy(s, d[[v[j]]], d[[v[i]]])))",
            &[("panels", "all(ok)")],
        ),
    );

    // correlations with their values printed, then standardised values
    let codes = "codes <- function(v, lim) { br <- seq(-lim, lim, length.out = 22); \
                 vapply(v, function(t) max(1, sum(br < t)) - 1, 0) }";
    shapes_of(
        &mut e,
        &HEATMAP,
        &[],
        &[("vars", &["x", "y", "z"])],
        &["numbers"],
    );
    holds(
        &mut e,
        &pinned::all_of(
            &format!(
                "{codes}; v <- d[c('x', 'y', 'z')]; \
                 R <- outer(1:3, 1:3, Vectorize(function(i, j) {{ a <- v[[i]] - mean(v[[i]]); \
                   b <- v[[j]] - mean(v[[j]]); sum(a * b) / sqrt(sum(a^2) * sum(b^2)) }})); \
                 img <- pick(s, 'C_image')[[1]]; txt <- pick(s, 'C_text')"
            ),
            &[
                ("colours", "same(as.vector(img[[3]]), codes(as.vector(R[, 3:1]), 1))"),
                (
                    "labels",
                    "any(vapply(txt, function(a) identical(a[[2]], sprintf('%.2f', R[, 3:1])), TRUE))",
                ),
            ],
        ),
    );
    shapes_of(
        &mut e,
        &HEATMAP,
        &[("show", "values")],
        &[("vars", &["x", "y", "z"])],
        &[],
    );
    holds(
        &mut e,
        &pinned::all_of(
            &format!(
                "{codes}; m <- sapply(d[c('x', 'y', 'z')], function(c) (c - mean(c)) / sd(c)); \
                 n <- nrow(m); lim <- max(abs(m)); img <- pick(s, 'C_image')[[1]]"
            ),
            &[(
                "colours",
                "same(as.vector(img[[3]]), codes(as.vector(t(m[n:1, ])), lim))",
            )],
        ),
    );
}
