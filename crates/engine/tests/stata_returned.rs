//! P2B.6, Stata's returned values: `e()` after each translated model, `r()`
//! after the r-class commands, `_b[]` and `_se[]`, and `display`,
//! `ereturn list` and `return list`, run in a real R.
//!
//! Every number is Stata 19's own, printed with `%24.17g` or listed by
//! `ereturn list` / `return list` on the same synthetic data (the P2B.6
//! notes have the probes). The display formats are checked against the
//! strings Stata printed for each width, and `e()` names RCraft does not
//! keep are checked to be refused, never left missing.
//!
//! Without a supported R, or without haven, dplyr, MASS and nnet, the test
//! is skipped with a note.

// Stata's numbers are kept exactly as it printed them
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
        "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE), requireNamespace(\"haven\", quietly = TRUE), \
         requireNamespace(\"MASS\", quietly = TRUE), requireNamespace(\"nnet\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: the R packages dplyr, haven, MASS and nnet are needed");
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
            "invisible(utils::capture.output({{{setup}}}))\ncat(format(c({show}), digits = 17), \"\\n\")"
        ),
    );
    let v: Vec<f64> = text
        .split_whitespace()
        .filter_map(|w| w.parse().ok())
        .collect();
    assert!(!v.is_empty(), "no number came back from {show}: {text}");
    v
}

fn fresh() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

/// The models data of `stata_models.rs`: the same seed, so the same rows
/// Stata 19 was run on.
fn fixture(e: &mut Engine) -> String {
    let dir = std::env::temp_dir().join(format!(
        "rcraft-stata-returned-{}-{}",
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

/// Why the last line was not translated.
fn refusal(dofile: &str) -> String {
    translate(&read(dofile))
        .lines
        .iter()
        .rev()
        .find_map(|l| l.result.as_ref().err().map(|e| e.reason.clone()))
        .unwrap_or_default()
}

/// Each value against Stata's, relative to its size.
fn near_stata(got: &[f64], stata: &[f64], tol: f64, what: &str) {
    assert_eq!(got.len(), stata.len(), "{what}: {got:?} against {stata:?}");
    for (i, (g, s)) in got.iter().zip(stata).enumerate() {
        assert!(
            (g - s).abs() <= tol * s.abs().max(1.0),
            "{what} [{i}]: {g} against Stata's {s}"
        );
    }
}

/// `e(name)` for each name, after the do-file `setup`.
fn e_values(e: &mut Engine, dir: &str, setup: &str, names: &[&str]) -> Vec<f64> {
    let code = script(&format!("use \"{dir}/models.dta\"\n{setup}\n"));
    ok(e, &code);
    let show: Vec<String> = names
        .iter()
        .map(|n| format!("stata_e[[\"{n}\"]]"))
        .collect();
    numbers(e, "0", &show.join(", "))
}

/// Whether each R condition holds (1) or not (0).
fn truths(e: &mut Engine, conds: &[&str]) -> Vec<f64> {
    let show: Vec<String> = conds
        .iter()
        .map(|c| format!("as.numeric(isTRUE({c}))"))
        .collect();
    numbers(e, "0", &show.join(", "))
}

/// `e()` after regress, as Stata 19's `ereturn list` shows it.
#[test]
fn e_after_regress_is_statas() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let names = [
        "N", "sum_w", "df_m", "df_r", "F", "r2", "rmse", "mss", "rss", "r2_a", "ll", "ll_0", "rank",
    ];
    near_stata(
        &e_values(&mut e, &dir, "regress y x w", &names),
        &[
            80.0,
            80.0,
            2.0,
            77.0,
            44.58311913646749,
            0.5366086348207253,
            2.047933000982293,
            373.9658405430359,
            322.9402773914502,
            0.5245724954654194,
            -169.3327135258075,
            -200.1000455570114,
            3.0,
        ],
        1e-12,
        "regress y x w",
    );
    let macros = [
        "stata_e$title == \"Linear regression\"",
        "stata_e$cmd == \"regress\"",
        "stata_e$vce == \"ols\"",
        "stata_e$cmdline == \"regress y x w\"",
        "stata_e$depvar == \"y\"",
        "stata_e$properties == \"b V\"",
    ];
    assert_eq!(truths(&mut e, &macros), vec![1.0; 6], "regress's macros");
    // clustered: N_clust, G - 1 and the Wald F; no title, as in Stata
    near_stata(
        &e_values(
            &mut e,
            &dir,
            "regress y x w, vce(cluster cl)",
            &["N_clust", "df_r", "F", "r2_a", "rmse"],
        ),
        &[
            8.0,
            7.0,
            64.66772412995202,
            0.5245724954654194,
            2.047933000982293,
        ],
        1e-12,
        "regress, vce(cluster cl)",
    );
    let macros = [
        "is.null(stata_e$title)",
        "stata_e$vce == \"cluster\"",
        "stata_e$clustvar == \"cl\"",
        "stata_e$vcetype == \"Robust\"",
    ];
    assert_eq!(
        truths(&mut e, &macros),
        vec![1.0; 4],
        "the clustered macros"
    );
    near_stata(
        &e_values(&mut e, &dir, "regress y x w, robust", &["F", "df_r"]),
        &[39.03993626684133, 77.0],
        1e-12,
        "regress, robust",
    );
    // weights: fw counts the copies; aw and pw scale the weights to sum to N
    let fw = "generate f = mod(_n, 3) + 1\nregress y x w [fw=f]";
    near_stata(
        &e_values(
            &mut e,
            &dir,
            fw,
            &[
                "N", "sum_w", "df_r", "F", "r2", "rmse", "mss", "rss", "r2_a", "ll", "ll_0",
            ],
        ),
        &[
            161.0,
            161.0,
            158.0,
            85.56421606619617,
            0.5199442388603963,
            2.109542118011241,
            761.5502636094559,
            703.1265357308098,
            0.5138675836560975,
            -347.1167704627442,
            -406.1919379920542,
        ],
        1e-12,
        "regress [fw=f]",
    );
    let macros = ["stata_e$wtype == \"fweight\"", "stata_e$wexp == \"= f\""];
    assert_eq!(truths(&mut e, &macros), vec![1.0; 2], "the weight macros");
    // a = _n / 3 is a float in Stata, so its sum is not quite 1080
    let aw = "generate a = _n / 3\nregress y x w [aw=a]";
    near_stata(
        &e_values(
            &mut e,
            &dir,
            aw,
            &["N", "sum_w", "F", "r2", "rmse", "mss", "rss", "ll", "ll_0"],
        ),
        &[
            80.0,
            1080.000000029802,
            45.62638514867598,
            0.5423552321669448,
            1.953678755061682,
            348.299110704646,
            293.8982722044111,
            -176.7788818420175,
            -208.045362317968,
        ],
        1e-9,
        "regress [aw=a]",
    );
    let pw = "generate a = _n / 3\nregress y x w [pw=a]";
    near_stata(
        &e_values(&mut e, &dir, pw, &["F", "ll", "ll_0"]),
        &[34.7558780448316, -165.563365051508, -196.8298455274585],
        1e-9,
        "regress [pw=a]",
    );
    let macros = ["stata_e$vce == \"robust\"", "!is.null(stata_e$title)"];
    assert_eq!(
        truths(&mut e, &macros),
        vec![1.0; 2],
        "pweights: robust, with a title as in Stata"
    );
}

/// `e()` after the likelihood models.
#[test]
fn e_after_the_likelihood_models_is_statas() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let names = [
        "rank",
        "N",
        "k",
        "k_eq",
        "k_dv",
        "converged",
        "ll",
        "ll_0",
        "df_m",
        "chi2",
        "p",
        "r2_p",
    ];
    near_stata(
        &e_values(&mut e, &dir, "logit yb x w", &names),
        &[
            3.0,
            80.0,
            3.0,
            1.0,
            1.0,
            1.0,
            -39.22338132495593,
            -55.4267718399777,
            2.0,
            32.40678103004353,
            9.18241488063e-08,
            0.2923387016260387,
        ],
        1e-8,
        "logit",
    );
    let macros = [
        "stata_e$chi2type == \"LR\"",
        "stata_e$vce == \"oim\"",
        "stata_e$title == \"Logistic regression\"",
    ];
    assert_eq!(truths(&mut e, &macros), vec![1.0; 3], "logit's macros");
    near_stata(
        &e_values(&mut e, &dir, "logit yb x w, robust", &["chi2", "p", "ll"]),
        &[20.08557619977539, 0.0000434983261514, -39.22338132495593],
        1e-8,
        "logit, robust (a Wald chi2)",
    );
    near_stata(
        &e_values(
            &mut e,
            &dir,
            "logit yb x w, vce(cluster cl)",
            &["N_clust", "chi2", "p"],
        ),
        &[8.0, 22.08816374396845, 0.0000159814501812],
        1e-8,
        "logit, vce(cluster cl)",
    );
    near_stata(
        &e_values(&mut e, &dir, "probit yb x w", &["ll", "ll_0", "p", "r2_p"]),
        &[
            -39.1348243655055,
            -55.4267718399777,
            8.40421419125e-08,
            0.2939364306026803,
        ],
        1e-8,
        "probit",
    );
    near_stata(
        &e_values(
            &mut e,
            &dir,
            "poisson cnt x",
            &["ll", "ll_0", "chi2", "p", "r2_p"],
        ),
        &[
            -152.7798404955019,
            -168.2959147417041,
            31.03214849240442,
            2.53789782849e-08,
            0.0921951924383657,
        ],
        1e-8,
        "poisson",
    );
    near_stata(
        &e_values(
            &mut e,
            &dir,
            "generate f = mod(_n, 3) + 1\npoisson cnt x [fw=f]",
            &["N", "ll", "ll_0", "chi2", "r2_p"],
        ),
        &[
            161.0,
            -305.1325926992846,
            -334.9939821350225,
            59.72277887147584,
            0.0891400772199603,
        ],
        1e-8,
        "poisson [fw=f]",
    );
    near_stata(
        &e_values(
            &mut e,
            &dir,
            "nbreg nb x",
            &[
                "rank", "k_eq", "ll", "ll_0", "rank0", "df_m", "p", "ll_c", "alpha", "k_aux",
                "r2_p",
            ],
        ),
        &[
            3.0,
            2.0,
            -146.3608674030971,
            -154.5689935844777,
            2.0,
            1.0,
            0.0000508473859417,
            -159.7054192774272,
            0.4816509245557584,
            1.0,
            0.0531033164610374,
        ],
        1e-6,
        "nbreg (the constant-only model is nbreg's own)",
    );
    near_stata(
        &e_values(
            &mut e,
            &dir,
            "ologit ord x w",
            &[
                "rank", "k_eq", "ll", "ll_0", "df_m", "p", "k_cat", "k_aux", "r2_p",
            ],
        ),
        &[
            4.0,
            3.0,
            -83.38134787443779,
            -87.49245845302663,
            2.0,
            0.0163895625180184,
            3.0,
            2.0,
            0.0469881707666956,
        ],
        1e-7,
        "ologit",
    );
    near_stata(
        &e_values(&mut e, &dir, "oprobit ord x w", &["ll", "p", "r2_p"]),
        &[-83.38704891548866, 0.0164832669393184, 0.0469230103957142],
        1e-7,
        "oprobit",
    );
    near_stata(
        &e_values(
            &mut e,
            &dir,
            "mlogit ord x w",
            &[
                "rank",
                "k",
                "ll",
                "ll_0",
                "df_m",
                "chi2",
                "p",
                "ibaseout",
                "baseout",
                "k_eq",
                "k_eq_model",
                "k_eq_base",
                "r2_p",
                "k_out",
            ],
        ),
        &[
            6.0,
            6.0,
            -80.02181177330152,
            -87.49245845302663,
            4.0,
            14.94129335945021,
            0.0048245402942209,
            1.0,
            1.0,
            3.0,
            3.0,
            1.0,
            0.0853861785554464,
            3.0,
        ],
        1e-7,
        "mlogit",
    );
    assert_eq!(
        truths(&mut e, &["stata_e$eqnames == \"1 2 3\""]),
        vec![1.0],
        "mlogit's equations"
    );
}

/// `_b[]`, `_se[]` and `e()` in expressions, against Stata's `display`.
#[test]
fn coefficients_in_expressions_are_statas() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let use_ = format!("use \"{dir}/models.dta\"\n");
    let code = script(&format!(
        "{use_}regress y x i.g\ngenerate double bx = _b[x] * x\ngenerate n80 = e(N)\n"
    ));
    ok(&mut e, &code);
    let show = "stata_bget(\"x\"), stata_bget(\"x\", se = TRUE), stata_bget(\"_cons\"), \
                stata_bget(\"1.g\"), stata_bget(\"1.g\", se = TRUE), stata_bget(\"0.g\"), \
                stata_bget(\"0.g\", se = TRUE), models$bx[1] / models$x[1], models$n80[1]";
    near_stata(
        &numbers(&mut e, "0", show),
        &[
            1.8314200732128889,
            0.21805036940171596,
            0.97039061660369219,
            0.77562454571890305,
            0.47818324692402581,
            0.0,
            0.0,
            1.8314200732128889,
            80.0,
        ],
        1e-12,
        "regress y x i.g: _b[], _se[], a base level is 0, as in Stata",
    );
    // Stata's own messages
    let (outcome, text) = run(
        &mut e,
        &script(&format!("{use_}regress y x\ngenerate z = _b[nosuch]\n")),
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("[nosuch] not found"), "{text}");
    let (outcome, text) = run(
        &mut e,
        &script(&format!("{use_}regress y x\ngenerate z = [y]_b[x]\n")),
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("equation [y] not found"), "{text}");
    // mlogit: an equation per outcome, the base one zeros (and the first)
    ok(&mut e, &script(&format!("{use_}mlogit ord x w\n")));
    near_stata(
        &numbers(
            &mut e,
            "0",
            "stata_bget(\"2:x\"), stata_bget(\"3:x\"), stata_bget(\"3:_cons\", se = TRUE), \
             stata_bget(\"x\"), stata_bget(\"1:x\")",
        ),
        &[
            0.98792055435843573,
            0.74124442684773262,
            0.29677199459671583,
            0.0,
            0.0,
        ],
        1e-7,
        "mlogit _b[2:x], [3]_b[x], _se[3:_cons], _b[x] (the base equation)",
    );
    ok(&mut e, &script(&format!("{use_}ologit ord x w\n")));
    near_stata(
        &numbers(
            &mut e,
            "0",
            "stata_bget(\"/cut1\"), stata_bget(\"/cut2\", se = TRUE)",
        ),
        &[-0.62940104652368734, 0.2425552757535259],
        1e-6,
        "ologit _b[/cut1], _se[/cut2]",
    );
    ok(&mut e, &script(&format!("{use_}nbreg nb x\n")));
    near_stata(
        &numbers(
            &mut e,
            "0",
            "stata_bget(\"/lnalpha\"), stata_eget(\"alpha\")",
        ),
        &[-0.73053565022100209, 0.48165092455575836],
        1e-6,
        "nbreg _b[/lnalpha], e(alpha)",
    );
}

/// `r()` after the r-class commands, as Stata 19's `return list` shows it.
#[test]
fn r_after_the_r_class_commands_is_statas() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let r_values = |e: &mut Engine, cmd: &str, names: &[&str]| -> Vec<f64> {
        let code = script(&format!("use \"{dir}/models.dta\"\n{cmd}\n"));
        ok(e, &code);
        let show: Vec<String> = names
            .iter()
            .map(|n| format!("stata_r[[\"{n}\"]]"))
            .collect();
        numbers(e, "0", &show.join(", "))
    };
    near_stata(
        &r_values(
            &mut e,
            "summarize y",
            &["N", "sum_w", "mean", "Var", "sd", "min", "max", "sum"],
        ),
        &[
            80.0,
            80.0,
            1.135731136069608,
            8.821596429550457,
            2.970117241717986,
            -6.061345365088699,
            9.181130339243705,
            90.85849088556864,
        ],
        1e-12,
        "summarize y",
    );
    // quietly still leaves r()
    near_stata(
        &r_values(
            &mut e,
            "quietly summarize y, detail",
            &[
                "skewness", "kurtosis", "p1", "p5", "p10", "p25", "p50", "p75", "p90", "p95", "p99",
            ],
        ),
        &[
            0.0729175641189797,
            2.945800768512033,
            -6.061345365088699,
            -3.869164305276796,
            -2.945146408485925,
            -0.9532353901444679,
            1.384290772100424,
            2.85956455876544,
            5.032766946910369,
            6.355480621373088,
            9.181130339243705,
        ],
        1e-12,
        "quietly summarize y, detail",
    );
    near_stata(
        &r_values(&mut e, "summarize y x", &["N", "mean", "Var", "min"]),
        &[80.0, -0.121475, 1.217525214556962, -3.891],
        1e-12,
        "summarize y x: the last variable's",
    );
    near_stata(
        &r_values(
            &mut e,
            "summarize x, meanonly",
            &["N", "sum_w", "sum", "mean", "min", "max"],
        ),
        &[80.0, 80.0, -9.718, -0.121475, -3.891, 2.063],
        1e-12,
        "summarize, meanonly",
    );
    near_stata(
        &r_values(&mut e, "tabulate g yb, chi2", &["N", "r", "c", "chi2", "p"]),
        &[80.0, 2.0, 2.0, 0.450281425891182, 0.5022013399924294],
        1e-12,
        "tabulate g yb, chi2",
    );
    near_stata(
        &r_values(&mut e, "quietly tabulate ord", &["N", "r"]),
        &[80.0, 3.0],
        0.0,
        "tabulate ord",
    );
    near_stata(
        &r_values(&mut e, "correlate y x w", &["N", "rho"]),
        &[80.0, 0.7102599004414356],
        1e-12,
        "correlate y x w",
    );
    near_stata(
        &r_values(
            &mut e,
            "ttest y, by(g)",
            &[
                "N_1",
                "N_2",
                "mu_1",
                "mu_2",
                "sd_1",
                "sd_2",
                "se_1",
                "se_2",
                "lb_1",
                "ub_2",
                "mu_diff",
                "lb_diff",
                "ub_diff",
                "t",
                "df_t",
                "p",
                "p_l",
                "p_u",
                "se",
                "sd",
                "level",
                "N_combined",
                "se_combined",
                "ub_combined",
            ],
        ),
        &[
            40.0,
            40.0,
            0.2913000534563532,
            1.980162218682863,
            2.82730017266337,
            2.900528622397172,
            0.4470354087301788,
            0.4586138432642771,
            -0.6129144087171012,
            2.907796275256678,
            -1.68886216522651,
            -2.963886355196214,
            -0.4138379752568051,
            -2.637021570036291,
            78.0,
            0.0100900980595278,
            0.0050450490297639,
            0.9949549509702361,
            0.6404430606167804,
            2.970117241717986,
            95.0,
            80.0,
            0.3320692026812795,
            1.796698350357512,
        ],
        1e-11,
        "ttest y, by(g)",
    );
    near_stata(
        &r_values(&mut e, "count if g == 1", &["N"]),
        &[40.0],
        0.0,
        "count",
    );
    near_stata(
        &r_values(
            &mut e,
            "regress y x w\ntest x w",
            &["drop", "df_r", "F", "df", "p"],
        ),
        &[0.0, 77.0, 44.58311913646748, 2.0, 1.37719542275e-13],
        1e-10,
        "test x w",
    );
    near_stata(
        &r_values(
            &mut e,
            "regress y x w\nlincom x + w",
            &["df", "ub", "lb", "p", "t", "level", "se", "estimate"],
        ),
        &[
            77.0,
            1.891952806385722,
            0.1332002953597375,
            0.024585754916523,
            2.292872356914284,
            95.0,
            0.4416192413935511,
            1.01257655087273,
        ],
        1e-11,
        "lincom x + w",
    );
    near_stata(
        &r_values(&mut e, "poisson cnt x\nmargins", &["N", "level"]),
        &[80.0, 95.0],
        0.0,
        "margins",
    );
    // after a likelihood model: chi2 and z, and no r(df), as in Stata
    near_stata(
        &r_values(
            &mut e,
            "quietly logit yb x w\ntest x w",
            &["drop", "chi2", "df", "p"],
        ),
        &[0.0, 18.15703265712155, 2.0, 0.0001140907551408],
        1e-8,
        "test after logit",
    );
    near_stata(
        &r_values(
            &mut e,
            "quietly logit yb x w\nlincom x + w",
            &["ub", "lb", "p", "z", "level", "se", "estimate"],
        ),
        &[
            0.6440306360209656,
            -1.411809518665888,
            0.4641852043108143,
            -0.7319727434274517,
            95.0,
            0.5244586561036476,
            -0.3838894413224614,
        ],
        1e-8,
        "lincom after logit",
    );
    let none = numbers(&mut e, "0", "as.numeric(is.null(stata_r[[\"df\"]]))");
    assert_eq!(none, vec![1.0], "no r(df) after a likelihood model");
    // or: the odds ratio, its bounds and its delta-method se
    near_stata(
        &r_values(
            &mut e,
            "quietly logit yb x w\nlincom x, or",
            &["ub", "lb", "p", "z", "se", "estimate"],
        ),
        &[
            7.881832457857859,
            1.938351888097059,
            0.0001392593283668,
            3.809480074616129,
            1.398696670738617,
            3.908678143100742,
        ],
        1e-8,
        "lincom x, or",
    );
}

/// Stata 19's `%w.0g` at every width from 5 to 20, as it printed them.
const G_GRID: &[(usize, &str, &str)] = &[
    (5, "1/3", " .333"),
    (5, "-1/3", "-.333"),
    (5, "1/7*1e-13", " 1.4e-14"),
    (5, "-1/7*1e-13", "-1.4e-14"),
    (5, "2/3*1e5", " 6.7e+04"),
    (5, "123456789012", " 1.2e+11"),
    (5, "-123456789012", "-1.2e+11"),
    (5, "1/7*1e-3", " 1.4e-04"),
    (5, "1/7*1e30", " 1.4e+29"),
    (5, "1.135731136069608", " 1.14"),
    (5, "0.00012345678912345", " 1.2e-04"),
    (6, "1/3", " .3333"),
    (6, "-1/3", "-.3333"),
    (6, "1/7*1e-13", " 1.4e-14"),
    (6, "-1/7*1e-13", "-1.4e-14"),
    (6, "2/3*1e5", " 6.7e+04"),
    (6, "123456789012", " 1.2e+11"),
    (6, "-123456789012", "-1.2e+11"),
    (6, "1/7*1e-3", " 1.4e-04"),
    (6, "1/7*1e30", " 1.4e+29"),
    (6, "1.135731136069608", " 1.136"),
    (6, "0.00012345678912345", " 1.2e-04"),
    (7, "1/3", " .33333"),
    (7, "-1/3", "-.33333"),
    (7, "1/7*1e-13", " 1.4e-14"),
    (7, "-1/7*1e-13", "-1.4e-14"),
    (7, "2/3*1e5", "  66667"),
    (7, "123456789012", " 1.2e+11"),
    (7, "-123456789012", "-1.2e+11"),
    (7, "1/7*1e-3", " .00014"),
    (7, "1/7*1e30", " 1.4e+29"),
    (7, "1.135731136069608", " 1.1357"),
    (7, "0.00012345678912345", " .00012"),
    (8, "1/3", " .333333"),
    (8, "-1/3", "-.333333"),
    (8, "1/7*1e-13", " 1.4e-14"),
    (8, "-1/7*1e-13", "-1.4e-14"),
    (8, "2/3*1e5", " 66666.7"),
    (8, "123456789012", " 1.2e+11"),
    (8, "-123456789012", "-1.2e+11"),
    (8, "1/7*1e-3", " .000143"),
    (8, "1/7*1e30", " 1.4e+29"),
    (8, "1.135731136069608", " 1.13573"),
    (8, "0.00012345678912345", " .000123"),
    (9, "1/3", " .3333333"),
    (9, "-1/3", "-.3333333"),
    (9, "1/7*1e-13", " 1.43e-14"),
    (9, "-1/7*1e-13", "-1.43e-14"),
    (9, "2/3*1e5", " 66666.67"),
    (9, "123456789012", " 1.23e+11"),
    (9, "-123456789012", "-1.23e+11"),
    (9, "1/7*1e-3", " .0001429"),
    (9, "1/7*1e30", " 1.43e+29"),
    (9, "1.135731136069608", " 1.135731"),
    (9, "0.00012345678912345", " .0001235"),
    (10, "1/3", " .33333333"),
    (10, "-1/3", "-.33333333"),
    (10, "1/7*1e-13", " 1.429e-14"),
    (10, "-1/7*1e-13", "-1.429e-14"),
    (10, "2/3*1e5", " 66666.667"),
    (10, "123456789012", " 1.235e+11"),
    (10, "-123456789012", "-1.235e+11"),
    (10, "1/7*1e-3", " .00014286"),
    (10, "1/7*1e30", " 1.429e+29"),
    (10, "1.135731136069608", " 1.1357311"),
    (10, "0.00012345678912345", " .00012346"),
    (11, "1/3", " .333333333"),
    (11, "-1/3", "-.333333333"),
    (11, "1/7*1e-13", " 1.4286e-14"),
    (11, "-1/7*1e-13", "-1.4286e-14"),
    (11, "2/3*1e5", " 66666.6667"),
    (11, "123456789012", " 1.2346e+11"),
    (11, "-123456789012", "-1.2346e+11"),
    (11, "1/7*1e-3", " .000142857"),
    (11, "1/7*1e30", " 1.4286e+29"),
    (11, "1.135731136069608", " 1.13573114"),
    (11, "0.00012345678912345", " .000123457"),
    (12, "1/3", " .3333333333"),
    (12, "-1/3", "-.3333333333"),
    (12, "1/7*1e-13", " 1.42857e-14"),
    (12, "-1/7*1e-13", "-1.42857e-14"),
    (12, "2/3*1e5", " 66666.66667"),
    (12, "123456789012", " 1.23457e+11"),
    (12, "-123456789012", "-1.23457e+11"),
    (12, "1/7*1e-3", " .0001428571"),
    (12, "1/7*1e30", " 1.42857e+29"),
    (12, "1.135731136069608", " 1.135731136"),
    (12, "0.00012345678912345", " .0001234568"),
    (13, "1/3", " .33333333333"),
    (13, "-1/3", "-.33333333333"),
    (13, "1/7*1e-13", " 1.428571e-14"),
    (13, "-1/7*1e-13", "-1.428571e-14"),
    (13, "2/3*1e5", " 66666.666667"),
    (13, "123456789012", " 1.234568e+11"),
    (13, "-123456789012", "-1.234568e+11"),
    (13, "1/7*1e-3", " .00014285714"),
    (13, "1/7*1e30", " 1.428571e+29"),
    (13, "1.135731136069608", " 1.1357311361"),
    (13, "0.00012345678912345", " .00012345679"),
    (14, "1/3", " .333333333333"),
    (14, "-1/3", "-.333333333333"),
    (14, "1/7*1e-13", " 1.4285714e-14"),
    (14, "-1/7*1e-13", "-1.4285714e-14"),
    (14, "2/3*1e5", " 66666.6666667"),
    (14, "123456789012", "  123456789012"),
    (14, "-123456789012", " -123456789012"),
    (14, "1/7*1e-3", " .000142857143"),
    (14, "1/7*1e30", " 1.4285714e+29"),
    (14, "1.135731136069608", " 1.13573113607"),
    (14, "0.00012345678912345", " .000123456789"),
    (15, "1/3", " .3333333333333"),
    (15, "-1/3", "-.3333333333333"),
    (15, "1/7*1e-13", " 1.42857143e-14"),
    (15, "-1/7*1e-13", "-1.42857143e-14"),
    (15, "2/3*1e5", " 66666.66666667"),
    (15, "123456789012", "   123456789012"),
    (15, "-123456789012", "  -123456789012"),
    (15, "1/7*1e-3", " .0001428571429"),
    (15, "1/7*1e30", " 1.42857143e+29"),
    (15, "1.135731136069608", "  1.13573113607"),
    (15, "0.00012345678912345", " .0001234567891"),
    (16, "1/3", " .33333333333333"),
    (16, "-1/3", "-.33333333333333"),
    (16, "1/7*1e-13", " 1.428571429e-14"),
    (16, "-1/7*1e-13", "-1.428571429e-14"),
    (16, "2/3*1e5", " 66666.666666667"),
    (16, "123456789012", "    123456789012"),
    (16, "-123456789012", "   -123456789012"),
    (16, "1/7*1e-3", " .00014285714286"),
    (16, "1/7*1e30", " 1.428571429e+29"),
    (16, "1.135731136069608", " 1.1357311360696"),
    (16, "0.00012345678912345", " .00012345678912"),
    (17, "1/3", " .333333333333333"),
    (17, "-1/3", "-.333333333333333"),
    (17, "1/7*1e-13", " 1.4285714286e-14"),
    (17, "-1/7*1e-13", "-1.4285714286e-14"),
    (17, "2/3*1e5", " 66666.6666666667"),
    (17, "123456789012", "     123456789012"),
    (17, "-123456789012", "    -123456789012"),
    (17, "1/7*1e-3", " .000142857142857"),
    (17, "1/7*1e30", " 1.4285714286e+29"),
    (17, "1.135731136069608", " 1.13573113606961"),
    (17, "0.00012345678912345", " .000123456789123"),
    (18, "1/3", " .3333333333333333"),
    (18, "-1/3", "-.3333333333333333"),
    (18, "1/7*1e-13", " 1.42857142857e-14"),
    (18, "-1/7*1e-13", "-1.42857142857e-14"),
    (18, "2/3*1e5", " 66666.66666666666"),
    (18, "123456789012", "      123456789012"),
    (18, "-123456789012", "     -123456789012"),
    (18, "1/7*1e-3", " .0001428571428571"),
    (18, "1/7*1e30", " 1.42857142857e+29"),
    (18, "1.135731136069608", " 1.135731136069608"),
    (18, "0.00012345678912345", " .0001234567891234"),
    (19, "1/3", " .33333333333333331"),
    (19, "-1/3", "-.33333333333333331"),
    (19, "1/7*1e-13", " 1.428571428571e-14"),
    (19, "-1/7*1e-13", "-1.428571428571e-14"),
    (19, "2/3*1e5", " 66666.666666666657"),
    (19, "123456789012", "       123456789012"),
    (19, "-123456789012", "      -123456789012"),
    (19, "1/7*1e-3", " .00014285714285714"),
    (19, "1/7*1e30", " 1.428571428571e+29"),
    (19, "1.135731136069608", " 1.1357311360696081"),
    (19, "0.00012345678912345", " .00012345678912345"),
    (20, "1/3", " .333333333333333315"),
    (20, "-1/3", "-.333333333333333315"),
    (20, "1/7*1e-13", " 1.4285714285714e-14"),
    (20, "-1/7*1e-13", "-1.4285714285714e-14"),
    (20, "2/3*1e5", "  66666.666666666657"),
    (20, "123456789012", "        123456789012"),
    (20, "-123456789012", "       -123456789012"),
    (20, "1/7*1e-3", " .000142857142857143"),
    (20, "1/7*1e30", " 1.4285714285714e+29"),
    (20, "1.135731136069608", " 1.13573113606960807"),
    (20, "0.00012345678912345", "  .00012345678912345"),
];

/// The other formats, as Stata printed each.
const FORMATS: &[(&str, &str, &str)] = &[
    ("%9.3g", "1/3", "     .333"),
    ("%9.3g", "12345", "    12345"),
    ("%9.3g", "0.00012345", "  .000123"),
    ("%9.3g", "123456789", " 1.23e+08"),
    ("%9.3g", "1e-7", " 1.00e-07"),
    ("%9.2g", "1/3", "      .33"),
    ("%9.2g", "1e-7", "  1.0e-07"),
    ("%12.5g", "123456789", "   123456789"),
    ("%12.5g", "1e-7", "  1.0000e-07"),
    ("%9.1g", "1/3", "      .33"),
    ("%9.3g", "2.5", "      2.5"),
    ("%9.3g", "1234567", "  1234567"),
    ("%9.3g", "12345678", " 1.23e+07"),
    ("%6.3g", "0.00012345", " 1.2e-04"),
    ("%9.3g", "0.0012345", "   .00123"),
    ("%9.5g", "1/3", "   .33333"),
    ("%9.8g", "1/3", " .3333333"),
    ("%18.10g", "1/3", "       .3333333333"),
    ("%9.3g", "-1234.5", "    -1235"),
    ("%-9.0g", "1/3", ".3333333 "),
    ("%8.0g", "123456", "  123456"),
    ("%8.0g", "1234567", " 1.2e+06"),
    ("%8.0g", "999999.5", " 1.0e+06"),
    ("%9.0g", "99999999.5", " 1.00e+08"),
    ("%9.0g", "0", "        0"),
    ("%9.0g", "-0.000000001", "-1.00e-09"),
    ("%9.2f", "1/3", "     0.33"),
    ("%5.1f", "12.345", " 12.3"),
    ("%8.0f", "2.5", "       2"),
    ("%8.0f", "3.5", "       4"),
    ("%-9.2f", "1/3", "0.33     "),
    ("%6.3f", "-0.5", "-0.500"),
    ("%09.4f", "1/3", "0000.3333"),
    ("%9.4fc", "12345.678", " 1.23e+04"),
    ("%12.0gc", "1234567.89", " 1,234,567.9"),
    ("%5.0f", "1e10", " 1.0e+10"),
    ("%8.3f", "-1234.5", "-1.2e+03"),
    ("%3.1f", "1234.5", "1234.5"),
    ("%12.4e", "1234.5678", "  1.2346e+03"),
    ("%10.3e", "-1234.5", "-1.235e+03"),
    ("%10.3e", "1e-300", " 1.00e-300"),
    ("%9.2e", "0", " 0.00e+00"),
    ("%9.2f", "NA_real_", "        ."),
];

/// `display`'s default, `%10.0g` without the padding, as Stata printed each.
const DEFAULTS: &[(&str, &str)] = &[
    ("1/3", ".33333333"),
    ("2/3*1e5", "66666.667"),
    ("123456", "123456"),
    ("1234567", "1234567"),
    ("1/7*1e30", "1.429e+29"),
    ("1/7*1e-13", "1.429e-14"),
    ("-123456789", "-1.235e+08"),
    ("1e15", "1.000e+15"),
    ("-.5", "-.5"),
    ("1e300", "1.00e+300"),
    ("99999999.5", "1.000e+08"),
    ("9999999.95", "9999999.9"),
    ("0.999999999", "1"),
    ("1e-5", ".00001"),
    ("1e-6", "1.000e-06"),
    ("123.456", "123.456"),
    ("0.000001234", "1.234e-06"),
    ("0.0001", ".0001"),
    ("0.00012345678", ".00012346"),
    ("99999999", "99999999"),
    ("12345678.9", "12345679"),
    ("0.99999999", ".99999999"),
    ("100000000", "1.000e+08"),
];

#[test]
fn display_formats_are_statas() {
    let Some(mut e) = engine() else { return };
    ok(&mut e, &translate(&read("display 1\n")).script());
    let mut calls = Vec::new();
    let mut want = Vec::new();
    for (w, v, s) in G_GRID {
        calls.push(format!("stata_fmt({v}, \"%{w}.0g\")"));
        want.push(s.to_string());
    }
    for (f, v, s) in FORMATS {
        calls.push(format!("stata_fmt({v}, \"{f}\")"));
        want.push(s.to_string());
    }
    for (v, s) in DEFAULTS {
        calls.push(format!("stata_fmt({v}, \"%10.0g\", trim = TRUE)"));
        want.push(s.to_string());
    }
    let text = ok(
        &mut e,
        &format!(
            "cat(paste0(\"[\", c({}), \"]\"), sep = \"\\n\")",
            calls.join(", ")
        ),
    );
    let got: Vec<&str> = text.lines().collect();
    assert_eq!(got.len(), want.len(), "{text}");
    let wrong: Vec<String> = got
        .iter()
        .zip(&want)
        .zip(&calls)
        .filter(|((g, w), _)| **g != format!("[{w}]"))
        .map(|((g, w), c)| format!("{c}: {g} against Stata's [{w}]"))
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} differ:\n{}",
        wrong.len(),
        want.len(),
        wrong.join("\n")
    );
}

/// `display` itself, and the lists, against what Stata printed.
#[test]
fn display_and_the_lists_print_as_stata() {
    let Some(mut e) = engine() else { return };
    let dir = fixture(&mut e);
    let code = script(&format!(
        "use \"{dir}/models.dta\"\n\
         display 1, 2\n\
         display \"a\" \"b\"\n\
         display \"a\" _n \"b\"\n\
         display _col(5) \"x\" _col(2) \"y\"\n\
         display _skip(3) \"x\"\n\
         display _dup(3) \"ab\"\n\
         display %9.4f 1/3 %9.4f 2/3\n\
         display %10s \"ab\"\n\
         display %-10s \"ab\" \"|\"\n\
         display x\n\
         display x[3]\n\
         display _N\n\
         display \"x is \" x \" and y is \" y\n\
         display 2+2 \"four\"\n\
         display \"len\" length(\"abc\")\n\
         display %6.2f 3.14159 \" pi\"\n\
         display 1/0\n\
         display as text \"t\" as result 1 as error \"e\"\n\
         regress y x i.g\n\
         display \"[\" _b[x] \"]\"\n\
         display %24.17g _b[x] \" \" %24.17g _se[1.g]\n\
         display e(N) \" \" e(cmd)\n"
    ));
    let text = ok(&mut e, &code);
    for want in [
        "1 2\n",
        "ab\n",
        "a\nb\n",
        "    xy\n",
        "   x\n",
        "ababab\n",
        "   0.3333   0.6667\n",
        "        ab\n",
        "ab        |\n",
        "-1.016\n",
        "-3.891\n",
        "80\n",
        "x is -1.016 and y is -6.0613454\n",
        "4four\n",
        "len3\n",
        "  3.14 pi\n",
        ".\n",
        "t1e\n",
        "[1.8314201]\n",
        // R's least squares agree with Stata's to 15 digits, not 17
        "      1.83142007321288",
        "80 regress\n",
    ] {
        assert!(text.contains(want), "{want:?} not in:\n{text}");
    }
    // the lists, line for line as Stata 19 printed them
    let code = script(&format!(
        "use \"{dir}/models.dta\"\nquietly regress y x w\nereturn list\nsummarize y\nreturn list\n"
    ));
    let text = ok(&mut e, &code);
    for want in [
        "\nscalars:\n                  e(N) =  80\n              e(sum_w) =  80\n               e(df_m) =  2\n",
        "               e(rank) =  3\n",
        "\nmacros:\n            e(cmdline) : \"regress y x w\"\n              e(title) : \"Linear regression\"\n",
        "\nmatrices:\n                  e(b) :  1 x 3\n                  e(V) :  3 x 3\n",
        "\nfunctions:\n             e(sample)   \n",
        "               r(mean) =  1.135731136069608\n                r(Var) =  8.821596429550457\n",
    ] {
        assert!(text.contains(want), "{want:?} not in:\n{text}");
    }
    // the long numbers: each line in Stata's layout, its value Stata's; the
    // 16th digit of a least-squares sum differs from one machine's R to the next
    let stata = [
        ("F", 44.58311913646749),
        ("r2", 0.5366086348207253),
        ("rmse", 2.047933000982293),
        ("mss", 373.9658405430359),
        ("rss", 322.9402773914502),
        ("r2_a", 0.5245724954654194),
        ("ll", -169.3327135258075),
        ("ll_0", -200.1000455570114),
    ];
    for (name, value) in stata {
        let key = format!("{:>22} =  ", format!("e({name})"));
        let line = text
            .lines()
            .find(|l| l.starts_with(&key))
            .unwrap_or_else(|| panic!("no line {key:?} in:\n{text}"));
        let shown = line.trim_start_matches(&key);
        let got: f64 = shown.parse().unwrap_or(f64::NAN);
        assert!(
            (got - value).abs() <= 1e-14 * value.abs().max(1.0) && shown.len() >= 16,
            "e({name}): {shown} against Stata's {value}"
        );
    }
}

/// Where Stata would hold a value RCraft does not keep, the translation says
/// so rather than leaving it missing.
#[test]
fn what_is_not_kept_is_refused_with_the_reason() {
    for (dofile, why) in [
        (
            "use s\ndisplay e(N)\n",
            "nothing before it in this do-file leaves e()",
        ),
        (
            "use s\ngenerate z = _b[x]\n",
            "no model is fitted before it",
        ),
        (
            "use s\nlogit y x\ndisplay e(ic)\n",
            "e(ic), which Stata leaves after logit, is not kept",
        ),
        ("use s\nlogit y x\ndisplay e(N_cdf)\n", "not kept"),
        ("use s\nregress y x\ndisplay e(b)\n", "is a matrix"),
        (
            "use s\nsummarize y\ndrop z\ndisplay r(mean)\n",
            "drop replaces r() in Stata",
        ),
        (
            "use s\nsummarize y\ndisplay r(skewness)\n",
            "r(skewness) after summarize is not kept",
        ),
        ("use s\ntabulate g h, chi2\ndisplay r(gamma)\n", "not kept"),
        (
            "use s\nsummarize y\nregress y x\ndisplay r(mean)\n",
            "regress replaces r()",
        ),
        (
            "use s\nregress y x\nglm y x\ndisplay e(N)\n",
            "glm was not translated",
        ),
        ("use s\nanova y g / h|g\ndisplay e(F)\n", "error terms"),
        ("use s\nanova y g h\ndisplay e(ss_1)\n", "not kept"),
        (
            "use s\nxtset id t\nxtreg y x, mle\ndisplay e(N)\n",
            "xtreg, mle",
        ),
        ("display %~9.4f 1/3\n", "format"),
    ] {
        let got = refusal(dofile);
        assert!(got.contains(why), "{dofile}: {got}");
    }
    // a name Stata does not have is missing, as in Stata
    let t = translate(&read("use s\nregress y x\ngenerate z = e(nosuch)\n"));
    assert!(t.lines.iter().all(|l| l.result.is_ok()), "{:?}", t.lines);
}
