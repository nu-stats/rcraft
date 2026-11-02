//! P2B.4: Stata's statistics commands translated to R and run in a real R.
//!
//! Each case is a short do-file over made-up data, with lines like
//! `= r(mean)` or `= _se[x1]` naming a returned value to check after the
//! command before them. The translation must run in R with every line
//! translated. When `RCRAFT_STATCRAFT` names a StatCraft executable (our
//! Stata-compatible program, whose estimates are checked against Stata's),
//! the same do-file runs there too, and every value checked must agree with
//! StatCraft's to about 8 significant digits. Without R the test is skipped
//! with a note.

#![allow(clippy::panic)]

use std::path::{Path, PathBuf};
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
    Engine::start(SessionConfig::new(install)).ok()
}

fn settle(e: &mut Engine) {
    let limit = Instant::now() + Duration::from_secs(180);
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
                    Item::Warning(w) => format!("warning: {w}\n"),
                    Item::Error { message, .. } => format!("error: {message}\n"),
                    Item::Notice(n) => format!("{n}\n"),
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

fn slash(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

/// The made-up dataset, written by R with haven: a few continuous
/// variables, two factors, a binary and a count outcome, clusters, an
/// exposure, and some missing values.
fn fixture(dir: &str) -> String {
    format!(
        "local({{
  set.seed(20261029)
  n <- 80
  g <- sample(1:3, n, TRUE); h <- sample(0:1, n, TRUE); cl <- sample(1:9, n, TRUE)
  x1 <- round(rnorm(n, 10, 3), 2); x2 <- round(runif(n, 0, 5), 2)
  y <- round(1 + 0.5 * x1 - 0.8 * x2 + g + 2 * h + rnorm(n), 3)
  b <- as.numeric(0.3 * x1 - 0.5 * x2 - 2 + rnorm(n) > 0)
  cnt <- rpois(n, exp(0.1 * x1 - 0.2 * x2))
  t <- round(runif(n, 1, 3), 2)
  oc <- as.numeric(cut(0.3 * x1 - 0.6 * x2 + rnorm(n), c(-Inf, 0, 1.5, 3, Inf)))
  mc <- 1 + (x2 + rnorm(n) > 2) + (x2 + rnorm(n) > 3.5)
  pid <- rep(1:16, each = 5); year <- rep(1:5, 16); ti <- rep(sample(0:1, 16, TRUE), each = 5)
  z1 <- round(rnorm(n), 3); z2 <- round(rnorm(n), 3); u <- rnorm(n)
  xe <- round(0.6 * z1 + 0.3 * z2 + u + rnorm(n), 3)
  yiv <- round(1 + 0.5 * xe + 0.3 * x2 + u + rnorm(n), 3)
  st <- round(rexp(n, exp(0.2 * x2 - 0.05 * x1)) * 10, 1) + 0.1
  died <- rbinom(n, 1, 0.7)
  cnb <- rnbinom(n, size = 1.5, mu = exp(0.1 * x1 - 0.2 * x2 + 0.5))
  x1[c(5, 17)] <- NA; y[9] <- NA; g[30] <- NA
  haven::write_dta(data.frame(id = 1:n, y, x1, x2, g, h, cl, b, cnt, t, oc, mc, pid, year, ti, z1, z2, xe, yiv, st, died, cnb),
                   file.path('{dir}', 'study.dta'))
}})"
    )
}

/// The cases: a name and the commands, with `= value` lines to check.
const CASES: &[(&str, &str)] = &[
    (
        "summarize",
        "summarize y x1\n= r(N)\n= r(mean)\n= r(sd)\n= r(min)\n= r(max)\n= r(sum)\n\
         summarize y, detail\n= r(p50)\n= r(p25)\n= r(p99)\n= r(p1)\n= r(skewness)\n= r(kurtosis)\n= r(Var)\n\
         summarize x1 if g == 2\n= r(mean)\n= r(N)\nsummarize x2 in 1/20, meanonly\n= r(mean)",
    ),
    (
        "tabulate",
        "tabulate g\n= r(N)\n= r(r)\ntabulate g h, chi2 lrchi2\n= r(chi2)\n= r(p)\n= r(chi2_lr)\n= r(p_lr)\n= r(r)\n= r(c)\n\
         tabulate g, missing\n= r(N)\n= r(r)\n\
         tabulate h b, V exact\n\
         ! stopifnot(abs(stata_r$p_exact - stats::fisher.test(table(study$h, study$b))$p.value) < 1e-12)\n\
         ! stopifnot(abs(abs(stata_r$CramersV) - sqrt(stats::chisq.test(table(study$h, study$b), correct = FALSE)$statistic / 80)) < 1e-12)",
    ),
    (
        "correlate",
        "correlate y x1 x2\n= r(N)\n= r(rho)\ncorrelate y x1, covariance\n= r(cov_12)\n= r(Var_1)\n= r(Var_2)",
    ),
    (
        "ttest",
        "ttest y == 10\n= r(t)\n= r(p)\n= r(se)\n= r(p_l)\nttest y, by(h)\n= r(t)\n= r(df_t)\n= r(mu_1)\n= r(sd_2)\n\
         ttest y, by(h) unequal\n= r(t)\n= r(df_t)\n= r(p)\nttest y, by(h) welch\n= r(df_t)\n\
         ttest x1 == x2\n= r(t)\n= r(p)\n= r(N_1)\nttest x1 == x2, unpaired\n= r(t)\n= r(df_t)",
    ),
    (
        "regress",
        "regress y x1 x2\n= _b[x1]\n= _se[x1]\n= _b[_cons]\n= _se[_cons]\n= e(N)\n= e(r2)\n= e(r2_a)\n= e(rmse)\n= e(F)\n= e(ll)\n= e(mss)\n= e(rss)\n= e(df_r)\n\
         regress y x1 if h == 1\n= _b[x1]\n= e(N)\nregress y x1 x2, noconstant\n= _b[x2]\n= _se[x2]\n= e(r2)\n= e(F)",
    ),
    (
        "regress_vce",
        "regress y x1 x2, robust\n= _se[x1]\n= _se[_cons]\n= e(F)\n\
         regress y x1 x2, vce(cluster cl)\n= _se[x1]\n= _se[x2]\n= e(F)\n= e(df_r)\n= e(N_clust)",
    ),
    (
        "factor_variables",
        "regress y x1 i.g##i.h\n= _b[2.g]\n= _se[3.g#1.h]\n= _b[1.h]\n= e(df_m)\n\
         regress y i.g#c.x1\n= _b[1b.g#c.x1]\n= _b[3.g#c.x1]\n= _se[_cons]\n\
         regress y ib3.g x2\n= _b[1.g]\n= _se[1.g]\nregress y c.x1##c.x1\n= _b[c.x1#c.x1]\n= _se[x1]",
    ),
    (
        "collinear",
        "gen double x3 = 2 * x1\nregress y x1 x3 x2\n= _b[x3]\n= _b[x2]\n= e(df_m)\nlogit b x1 x3 x2\n= _b[x1]\n= _se[x1]",
    ),
    (
        "logit",
        "logit b x1 x2\n= _b[x1]\n= _se[x1]\n= _b[_cons]\n= e(ll)\n= e(ll_0)\n= e(chi2)\n= e(r2_p)\n= e(N)\n\
         logit b x1 i.g, vce(robust)\n= _se[x1]\n= _se[3.g]\n= e(chi2)\nlogit b x1, vce(cluster cl)\n= _se[x1]\n= e(chi2)",
    ),
    (
        "probit",
        "probit b x1 x2\n= _b[x1]\n= _se[x1]\n= _se[_cons]\n= e(ll)\n= e(chi2)\nprobit b x1 x2, robust\n= _se[x2]",
    ),
    (
        "poisson",
        "poisson cnt x1 x2\n= _b[x1]\n= _se[x1]\n= e(ll)\n= e(chi2)\n= e(r2_p)\n\
         poisson cnt x1, vce(robust)\n= _se[x1]\n\
         poisson cnt x1, exposure(t)\n\
         ! stopifnot(isTRUE(all.equal(unname(stata_e$b), unname(coef(glm(cnt ~ x1 + offset(log(t)), poisson, study)))[c(2, 1)], tolerance = 1e-7)))",
    ),
    (
        "predict",
        "regress y x1 x2\npredict yhat\npredict double res, residuals\npredict se, stdp\n\
         summarize yhat\n= r(mean)\n= r(sd)\n= r(N)\nsummarize res\n= r(sd)\n= r(N)\nsummarize se\n= r(mean)\n\
         logit b x1\npredict p\npredict pxb, xb\nsummarize p\n= r(mean)\n= r(sd)\nsummarize pxb\n= r(mean)\n\
         poisson cnt x1\npredict n\nsummarize n\n= r(mean)",
    ),
    (
        "test_lincom",
        "regress y x1 x2 i.g\ntest x1\n= r(F)\n= r(p)\ntest x1 x2\n= r(F)\n= r(df)\ntest x1 = x2\n= r(F)\n\
         test x1 = 0.5\n= r(F)\ntest _b[2.g] = _b[3.g]\n= r(F)\n= r(p)\n\
         test (x1 = 0.5) (_cons = 3)\n= r(F)\n= r(df)\ntest 2.g 3.g\n= r(F)\n\
         lincom x1 + 2*x2\n= r(estimate)\n= r(se)\n= r(p)\n\
         lincom _b[3.g] - _b[2.g] + 1\n= r(estimate)\n= r(se)\nlogit b x1 x2\ntest x1 x2\n= r(chi2)\n= r(p)\nlincom x1 - x2\n= r(estimate)\n= r(se)\n= r(z)",
    ),
];

/// The cases of the second part: models with more than one equation or
/// their own settings.
const MORE_CASES: &[(&str, &str)] = &[
    (
        "nbreg",
        "nbreg cnb x1 x2\n= _b[x1]\n= _se[x1]\n= _se[_cons]\n= e(alpha)\n= e(ll)\n= e(ll_0)\n= e(chi2)\n= e(chi2_c)\n\
         nbreg cnb x1, vce(robust)\n= _se[x1]\npredict nb\nsummarize nb\n= r(mean)",
    ),
    (
        "ordered",
        "ologit oc x1 x2\n= _b[x1]\n= _se[x2]\n= _b[/cut1]\n= _se[/cut3]\n= e(ll)\n= e(ll_0)\n= e(chi2)\n\
         oprobit oc x1 x2\n= _b[x1]\n= _se[/cut2]\n= e(ll)\nologit oc x1, vce(robust)\n= _se[x1]",
    ),
    (
        "mlogit",
        "mlogit mc x1 x2\n= _b[2:x1]\n= _se[3:x2]\n= _b[3:_cons]\n= e(ll)\n= e(chi2)\n= e(k_out)\n\
         mlogit mc x2, baseoutcome(3)\n= _b[1:x2]\n= _se[2:_cons]",
    ),
    (
        "xtreg",
        "xtset pid year\nxtreg y x1 x2, fe\n= _b[x1]\n= _se[x1]\n= _b[_cons]\n= e(sigma_u)\n= e(sigma_e)\n= e(rho)\n= e(r2_w)\n= e(r2_b)\n= e(r2_o)\n= e(F)\n= e(F_f)\n= e(corr)\n\
         xtreg y x1 x2 ti, fe\n= _b[x2]\n= e(df_m)\n\
         xtreg y x1 x2, re\n= _b[x1]\n= _se[x1]\n= _se[_cons]\n= e(sigma_u)\n= e(chi2)\n= e(r2_o)\n= e(Tbar)\n\
         xtreg y x1 x2, fe vce(robust)\n= _se[x1]\n= e(F)\nxtreg y x1 x2 ti, re vce(robust)\n= _se[x2]\n= _se[ti]",
    ),
    (
        "ivregress",
        "ivregress 2sls yiv x2 (xe = z1 z2)\n= _b[xe]\n= _se[xe]\n= _se[_cons]\n= e(r2)\n= e(rmse)\n= e(chi2)\n\
         ivregress 2sls yiv x2 (xe = z1 z2), small\n= _se[xe]\n= e(F)\n\
         ivregress 2sls yiv x2 (xe = z1 z2), vce(robust)\n= _se[x2]",
    ),
    (
        "anova",
        "anova y g##h\n= e(ss_1)\n= e(df_1)\n= e(F_1)\n= e(ss_2)\n= e(ss_3)\n= e(F_3)\n= e(mss)\n= e(rss)\n= e(F)\n= e(r2)\n= e(rmse)\n= _b[2.g#1.h]\n\
         anova y g c.x2\n= e(ss_1)\n= e(ss_2)\n= e(F_2)",
    ),
    (
        "margins",
        "regress y x1 x2 i.g\nmargins, dydx(x1)\n= el(r(b),1,1)\n= el(r(V),1,1)\nmargins g\n= el(r(b),1,2)\n= el(r(V),3,3)\n\
         logit b x1 x2 i.h\nmargins, dydx(x1 h)\n= el(r(b),1,1)\n= el(r(V),1,1)\n= el(r(b),1,2)\n= el(r(V),2,2)\n\
         margins h\n= el(r(b),1,2)\n= el(r(V),1,1)\nmargins, at(x1=(5 10 15))\n= el(r(b),1,3)\n= el(r(V),2,2)\n\
         margins, at(x2=2) atmeans\n= el(r(b),1,1)\n= el(r(V),1,1)\npoisson cnt x1 x2\nmargins, dydx(x2)\n= el(r(b),1,1)\n= el(r(V),1,1)\n= r(N)",
    ),
    (
        "estat",
        "regress y x1 x2\nestat ic\n= el(r(S),1,5)\n= el(r(S),1,6)\nlogit b x1\nestat ic\n= el(r(S),1,2)\n= el(r(S),1,6)",
    ),
    (
        "stcox",
        "stset st, failure(died)\nstcox x1 x2\n= _b[x1]\n= _se[x1]\n= e(ll)\n= e(ll_0)\n= e(chi2)\n= e(N_fail)\n\
         stcox x2 i.h, nohr\n= _b[1.h]\n= _se[1.h]",
    ),
];

/// A check as R: `r(k)`, `e(k)`, `_b[n]` and `_se[n]`.
fn r_value(check: &str) -> String {
    let inner = |s: &str, open: &str, close: char| {
        s.strip_prefix(open)
            .and_then(|r| r.strip_suffix(close))
            .map(str::to_string)
    };
    if let Some(args) = inner(check, "el(", ')') {
        // el(r(b),1,2): an element of a returned matrix
        let mut it = args.rsplitn(3, ',');
        let (j, i, m) = (
            it.next().unwrap_or("1"),
            it.next().unwrap_or("1"),
            it.next().unwrap_or(""),
        );
        format!("{}[{i}, {j}]", r_value(m.trim()))
    } else if let Some(k) = inner(check, "r(", ')') {
        format!("stata_r[[\"{k}\"]]")
    } else if let Some(k) = inner(check, "e(", ')') {
        format!("stata_e[[\"{k}\"]]")
    } else if let Some(k) = inner(check, "_b[", ']') {
        format!("stata_e$b[[\"{k}\"]]")
    } else if let Some(k) = inner(check, "_se[", ']') {
        format!("sqrt(stata_e$V[[\"{k}\", \"{k}\"]])")
    } else {
        panic!("a check must be r(), e(), _b[] or _se[]: {check}")
    }
}

/// The numbers after `CHECK` in a log (`.` is missing).
fn checks(text: &str) -> Vec<Option<f64>> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("CHECK"))
        .map(|v| v.trim().parse::<f64>().ok())
        .collect()
}

#[test]
fn statistics_commands_translate_and_run_like_stata() {
    let Some(mut e) = engine() else { return };
    settle(&mut e);
    let (have, _) = run(
        &mut e,
        "stopifnot(requireNamespace(\"dplyr\", quietly = TRUE), requireNamespace(\"haven\", quietly = TRUE))",
    );
    if have != Outcome::Ok {
        eprintln!("skipped: the R packages dplyr and haven are needed");
        return;
    }
    let dir: PathBuf = std::env::temp_dir().join(format!("rcraft-p2b4-{}", std::process::id()));
    assert!(std::fs::create_dir_all(&dir).is_ok());
    let d = slash(&dir);
    let (made, text) = run(&mut e, &fixture(&d));
    assert_eq!(made, Outcome::Ok, "{text}");
    let statcraft = std::env::var("RCRAFT_STATCRAFT").ok().map(PathBuf::from);
    if statcraft.is_none() {
        eprintln!("note: set RCRAFT_STATCRAFT to a statcraft executable to compare with it");
    }
    let (_, _) = run(
        &mut e,
        "stata_check <- function(x) cat(\"CHECK\", if (is.null(x) || length(x) != 1 || is.na(x)) \".\" else sprintf(\"%.15g\", x), \"\\n\")",
    );

    let mut wrong = Vec::new();
    for (name, body) in CASES.iter().chain(MORE_CASES) {
        // the commands, and after which of them each check comes
        let mut commands = vec![format!("use \"{d}/study.dta\", clear")];
        let mut after: Vec<(usize, String)> = Vec::new();
        // R-only checks (`! stopifnot(...)`), for what StatCraft does not do
        let mut r_only: Vec<(usize, String)> = Vec::new();
        for line in body.lines() {
            let line = line.trim();
            if let Some(c) = line.strip_prefix('=') {
                after.push((commands.len(), c.trim().to_string()));
            } else if let Some(r) = line.strip_prefix('!') {
                r_only.push((commands.len(), r.trim().to_string()));
            } else {
                commands.push(line.to_string());
            }
        }
        // R
        let t = translate(&read(&format!("{}\n", commands.join("\n"))));
        let not: Vec<String> = t
            .lines
            .iter()
            .filter_map(|l| {
                l.result
                    .as_ref()
                    .err()
                    .map(|e| format!("{} -> {}", l.stata, e.reason))
            })
            .collect();
        if !not.is_empty() || t.lines.len() != commands.len() {
            wrong.push(format!("{name}: not translated: {}", not.join("; ")));
            continue;
        }
        let mut script = t.helpers.join("\n");
        script.push('\n');
        for (i, l) in t.lines.iter().enumerate() {
            if let Ok(r) = &l.result {
                script.push_str(&r.code);
                script.push('\n');
            }
            for (_, c) in after.iter().filter(|(k, _)| *k == i + 1) {
                script.push_str(&format!("stata_check({})\n", r_value(c)));
            }
            for (_, r) in r_only.iter().filter(|(k, _)| *k == i + 1) {
                script.push_str(r);
                script.push('\n');
            }
        }
        let (ran, text) = run(&mut e, &script);
        if ran != Outcome::Ok {
            wrong.push(format!("{name}: R stopped: {text}\n{script}"));
            continue;
        }
        let ours = checks(&text);
        if ours.len() != after.len() {
            wrong.push(format!(
                "{name}: {} checks printed, {} asked for\n{text}",
                ours.len(),
                after.len()
            ));
            continue;
        }
        // StatCraft, when asked
        let Some(sc) = &statcraft else { continue };
        let mut dofile = String::new();
        for (i, c) in commands.iter().enumerate() {
            dofile.push_str(c);
            dofile.push('\n');
            for (_, chk) in after.iter().filter(|(k, _)| *k == i + 1) {
                dofile.push_str(&format!("display \"CHECK \" %21.15g {chk}\n"));
            }
        }
        let path = dir.join(format!("{name}.do"));
        assert!(std::fs::write(&path, &dofile).is_ok());
        let status = std::process::Command::new(sc)
            .arg("run")
            .arg(&path)
            .current_dir(&dir)
            .status();
        let log = std::fs::read_to_string(dir.join(format!("{name}.log"))).unwrap_or_default();
        if !status.is_ok_and(|s| s.success()) {
            wrong.push(format!("{name}: StatCraft stopped:\n{log}"));
            continue;
        }
        let theirs: Vec<Option<f64>> = log
            .lines()
            .filter(|l| !l.starts_with(". display"))
            .filter_map(|l| l.trim().strip_prefix("CHECK"))
            .map(|v| v.trim().parse::<f64>().ok())
            .collect();
        if theirs.len() != after.len() {
            wrong.push(format!(
                "{name}: StatCraft printed {} checks of {}\n{log}",
                theirs.len(),
                after.len()
            ));
            continue;
        }
        for (((_, c), a), b) in after.iter().zip(&theirs).zip(&ours) {
            let same = match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => (a - b).abs() <= 1e-8 * a.abs().max(b.abs()) + 1e-12,
                _ => false,
            };
            if !same {
                wrong.push(format!("{name}: {c}: StatCraft {a:?} | R {b:?}"));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n\n"));
}
