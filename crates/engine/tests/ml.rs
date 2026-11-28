//! Statistics > Machine learning model (P6.12) end to end against a real R
//! (skipped with a note when none is found, or when tidymodels is not
//! installed). The dialog's code is built from its spec and run on made-up
//! data (120 rows: a numeric outcome y driven by x1, x2 and a three-level
//! factor g, and a logical outcome hit; a copy with some values missing).
//! There is no Stata analogue, so every number is checked by hand against R
//! written out in plain code, on a fixed split:
//!
//! - the split's rows are `sample(n, floor(n * share))` with the same seed
//!   (rsample draws them so), the test rows are the rest, and the data are
//!   left as they were; rows missing a value are left out before the split;
//! - the linear fit equals `lm` on the training rows, its predictions equal
//!   `predict(lm)` on the test rows, and the RMSE and R-squared printed equal
//!   `sqrt(mean((y - yhat)^2))` and `cor(y, yhat)^2`;
//! - the logistic fit equals `glm(binomial)` on the training rows, and the
//!   accuracy and the ROC AUC printed equal the share of right classes and
//!   the rank formula (Mann-Whitney) for the first level as the event;
//! - the stratified split keeps each stratum's share, and splits each
//!   stratum without overlap;
//! - the random forest (ranger) and the penalty tuning (glmnet) need packages
//!   tidymodels does not bring: with them missing the dialog stops and names
//!   the package; with them installed the forest equals `ranger::ranger` with
//!   the seed parsnip draws after the split, and the tuning (the folds, each
//!   penalty's mean RMSE, the best penalty, the final coefficients and
//!   predictions) equals glmnet fitted by hand as parsnip fits it: glmnet's
//!   own path of penalties, each penalty read off it by interpolation.
//!
//! The test records the R and tidymodels versions it ran with (AGENTS.md
//! section 5), and checks R is the one `ENVIRONMENT` pins.

use std::time::{Duration, Instant};

use rcraft_catalog::ml::ML;
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

fn run(e: &mut Engine, source: Source, code: &str) -> (Outcome, String) {
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
                    Item::Notice(n) => n.clone(),
                    Item::Plot { .. } => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    (outcome, text)
}

/// The R and tidymodels versions, recorded, and whether ranger and glmnet are
/// installed; `None` when tidymodels is missing.
fn versions(e: &mut Engine) -> Option<(bool, bool)> {
    let (outcome, text) = run(
        e,
        Source::Typed,
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         if (requireNamespace('tidymodels', quietly = TRUE)) as.character(packageVersion('tidymodels')) \
         else 'none', requireNamespace('ranger', quietly = TRUE), \
         requireNamespace('glmnet', quietly = TRUE))",
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let mut words = text.split_whitespace();
    let (r, tm) = (words.next().unwrap_or(""), words.next().unwrap_or("none"));
    let ranger = words.next() == Some("TRUE");
    let glmnet = words.next() == Some("TRUE");
    if tm == "none" {
        eprintln!("skipped: tidymodels is not installed");
        return None;
    }
    let pinned = include_str!("../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    eprintln!(
        "ran with R {r} and tidymodels {tm} (ENVIRONMENT pins R {pinned}); \
         ranger installed: {ranger}, glmnet installed: {glmnet}"
    );
    if r != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
    Some((ranger, glmnet))
}

/// 120 rows, rounded to 6 decimals. dn is the same with 9 values of x2 and 5
/// of y missing (different rows).
const MAKE: &str = "d <- local({ \
     set.seed(612); n <- 120; \
     x1 <- rnorm(n); x2 <- runif(n, 0, 3); g <- factor(sample(c('a', 'b', 'c'), n, TRUE)); \
     y <- 1 + 2 * x1 - x2 + 0.5 * (g == 'b') + rnorm(n); \
     hit <- rbinom(n, 1, plogis(0.3 + x1 - 0.5 * x2)) == 1; \
     data.frame(y = round(y, 6), x1 = round(x1, 6), x2 = round(x2, 6), g, hit) }); \
     d0 <- d; \
     dn <- local({ k <- d; set.seed(5); k$x2[sample(120, 9)] <- NA; k$y[sample(120, 5)] <- NA; k })";

fn context() -> Context {
    let col = |name: &str, kind| ColumnInfo {
        name: name.into(),
        kind,
    };
    Context {
        dataset: "d".into(),
        frames: Vec::new(),
        columns: vec![
            col("y", VarKind::Number),
            col("x1", VarKind::Number),
            col("x2", VarKind::Number),
            col("g", VarKind::Factor),
            col("hit", VarKind::Logical),
        ],
    }
}

/// Runs the dialog on `dataset`: `set` gives text boxes and choices, `lists`
/// the variable lists, `on` the check boxes to turn on.
fn dialog(
    e: &mut Engine,
    dataset: &str,
    set: &[(&str, &str)],
    lists: &[(&str, &[&str])],
    on: &[&str],
) -> (Outcome, String) {
    let spec: &Spec = &ML;
    let mut ctx = context();
    ctx.dataset = dataset.into();
    let mut values = spec.defaults(&ctx);
    for (id, text) in set {
        values.set_text(id, *text);
    }
    for (id, names) in lists {
        values.set_list(id, names.iter().map(|n| n.to_string()).collect());
    }
    for w in spec.widgets {
        if let Widget::Check { id, .. } = w {
            if on.contains(id) {
                values.set_flag(id, true);
            }
        }
    }
    let code = spec.generate(&ctx, &values, Style::Base);
    assert!(code.is_ok(), "{code:?}");
    let code = code.unwrap_or_default();
    run(e, Source::Menu(spec.menu.into()), &code)
}

fn make(e: &mut Engine) {
    let (outcome, text) = run(e, Source::Typed, MAKE);
    assert_eq!(outcome, Outcome::Ok, "{text}");
}

fn holds(e: &mut Engine, exprs: &[&str]) {
    let all = exprs.join(", ");
    let (outcome, text) = run(e, Source::Typed, &format!("stopifnot({all})"));
    if outcome != Outcome::Ok {
        let mut each = String::new();
        for x in exprs {
            let shown = x
                .strip_prefix("isTRUE(")
                .and_then(|inner| inner.strip_suffix(')'))
                .unwrap_or(x);
            let (_, got) = run(e, Source::Typed, &format!("print({shown})"));
            each.push_str(&format!("\n{x}\n  -> {}", got.trim()));
        }
        assert_eq!(
            outcome,
            Outcome::Ok,
            "{all}\n{text}\nchecks one by one:{each}"
        );
    }
}

/// A number R computes, to compare with one the dialog printed.
fn number(e: &mut Engine, expr: &str) -> f64 {
    let (outcome, text) = run(e, Source::Typed, &format!("cat(sprintf('%.12f', {expr}))"));
    assert_eq!(outcome, Outcome::Ok, "{expr}\n{text}");
    text.trim().parse().unwrap_or(f64::NAN)
}

/// The estimate printed on the metric's row (`1 rmse standard 0.98`).
fn printed(text: &str, metric: &str) -> f64 {
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .find(|w| w.get(1) == Some(&metric))
        .and_then(|w| w.last().and_then(|v| v.parse().ok()))
        .unwrap_or(f64::NAN)
}

fn close(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= 5e-6 * b.abs().max(1.0),
        "{what}: dialog printed {a}, by hand {b}"
    );
}

#[test]
fn linear_fit_split_and_rmse_match_the_formulas() {
    let Some(mut e) = engine() else { return };
    if versions(&mut e).is_none() {
        return;
    }
    make(&mut e);
    let (outcome, text) = dialog_with_outcome(&mut e, "d", "y", &["x1", "x2", "g"], &[], &[]);
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("Training rows: 90   Test rows: 30   Seed: 123"),
        "{text}"
    );
    // the split is the rows sample() draws with the same seed
    let (o, t) = run(
        &mut e,
        Source::Typed,
        "i <- local({ set.seed(123); sample(nrow(d), floor(nrow(d) * 0.75)) }); \
         lmf <- lm(y ~ x1 + x2 + g, data = d[i, ]); \
         yhat <- predict(lmf, d[-i, ])",
    );
    assert_eq!(o, Outcome::Ok, "{t}");
    holds(
        &mut e,
        &[
            "isTRUE(identical(as.integer(ml_split$in_id), as.integer(i)))",
            "isTRUE(all.equal(ml_train$y, d$y[i]))",
            "isTRUE(all.equal(ml_train$x1, d$x1[i]))",
            "isTRUE(all.equal(ml_test$y, d$y[-i]))",
            "isTRUE(all.equal(ml_test$x2, d$x2[-i]))",
            "nrow(ml_train) == 90L && nrow(ml_test) == 30L",
            // the fit is lm on the training rows
            "isTRUE(all.equal(coef(workflows::extract_fit_engine(ml_fit)), coef(lmf), tolerance = 1e-10))",
            "isTRUE(all.equal(ml_pred$.pred, unname(yhat), tolerance = 1e-10))",
            "isTRUE(all.equal(ml_pred$truth, d$y[-i]))",
            // the data are as they were
            "identical(d, d0)",
        ],
    );
    let rmse = number(&mut e, "sqrt(mean((d$y[-i] - yhat)^2))");
    let rsq = number(&mut e, "cor(d$y[-i], yhat)^2");
    close(printed(&text, "rmse"), rmse, "RMSE");
    close(printed(&text, "rsq"), rsq, "R-squared");
    // the same on the data with values missing: rows with one are left out
    // first, then the split is the sample of what is left
    let (outcome, text) = dialog_with_outcome(
        &mut e,
        "dn",
        "y",
        &["x1", "x2", "g"],
        &[("seed", "9"), ("proportion", "0.6")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (o, t) = run(
        &mut e,
        Source::Typed,
        "kk <- na.omit(dn[c('y', 'x1', 'x2', 'g')]); \
         j <- local({ set.seed(9); sample(nrow(kk), floor(nrow(kk) * 0.6)) }); \
         lm2 <- lm(y ~ x1 + x2 + g, data = kk[j, ]); y2 <- predict(lm2, kk[-j, ])",
    );
    assert_eq!(o, Outcome::Ok, "{t}");
    holds(
        &mut e,
        &[
            "nrow(kk) < nrow(dn)",
            "isTRUE(identical(as.integer(ml_split$in_id), as.integer(j)))",
            "isTRUE(all.equal(ml_train$y, kk$y[j]))",
            "isTRUE(all.equal(ml_test$y, kk$y[-j]))",
            "isTRUE(all.equal(coef(workflows::extract_fit_engine(ml_fit)), coef(lm2), tolerance = 1e-10))",
        ],
    );
    assert!(
        text.contains(&format!("Rows used: {}", number(&mut e, "nrow(kk)") as i64)),
        "{text}"
    );
    close(
        printed(&text, "rmse"),
        number(&mut e, "sqrt(mean((kk$y[-j] - y2)^2))"),
        "RMSE with rows left out",
    );
}

/// `dialog` with the outcome picker set.
fn dialog_with_outcome(
    e: &mut Engine,
    dataset: &str,
    outcome: &str,
    predictors: &[&str],
    set: &[(&str, &str)],
    on: &[&str],
) -> (Outcome, String) {
    let mut set: Vec<(&str, &str)> = set.to_vec();
    set.push(("outcome", outcome));
    dialog(e, dataset, &set, &[("predictors", predictors)], on)
}

#[test]
fn logistic_fit_accuracy_and_auc_match_the_formulas() {
    let Some(mut e) = engine() else { return };
    if versions(&mut e).is_none() {
        return;
    }
    make(&mut e);
    let (outcome, text) = dialog_with_outcome(
        &mut e,
        "d",
        "hit",
        &["x1", "x2"],
        &[("model", "logistic"), ("seed", "42"), ("strata", "g")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    assert!(
        text.contains("The event (the first level of the outcome) is FALSE"),
        "{text}"
    );
    // the split is stratified: no row twice, every stratum split near 75/25
    holds(
        &mut e,
        &[
            "anyDuplicated(ml_split$in_id) == 0L",
            "nrow(ml_train) + nrow(ml_test) == nrow(d)",
            "all(abs(as.vector(table(ml_train$g)) / as.vector(table(d$g)) - 0.75) < 0.06)",
            "all(abs(as.vector(table(ml_test$g)) / as.vector(table(d$g)) - 0.25) < 0.06)",
            "isTRUE(all.equal(ml_train$x1, d$x1[ml_split$in_id]))",
            // the outcome became a factor, FALSE first
            "identical(levels(ml_train$hit), c('FALSE', 'TRUE'))",
        ],
    );
    let (o, t) = run(
        &mut e,
        Source::Typed,
        "gf <- glm(hit ~ x1 + x2, family = binomial, data = ml_train); \
         p_true <- predict(gf, ml_test, type = 'response'); \
         cls <- factor(ifelse(p_true > 0.5, 'TRUE', 'FALSE'), levels = c('FALSE', 'TRUE')); \
         score <- 1 - p_true; ev <- ml_test$hit == 'FALSE'; n1 <- sum(ev); n0 <- sum(!ev); \
         auc <- (sum(rank(score)[ev]) - n1 * (n1 + 1) / 2) / (n1 * n0)",
    );
    assert_eq!(o, Outcome::Ok, "{t}");
    holds(
        &mut e,
        &[
            "isTRUE(all.equal(coef(workflows::extract_fit_engine(ml_fit)), coef(gf), tolerance = 1e-8))",
            "isTRUE(all.equal(ml_pred$.prob, unname(score), tolerance = 1e-8))",
            "identical(as.character(ml_pred$.pred_class), as.character(cls))",
        ],
    );
    close(
        printed(&text, "accuracy"),
        number(&mut e, "mean(cls == ml_test$hit)"),
        "accuracy",
    );
    close(printed(&text, "roc_auc"), number(&mut e, "auc"), "ROC AUC");
    assert!(text.contains("Confusion matrix"), "{text}");
}

#[test]
fn a_logistic_outcome_with_three_classes_stops_with_a_reason() {
    let Some(mut e) = engine() else { return };
    if versions(&mut e).is_none() {
        return;
    }
    make(&mut e);
    // g has three levels: the logistic model needs two
    let (outcome, text) = dialog_with_outcome(
        &mut e,
        "d",
        "g",
        &["x1", "x2"],
        &[("model", "logistic")],
        &[],
    );
    assert_eq!(outcome, Outcome::Error, "{text}");
    assert!(text.contains("exactly two classes"), "{text}");
}

#[test]
fn the_forest_and_the_tuning_name_their_package_or_run() {
    let Some(mut e) = engine() else { return };
    let Some((ranger, glmnet)) = versions(&mut e) else {
        return;
    };
    make(&mut e);
    let (outcome, text) = dialog_with_outcome(
        &mut e,
        "d",
        "y",
        &["x1", "x2", "g"],
        &[("model", "forest"), ("trees", "50")],
        &[],
    );
    if ranger {
        assert_eq!(outcome, Outcome::Ok, "{text}");
        holds(
            &mut e,
            &[
                "nrow(ml_pred) == nrow(ml_test)",
                "all(is.finite(ml_pred$.pred))",
            ],
        );
    } else {
        assert_eq!(outcome, Outcome::Error, "{text}");
        assert!(text.contains("needs the ranger package"), "{text}");
        // nothing was split: the check ran before the data were touched
        let (_, t) = run(&mut e, Source::Typed, "cat(exists('ml_split'))");
        assert_eq!(t.trim(), "FALSE");
    }
    let (outcome, text) = dialog_with_outcome(
        &mut e,
        "d",
        "y",
        &["x1", "x2"],
        &[("folds", "4"), ("levels", "6")],
        &["tune"],
    );
    if glmnet {
        assert_eq!(outcome, Outcome::Ok, "{text}");
        holds(
            &mut e,
            &[
                "nrow(tune::collect_metrics(ml_tuned)) == 12L",
                "ml_best$penalty %in% tune::collect_metrics(ml_tuned)$penalty",
                "nrow(ml_pred) == nrow(ml_test)",
            ],
        );
    } else {
        assert_eq!(outcome, Outcome::Error, "{text}");
        assert!(text.contains("needs the glmnet package"), "{text}");
    }
}

#[test]
fn random_forest_matches_ranger_by_hand() {
    let Some(mut e) = engine() else { return };
    let Some((ranger, _)) = versions(&mut e) else {
        return;
    };
    if !ranger {
        eprintln!("skipped: ranger is not installed");
        return;
    }
    make(&mut e);
    // regression: the forest ranger grows with the seed parsnip draws after
    // the split (the dialog sets the seed once, before the split)
    let (outcome, text) = dialog_with_outcome(
        &mut e,
        "d",
        "y",
        &["x1", "x2"],
        &[("model", "forest"), ("trees", "50")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (o, t) = run(
        &mut e,
        Source::Typed,
        "set.seed(123); i <- sample(nrow(d), floor(nrow(d) * 0.75)); sd <- sample.int(10^5, 1); \
         rf <- ranger::ranger(y ~ x1 + x2, data = d[i, ], num.trees = 50, num.threads = 1, seed = sd); \
         yhat <- predict(rf, d[-i, ])$predictions",
    );
    assert_eq!(o, Outcome::Ok, "{t}");
    holds(
        &mut e,
        &[
            "isTRUE(all.equal(ml_pred$.pred, yhat, tolerance = 1e-10))",
            "isTRUE(all.equal(ml_pred$truth, d$y[-i]))",
        ],
    );
    close(
        printed(&text, "rmse"),
        number(&mut e, "sqrt(mean((d$y[-i] - yhat)^2))"),
        "forest RMSE",
    );
    // classes: a probability forest, the first level (FALSE) as the event
    let (outcome, text) = dialog_with_outcome(
        &mut e,
        "d",
        "hit",
        &["x1", "x2"],
        &[("model", "forest"), ("trees", "50")],
        &[],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    let (o, t) = run(
        &mut e,
        Source::Typed,
        "set.seed(123); i <- sample(nrow(d), floor(nrow(d) * 0.75)); sd <- sample.int(10^5, 1); \
         k <- d[c('hit', 'x1', 'x2')]; k$hit <- factor(k$hit); \
         rf <- ranger::ranger(hit ~ x1 + x2, data = k[i, ], num.trees = 50, num.threads = 1, \
         seed = sd, probability = TRUE); \
         ph <- predict(rf, k[-i, ])$predictions[, 1]; \
         cls <- factor(ifelse(ph > 0.5, 'FALSE', 'TRUE'), levels = c('FALSE', 'TRUE'))",
    );
    assert_eq!(o, Outcome::Ok, "{t}");
    holds(
        &mut e,
        &["isTRUE(all.equal(ml_pred$.prob, unname(ph), tolerance = 1e-10))"],
    );
    close(
        printed(&text, "accuracy"),
        number(&mut e, "mean(cls == k$hit[-i])"),
        "forest accuracy",
    );
}

#[test]
fn tuned_penalty_matches_glmnet_and_the_folds_by_hand() {
    let Some(mut e) = engine() else { return };
    let Some((_, glmnet)) = versions(&mut e) else {
        return;
    };
    if !glmnet {
        eprintln!("skipped: glmnet is not installed");
        return;
    }
    make(&mut e);
    let (outcome, text) = dialog_with_outcome(
        &mut e,
        "d",
        "y",
        &["x1", "x2"],
        &[("folds", "4"), ("levels", "6"), ("mixture", "1")],
        &["tune"],
    );
    assert_eq!(outcome, Outcome::Ok, "{text}");
    // by hand: the same 4 folds, and glmnet (lasso) fitted on each analysis
    // set as parsnip fits it: glmnet's own path of penalties, each penalty
    // read off the path by interpolation, a penalty below the path's end
    // predicted at that end (so every penalty below it scores alike)
    let (o, t) = run(
        &mut e,
        Source::Typed,
        "pens <- 10^seq(-10, 0, length.out = 6); \
         set.seed(123); fo <- rsample::vfold_cv(ml_train, v = 4); \
         X <- function(k) as.matrix(k[c('x1', 'x2')]); \
         per_fold <- sapply(fo$splits, function(s) { \
           a <- rsample::analysis(s); b <- rsample::assessment(s); \
           f <- glmnet::glmnet(X(a), a$y, alpha = 1); \
           sapply(pens, function(pn) sqrt(mean((b$y - as.vector(predict(f, X(b), s = pn)))^2))) }); \
         cv <- rowMeans(per_fold); \
         tm <- tune::collect_metrics(ml_tuned); tm <- tm[tm$.metric == 'rmse', ]; \
         tm <- tm[order(tm$penalty), ]; bp <- pens[which.min(cv)]; \
         ff <- glmnet::glmnet(X(ml_train), ml_train$y, alpha = 1); lo <- min(ff$lambda); \
         ph <- as.vector(predict(ff, X(ml_test), s = bp))",
    );
    assert_eq!(o, Outcome::Ok, "{t}");
    holds(
        &mut e,
        &[
            "isTRUE(all.equal(tm$penalty, pens, tolerance = 1e-12))",
            "all(tm$n == 4L)",
            "isTRUE(all.equal(tm$mean, cv, tolerance = 1e-9))",
            "isTRUE(all.equal(ml_best$penalty, bp, tolerance = 1e-12))",
            "isTRUE(all.equal(as.matrix(coef(workflows::extract_fit_engine(ml_fit), s = bp)), \
             as.matrix(coef(ff, s = bp)), tolerance = 1e-9))",
            "isTRUE(all.equal(ml_pred$.pred, ph, tolerance = 1e-9))",
            // the penalties below the path's end score alike, and the dialog says so
            "bp < lo",
            "length(unique(round(tm$mean[tm$penalty < lo], 12))) == 1L",
        ],
    );
    assert!(
        text.contains("glmnet's path of penalties stops at"),
        "{text}"
    );
    close(
        printed(&text, "rmse"),
        number(&mut e, "sqrt(mean((ml_test$y - ph)^2))"),
        "tuned RMSE",
    );
}
