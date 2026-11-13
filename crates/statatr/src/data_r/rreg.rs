//! `rreg depvar indepvars [if] [in]`: robust regression as Stata 19 fits it, worked out
//! from its iteration logs, e() and genwt() weights on made-up data, not from its code
//! (`plan/stata/rreg.md`).
//!
//! - Least squares first; rows with Cook's distance above 1 are left out, silently.
//! - Huber iterations, each weight min(1, 2M/|r|) with M the residuals' median absolute
//!   deviation about their median, until the largest change in a weight is below 0.05;
//!   then biweight iterations, (1 - (u/4.685)^2)^2 with u = r/s, s = M/0.6745, until it
//!   is below 0.01 (`tolerance()`'s default). Each is a weighted least-squares fit.
//! - The variance is the pseudovalues' one: rmse^2 (X'X)^-1, rmse^2 the sum of (w r)^2
//!   over (n - p), over m^2, times (1 + (p/n') (1 - m)/m)^2, with m the mean of the
//!   biweight's psi' at the last residuals and scale, n' the rows of nonzero weight.
//! - e(F) is the Wald test of the slopes; e(mss) = F df_m rmse^2, e(rss) = rmse^2 df_r,
//!   e(r2) and e(r2_a) from them; e(beta) each slope times sd(x) over
//!   sqrt((mss + rss)/(N - 1)).

use super::model::Spec;
use super::*;

/// The fit, its e() and Stata's iteration log and header.
pub(super) const RREG_HELPER: &str = r#"stata_rreg <- function(d, dep, xs, cmdline, show = TRUE) {
  vars <- c(dep, xs)
  rows <- which(stats::complete.cases(d[vars]))
  y <- as.numeric(d[[dep]][rows])
  X <- cbind(as.matrix(d[rows, xs, drop = FALSE]), `_cons` = 1)
  storage.mode(X) <- "double"
  p <- ncol(X)
  if (length(y) <= p) stop("insufficient observations (Stata: r(2001))", call. = FALSE)
  q <- qr(X)
  if (q$rank < p) stop("RCraft: rreg with collinear variables is not translated yet", call. = FALSE)
  # Cook's distance of least squares: rows above 1 are left out
  e0 <- as.numeric(qr.resid(q, y))
  h <- rowSums(qr.Q(q)^2)
  if (any(h > 1 - 1e-12)) stop("RCraft: rreg with a row of leverage 1 is not translated yet", call. = FALSE)
  cook <- e0^2 / (p * sum(e0^2) / (length(y) - p)) * h / (1 - h)^2
  keep <- !(cook > 1)
  rows <- rows[keep]
  y <- y[keep]
  X <- X[keep, , drop = FALSE]
  n <- length(y)
  if (n <= p || qr(X)$rank < p) stop("RCraft: rreg with too few rows or collinear variables after its Cook's distance screen is not translated yet", call. = FALSE)
  wfit <- function(w) {
    b <- solve(crossprod(X * w, X), crossprod(X * w, y))
    list(b = as.numeric(b), r = as.numeric(y - X %*% b))
  }
  mad <- function(r) stats::median(abs(r - stats::median(r)))
  line <- function(kind, it, dl) if (show) cat(sprintf("%8s iteration %d:  Maximum difference in weights = %s\n", kind, it, trimws(stata_fmt(dl, "%10.0g"))))
  if (show) cat("\n")
  w <- rep(1, n)
  f <- wfit(w)
  it <- 0
  repeat {
    it <- it + 1
    M <- mad(f$r)
    nw <- pmin(1, 2 * M / abs(f$r))
    nw[!is.finite(nw)] <- 1
    dl <- max(abs(nw - w))
    w <- nw
    f <- wfit(w)
    line("Huber", it, dl)
    if (dl < 0.05) break
    if (it >= 1000) stop("RCraft: rreg's Huber iterations did not settle in 1000 steps", call. = FALSE)
  }
  repeat {
    it <- it + 1
    M <- mad(f$r)
    s <- M / 0.6745
    u <- f$r / (4.685 * s)
    nw <- ifelse(abs(u) < 1, (1 - u^2)^2, 0)
    dl <- max(abs(nw - w))
    w <- nw
    f <- wfit(w)
    line("Biweight", it, dl)
    if (dl < 0.01) break
    if (it >= 1000) stop("RCraft: rreg's biweight iterations did not settle in 1000 steps", call. = FALSE)
  }
  if (!is.finite(s) || s <= 0) stop("RCraft: rreg with a median absolute deviation of 0 is not translated yet", call. = FALSE)
  # the pseudovalues' variance
  u <- f$r / s
  dpsi <- ifelse(abs(u) < 4.685, (1 - (u / 4.685)^2) * (1 - 5 * (u / 4.685)^2), 0)
  m <- mean(dpsi)
  ne <- sum(w > 0)
  rm2 <- sum((w * f$r)^2) / (n - p) / m^2 * (1 + p / ne * (1 - m) / m)^2
  nm <- colnames(X)
  V <- rm2 * solve(crossprod(X))
  b <- matrix(f$b, 1, dimnames = list("y1", nm))
  dimnames(V) <- list(nm, nm)
  sl <- seq_len(p - 1)
  Fv <- drop(t(f$b[sl]) %*% solve(V[sl, sl, drop = FALSE], f$b[sl])) / (p - 1)
  df_r <- n - p
  mss <- Fv * (p - 1) * rm2
  rss <- rm2 * df_r
  r2 <- mss / (mss + rss)
  beta <- matrix(f$b[sl] * apply(X[, sl, drop = FALSE], 2, stats::sd) / sqrt((mss + rss) / (n - 1)), 1,
                 dimnames = list("r1", nm[sl]))
  e <- list(N = n, sum_w = n, df_m = p - 1, df_r = df_r, F = Fv, r2 = r2, rmse = sqrt(rm2), mss = mss,
            rss = rss, r2_a = 1 - (1 - r2) * (n - 1) / df_r, rank = p, cmd = "rreg", cmdline = cmdline,
            title = "Robust regression", marginsok = "XB default", predict = "rreg_p", depvar = dep,
            properties = "b V", model = "ols", b = b, V = V, beta = beta)
  attr(e, "stata_rows") <- rows
  if (show) {
    row <- function(left, right) cat(formatC(left, width = -48, flag = "-"), right, "\n", sep = "")
    cat("\n")
    row("Robust regression", sprintf("Number of obs     = %10s", format(n, big.mark = ",")))
    row("", sprintf("F(%3d,%10d) = %10.2f", as.integer(p - 1), as.integer(df_r), Fv))
    row("", sprintf("Prob > F          = %10.4f", stats::pf(Fv, p - 1, df_r, lower.tail = FALSE)))
    cat("\n")
  }
  e
}"#;

impl Session {
    /// `rreg depvar indepvars [if] [in]`.
    pub(super) fn rreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "rreg")?;
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "rreg with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("rreg with weights is not translated yet"));
        }
        let spec = Spec::read(c, "rreg")?;
        if !spec.factors.is_empty() || spec.terms.iter().any(|t| !is_name(t)) {
            return Err(no(
                "rreg with factor variables, interactions or operators is not translated yet",
            ));
        }
        if spec.terms.is_empty() {
            return Err(no(
                "rreg without an independent variable is not translated yet",
            ));
        }
        let src = self.rows_for(c, out)?;
        for h in [
            "stata_rreg",
            "stata_fmt",
            "stata_eclass",
            "stata_returned",
            "stata_tables",
        ] {
            self.helpers.insert(h);
        }
        let show = !self.quiet(c);
        out.line(format!(
            "stata_e <- stata_rreg({src}, {}, c({}), {}, show = {})",
            r_text(&spec.dep),
            spec.terms
                .iter()
                .map(|t| r_text(t))
                .collect::<Vec<_>>()
                .join(", "),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(show)
        ));
        if show {
            out.line("stata_epost_display()");
        }
        self.estimates = None;
        self.estimates_lost =
            Some("predict, test and margins are not translated after rreg yet".into());
        self.active_store = None;
        self.e_text.clear();
        self.esample = false;
        self.e_ret = super::returned::Returned::Program {
            cmd: "rreg".into(),
            texts: [
                "cmd",
                "cmdline",
                "title",
                "marginsok",
                "predict",
                "depvar",
                "properties",
                "model",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        };
        out.caveat(
            "rreg as Stata 19 fits it (Cook's distance screen, Huber then biweight iterations, the \
             pseudovalues' variance), matched to its results, not its code",
        );
        Ok(())
    }
}
