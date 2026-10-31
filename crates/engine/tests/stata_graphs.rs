//! P2B.5: Stata's graph commands translated to ggplot2 and run in a real R.
//! What each graph shows (the bins, densities, smooths and bar heights) is
//! held to Stata 19 SE's own numbers on the models data (batch mode,
//! 2026-10-31, `%24.17g`), and every graph is built by ggplot2.
//!
//! Without a supported R, or without dplyr, haven and ggplot2, the test is
//! skipped with a note.

// Stata's numbers are kept exactly as it printed them (%24.17g)
#![allow(clippy::panic, clippy::excessive_precision)]

use std::time::{Duration, Instant};

use rcraft_engine::{Engine, Item, Outcome, Source};
use rcraft_rhost::{find_installs, Env, SessionConfig};
use rcraft_statatr::data_r::translate;
use rcraft_statatr::reader::read;

fn engine() -> Option<Engine> {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return None;
    };
    let mut e = Engine::start(SessionConfig::new(install)).ok()?;
    let (have, _) = run(
        &mut e,
        "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE), requireNamespace(\"haven\", quietly = TRUE), requireNamespace(\"ggplot2\", quietly = TRUE), \
         requireNamespace(\"MASS\", quietly = TRUE), requireNamespace(\"nnet\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: the R packages dplyr, haven, ggplot2, MASS and nnet are needed");
        return None;
    }
    Some(e)
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(120);
    loop {
        e.poll(Duration::from_millis(50));
        if !e.is_working() {
            break;
        }
        assert!(Instant::now() < limit, "the engine did not settle");
    }
}

fn run(e: &mut Engine, code: &str) -> (Outcome, String) {
    let id = e.submit(Source::Typed, code);
    assert!(id.is_ok(), "{id:?}");
    let id = id.unwrap_or_default();
    settle(e);
    let outcome = e
        .history()
        .entries()
        .last()
        .and_then(|h| h.outcome)
        .unwrap_or(Outcome::Error);
    let text = e
        .results()
        .blocks()
        .iter()
        .find(|b| b.command == id)
        .map(|b| {
            b.items
                .iter()
                .map(|i| match i {
                    Item::Text { text, .. } => text.clone(),
                    Item::Warning(w) => format!("warning: {w}"),
                    Item::Error { message, .. } => format!("error: {message}"),
                    Item::Notice(n) => n.clone(),
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

fn ok(e: &mut Engine, code: &str) -> String {
    let (outcome, text) = run(e, code);
    assert_eq!(outcome, Outcome::Ok, "{code}\n{text}");
    text
}

/// The numbers `show` prints, at full precision, with anything `setup` prints
/// thrown away.
fn numbers(e: &mut Engine, setup: &str, show: &str) -> Vec<f64> {
    let text = ok(
        e,
        &format!(
            "invisible(utils::capture.output({{{setup}}}))\ncat(format(c({show}), digits = 15), \"\\n\")"
        ),
    );
    let v: Vec<f64> = text
        .split_whitespace()
        .filter_map(|w| w.parse().ok())
        .collect();
    assert!(!v.is_empty(), "no number came back from {show}: {text}");
    v
}

fn at(v: &[f64], i: usize) -> f64 {
    v.get(i).copied().unwrap_or(f64::NAN)
}

/// Eighty rows with a binary, a count, an overdispersed count, an ordered
/// and an unordered outcome, made with a fixed seed.
fn fixture(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-graphs-{}-{}",
        std::process::id(),
        fresh()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let d = dir.display().to_string().replace('\\', "/");
    ok(
        e,
        &format!(
            "local({{
  set.seed(20261030)
  n <- 80
  x <- round(stats::rnorm(n), 3)
  w <- round(stats::runif(n, -1, 1), 3)
  g <- rep(0:1, length.out = n)
  cl <- rep(1:8, each = 10)
  expo <- round(stats::runif(n, 0.5, 3), 2)
  yb <- as.numeric(stats::runif(n) < stats::plogis(-0.3 + 0.8 * x - 0.6 * w))
  cnt <- stats::rpois(n, expo * exp(0.2 + 0.5 * x))
  nb <- stats::rnbinom(n, size = 1.5, mu = exp(0.6 + 0.4 * x))
  ord <- cut(0.9 * x - 0.5 * w + stats::rlogis(n), c(-Inf, -0.8, 0.6, Inf), labels = FALSE)
  e2 <- exp(0.8 + 0.5 * x)
  e3 <- exp(0.2 - 0.7 * x)
  u <- stats::runif(n)
  p1 <- 1 / (1 + e2 + e3)
  p2 <- e2 / (1 + e2 + e3)
  cat3 <- ifelse(u < p1, 1, ifelse(u < p1 + p2, 2, 3))
  y <- 1 + 2 * x - w + 0.5 * g + stats::rnorm(n) * (1 + abs(x))
  haven::write_dta(data.frame(id = 1:n, x = x, w = w, g = as.numeric(g), cl = as.numeric(cl),
    expo = expo, yb = yb, yb2 = 2 * yb, cnt = as.numeric(cnt), nb = as.numeric(nb),
    ord = as.numeric(ord), cat3 = cat3, y = y), '{d}/models.dta')
}})"
        ),
    );
    d
}
/// Translates a do-file and insists every line became R.
fn script(dofile: &str) -> String {
    let t = translate(&read(dofile));
    for l in &t.lines {
        assert!(
            l.result.is_ok(),
            "not translated: {}\n{:?}",
            l.stata,
            l.result
        );
    }
    t.script()
}
fn refusal(dofile: &str) -> String {
    translate(&read(dofile))
        .lines
        .iter()
        .find_map(|l| l.result.as_ref().err().map(|e| e.reason.clone()))
        .unwrap_or_default()
}
/// Each number within `tol` of Stata's, relative to its size.
fn near_stata(got: &[f64], stata: &[f64], tol: f64, what: &str) {
    assert_eq!(
        got.len(),
        stata.len(),
        "{what}: {got:?} against Stata's {stata:?}"
    );
    for (g, s) in got.iter().zip(stata) {
        assert!(
            (g - s).abs() <= tol * s.abs().max(1.0),
            "{what}: {g} against Stata's {s}"
        );
    }
}

/// The histograms' bins and heights, against Stata 19's `twoway__histogram_gen`.
#[test]
fn histogram_bins_are_statas() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let hist = |e: &mut Engine, cmd: &str, show: &str| -> Vec<f64> {
        let code = script(&format!("use \"{dir}/models.dta\"\n{cmd}\n"));
        ok(e, &code);
        numbers(e, "0", show)
    };
    // the default: int(min(sqrt(80), 10 log10 80)) = 8 bins from the minimum
    near_stata(
        &hist(
            &mut e,
            "histogram y",
            "nrow(stata_h), stata_h$width[1], stata_h$height[1:3], stata_h$mid[1:3]",
        ),
        &[
            8.0,
            1.9053094630415504,
            0.019681842997670174,
            0.039363685995340347,
            0.11809105426073074,
            -5.1086907386779785,
            -3.2033810615539551,
            -1.2980717420578003,
        ],
        1e-7,
        "histogram y (Stata keeps the midpoints as floats)",
    );
    near_stata(
        &hist(
            &mut e,
            "histogram y, bin(6) frequency",
            "stata_h$width[1], stata_h$height",
        ),
        &[2.5404126173887338, 5.0, 15.0, 21.0, 28.0, 8.0, 3.0],
        1e-12,
        "histogram y, bin(6) frequency",
    );
    near_stata(
        &hist(
            &mut e,
            "histogram y, width(2) start(-7) fraction",
            "nrow(stata_h), stata_h$height[1:3], stata_h$mid[1:3]",
        ),
        &[9.0, 0.0125, 0.0875, 0.15, -6.0, -4.0, -2.0],
        1e-12,
        "histogram y, width(2) start(-7) fraction",
    );
    near_stata(
        &hist(
            &mut e,
            "histogram cnt, discrete",
            "stata_h$width[1], stata_h$mid[1:4], stata_h$height[1:4]",
        ),
        &[1.0, 0.0, 1.0, 2.0, 3.0, 0.15, 0.2625, 0.2, 0.2],
        1e-12,
        "histogram cnt, discrete",
    );
    // Stata's own message for a start above the minimum
    let (outcome, text) = run(
        &mut e,
        &script(&format!(
            "use \"{dir}/models.dta\"\nhistogram y, start(-6)\n"
        )),
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(
        text.contains("option start() may not be larger than minimum of y"),
        "{text}"
    );
}

/// kdensity and lowess against Stata 19.
#[test]
fn kdensity_and_lowess_are_statas() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let use_ = format!("use \"{dir}/models.dta\"\n");
    let code = script(&format!("{use_}kdensity y, generate(kx kd)\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        "0",
        "attr(stata_k, \"bwidth\"), nrow(stata_k), models$kx[1:3], models$kd[1:3], models$kx[50], models$kd[50]",
    );
    near_stata(
        &v,
        &[
            1.058903982202364,
            50.0,
            -7.1202493472910628,
            -6.7659578438474481,
            -6.4116663404038334,
            0.0031675222896731842,
            0.0041411987698304256,
            0.0060150036063284463,
            10.240034321446069,
            0.0031675222896731829,
        ],
        1e-12,
        "kdensity y: Epanechnikov, Stata's bandwidth, 50 points",
    );
    // the other kernels, on the same bandwidth, at five points
    for (k, d2, d3) in [
        ("epan2", 0.046807585263268135, 0.13870984924204177),
        ("biweight", 0.043486593967932988, 0.13396805534098988),
        ("cosine", 0.037454881858707247, 0.093838885742441008),
        ("parzen", 0.04014130488731895, 0.12071876033456119),
        ("rectangle", 0.053120963699662647, 0.13575357389913786),
        ("triangle", 0.044795750958442064, 0.1332502800007315),
    ] {
        let code = script(&format!("{use_}kdensity y, kernel({k}) n(5) nograph\n"));
        ok(&mut e, &code);
        let v = numbers(&mut e, "0", "stata_k$x[2], stata_k$density[2:3]");
        near_stata(&v, &[-2.7801784301067798, d2, d3], 1e-12, k);
    }
    let code = script(&format!("{use_}kdensity y, at(x) nograph\n"));
    ok(&mut e, &code);
    near_stata(
        &numbers(&mut e, "0", "stata_k$density[1:2]"),
        &[0.093811829984077513, 0.10739772340714149],
        1e-12,
        "kdensity y, at(x)",
    );
    let code = script(&format!("{use_}lowess y x, generate(lw) nograph\n"));
    ok(&mut e, &code);
    near_stata(
        &numbers(&mut e, "0", "models$lw[1:3]"),
        &[
            -0.72079848927489887,
            1.5867222846565943,
            -4.2532378506529573,
        ],
        1e-12,
        "lowess y x (Stata's, not R's)",
    );
    // R's own lowess is a different curve
    let r = numbers(
        &mut e,
        "0",
        "suppressWarnings(stats::approx(stats::lowess(models$x, models$y), xout = models$x[1])$y)",
    );
    assert!(
        (r[0] + 0.72079848927489887).abs() > 1e-3,
        "R's lowess differs: {r:?}"
    );
}

/// The other graphs draw, show the numbers Stata would, and export.
#[test]
fn scatter_twoway_bar_and_export_draw() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let use_ = format!("use \"{dir}/models.dta\"\n");
    for cmd in [
        "histogram y, normal kdensity",
        "kdensity y, normal",
        "scatter y x, title(\"y on x\") msymbol(O)",
        "scatter y w x",
        "line y x, sort",
        "twoway (scatter y x) (lfit y x) (qfit y x) (lowess y x)",
        "twoway scatter y x if g == 0 || scatter y x if g == 1, legend(off)",
        "twoway (histogram y) (kdensity y)",
        "scatter y x, by(g)",
        "graph bar (mean) y (median) w, over(g)",
        "graph hbar y, over(g) over(yb)",
    ] {
        let code = script(&format!("{use_}{cmd}\n"));
        let text = ok(&mut e, &code);
        let built = numbers(
            &mut e,
            "0",
            "length(ggplot2::ggplot_build(stata_graph)$data)",
        );
        assert!(at(&built, 0) >= 1.0, "{cmd}: {text}");
    }
    // the bars are each group's statistic (Stata 19's collapse, as floats)
    let code = script(&format!(
        "{use_}graph bar (mean) y (median) y (sum) x, over(g)\n"
    ));
    ok(&mut e, &code);
    near_stata(
        &numbers(&mut e, "0", "stata_bar$value"),
        &[0.29130005, 1.9801622, 0.05356588, 2.1237951, -14.832, 5.114],
        1e-7,
        "graph bar statistics",
    );
    // lfit is the least-squares line
    let code = script(&format!("{use_}twoway (scatter y x) (lfit y x)\n"));
    ok(&mut e, &code);
    let v = numbers(
        &mut e,
        "f <- stata_fitline(models, \"y\", \"x\"); b <- stats::coef(stats::lm(y ~ x, data = models))",
        "f$y[1], b[1] + b[2] * f$x[1], f$y[100], b[1] + b[2] * f$x[100]",
    );
    near_stata(&v[..1], &v[1..2], 1e-12, "lfit at the left end");
    near_stata(&v[2..3], &v[3..4], 1e-12, "lfit at the right end");
    // graph export writes the file, and will not overwrite it without replace
    let png = format!("{dir}/g.png");
    let code = script(&format!("{use_}scatter y x\ngraph export \"{png}\"\n"));
    ok(&mut e, &code);
    assert!(std::path::Path::new(&png).exists(), "{png}");
    let (outcome, text) = run(&mut e, &code);
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("already exists"), "{text}");
    let code = script(&format!(
        "{use_}scatter y x\ngraph export \"{png}\", replace width(400)\n"
    ));
    ok(&mut e, &code);
}

#[test]
fn what_the_graphs_do_not_translate_says_why() {
    for (cmd, why) in [
        ("histogram y, bin(5) width(2)", "may not be combined"),
        ("twoway (area y x)", "twoway area"),
        ("graph pie y", "graph pie"),
        ("graph export g.png", "needs a graph"),
        ("scatter y x [fw=f]", "weights"),
        ("kdensity y, kernel(foo)", "kernel(foo)"),
        ("graph bar (iqr) y", "(iqr)"),
    ] {
        let got = refusal(&format!("use s\n{cmd}\n"));
        assert!(got.contains(why), "{cmd}: {got}");
    }
}
/// A number no other call in this run has had: the tests in a file run at
/// once, and each writing its data to the same folder let one read a file
/// another was still writing.
fn fresh() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
