//! The R and package versions a check ran with (AGENTS.md section 5): a
//! menu entry moves to Verified only on a check that records them, run in the
//! R that `ENVIRONMENT` pins. Shared by the test files that verify entries
//! (`mod pinned;`).

use std::time::{Duration, Instant};

use rcraft_engine::Engine;

/// Prints the R version and those of `packages`, and says when this R is not
/// the pinned one (the run is then not a Verified one).
pub fn record(e: &mut Engine, packages: &[&str]) {
    let names = packages
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let code = format!(
        "cat(paste(R.version$major, R.version$minor, sep = '.'), \
         vapply(c({names}), function(p) paste(p, tryCatch(as.character(utils::packageVersion(p)), \
         error = function(e) 'not installed')), ''), sep = '\\n')"
    );
    let id = e.query(&code);
    let limit = Instant::now() + Duration::from_secs(60);
    while e.is_working() || e.query_result(id).is_none() {
        e.poll(Duration::from_millis(20));
        if Instant::now() > limit {
            eprintln!("note: R did not say its version");
            return;
        }
    }
    let text = e
        .query_result(id)
        .map(|r| r.text.clone())
        .unwrap_or_default();
    let mut lines = text.lines();
    let ran = lines.next().unwrap_or("").trim().to_string();
    let pinned = include_str!("../../../../ENVIRONMENT")
        .lines()
        .find_map(|l| l.strip_prefix("R_VERSION="))
        .unwrap_or("")
        .trim();
    let with: Vec<&str> = lines.map(str::trim).filter(|l| !l.is_empty()).collect();
    eprintln!(
        "ran with R {ran}{}{}; ENVIRONMENT pins R {pinned}",
        if with.is_empty() { "" } else { " and " },
        with.join(", ")
    );
    if ran != pinned {
        eprintln!("note: this R is not the pinned one, so the run is not a Verified one");
    }
}

/// R that runs a dialog's code as it is and keeps the value of its last
/// expression as `name`, so a check can read the numbers it gave.
#[allow(dead_code)]
pub fn keep_value(code: &str, name: &str) -> String {
    format!("{name} <- eval(parse(text = r\"---({code})---\"), envir = globalenv())")
}

/// R that is TRUE when every named check holds, and prints the ones that do
/// not.
#[allow(dead_code)]
pub fn all_of(setup: &str, checks: &[(&str, &str)]) -> String {
    let list = checks
        .iter()
        .map(|(name, check)| format!("{name} = isTRUE({check})"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("local({{ {setup}; ok <- c({list}); if (!all(ok)) print(ok); all(ok) }})")
}

/// Shapiro and Wilk's W and its p-value written out in plain R from
/// Royston's algorithm (Applied Statistics AS R94, 1995): the coefficients
/// from normal scores with his polynomial corrections, and his normal
/// approximations of log(1 - W) for n of 4 to 11 and of 12 or more.
#[allow(dead_code)]
pub const SHAPIRO_BY_HAND: &str = "sw <- function(x) { x <- sort(x); n <- length(x); \
     m <- qnorm((seq_len(n) - 0.375) / (n + 0.25)); mm <- sum(m^2); u <- 1 / sqrt(n); \
     cc <- m / sqrt(mm); \
     an <- cc[n] + 0.221157 * u - 0.147981 * u^2 - 2.071190 * u^3 + 4.434685 * u^4 - 2.706056 * u^5; \
     if (n > 5) { \
       an1 <- cc[n - 1] + 0.042981 * u - 0.293762 * u^2 - 1.752461 * u^3 + 5.682633 * u^4 - 3.582633 * u^5; \
       phi <- (mm - 2 * m[n]^2 - 2 * m[n - 1]^2) / (1 - 2 * an^2 - 2 * an1^2); \
       a <- m / sqrt(phi); a[c(n, n - 1)] <- c(an, an1); a[c(1, 2)] <- -c(an, an1) \
     } else { \
       phi <- (mm - 2 * m[n]^2) / (1 - 2 * an^2); a <- m / sqrt(phi); a[n] <- an; a[1] <- -an }; \
     w <- sum(a * x)^2 / sum((x - mean(x))^2); ln <- log(n); \
     if (n >= 12) { mu <- -1.5861 - 0.31082 * ln - 0.083751 * ln^2 + 0.0038915 * ln^3; \
       s <- exp(-0.4803 - 0.082676 * ln + 0.0030302 * ln^2); z <- (log(1 - w) - mu) / s \
     } else { g <- -2.273 + 0.459 * n; \
       mu <- 0.5440 - 0.39978 * n + 0.025054 * n^2 - 0.0006714 * n^3; \
       s <- exp(1.3822 - 0.77857 * n + 0.062767 * n^2 - 0.0020322 * n^3); \
       z <- (-log(g - log(1 - w)) - mu) / s }; \
     c(n = n, W = w, p.value = pnorm(z, lower.tail = FALSE)) }";

/// R that draws a dialog's code on a recording device and reads back what it
/// drew (R's display list: each rectangle, polygon, set of points, line,
/// segment, image and text, with its coordinates), so a check can set the
/// shapes against values worked out by hand. `shapes(code)` gives the list,
/// `pick(s, op)` the arguments of each drawing of one kind (`C_rect`,
/// `C_polygon`, `C_plotXY`, `C_segments`, `C_abline`, `C_image`, `C_text`),
/// `has_xy(s, x, y)` whether a set of points or a line went through exactly
/// these coordinates; `q7` is the type 7 quantile and `kde` a Gaussian kernel
/// density with Silverman's bandwidth, both written out.
#[allow(dead_code)]
pub const SHAPES: &str = "shapes <- function(code) { grDevices::pdf(NULL); \
       grDevices::dev.control('enable'); on.exit(grDevices::dev.off()); \
       eval(parse(text = code), envir = globalenv()); p <- grDevices::recordPlot(); \
       lapply(p[[1]], function(o) { f <- o[[2]][[1]]; \
         list(op = if (is.list(f) && !is.null(f$name)) f$name else '?', args = as.list(o[[2]])[-1]) }) }; \
     pick <- function(s, name) lapply(Filter(function(o) o$op == name, s), `[[`, 'args'); \
     same <- function(a, b, tol = 1e-10) length(a) == length(b) && \
       isTRUE(all.equal(as.numeric(a), as.numeric(b), tolerance = tol)); \
     has_xy <- function(s, x, y, tol = 1e-10) any(vapply(pick(s, 'C_plotXY'), \
       function(a) same(a[[1]]$x, x, tol) && same(a[[1]]$y, y, tol), TRUE)); \
     q7 <- function(v, p) { v <- sort(v); h <- (length(v) - 1) * p + 1; lo <- floor(h); \
       v[lo] + (h - lo) * (v[min(lo + 1, length(v))] - v[lo]) }; \
     kde <- function(v, at) { v <- v[!is.na(v)]; n <- length(v); \
       bw <- 0.9 * min(sd(v), (q7(v, 0.75) - q7(v, 0.25)) / 1.34) * n^-0.2; \
       vapply(at, function(t) mean(dnorm((t - v) / bw)) / bw, 0) }";

/// R that draws `code` (a dialog's) and keeps what it drew as `s`.
#[allow(dead_code)]
pub fn drawn(code: &str) -> String {
    format!("{SHAPES}; s <- shapes(r\"---({code})---\")")
}
