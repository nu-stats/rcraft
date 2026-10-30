//! P2B.4: Stata's weights, `[fweight = w]`, `[aweight = w]`, `[pweight = w]`
//! and `[iweight = w]`, for the statistics commands and the models that take
//! them, each rule decoded in Stata 19 (see the P2B.4 notes):
//!
//! - a row whose weight is missing or 0 is left out, for every kind; a
//!   negative weight stops (`negative weights encountered`) except an
//!   iweight, and a frequency weight must be a whole number
//!   (`may not use noninteger frequency weights`);
//! - frequency weights are copies of the row: the number of observations is
//!   their sum, and so are the degrees of freedom;
//! - analytic weights are scaled to sum to the number of rows, so they change
//!   the estimates but not the count;
//! - sampling weights give the same estimates as analytic ones with robust
//!   errors, which they imply;
//! - importance weights are taken as they are, with their sum in the
//!   variance's divisor.
//!
//! `[weight = w]` means a different kind for each command, and a weight that
//! is an expression would have to be worked out first; both are refused.

use super::*;
use crate::lang::syntax::WeightKind;

/// A kind of weight, as the R helpers name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum W {
    F,
    A,
    P,
    I,
}

impl W {
    pub(super) fn code(self) -> &'static str {
        match self {
            W::F => "f",
            W::A => "a",
            W::P => "p",
            W::I => "i",
        }
    }
    fn word(self) -> &'static str {
        match self {
            W::F => "fweights",
            W::A => "aweights",
            W::P => "pweights",
            W::I => "iweights",
        }
    }
}

/// A command's weight: its kind and its variable.
pub(super) type Weighting = Option<(W, String)>;

/// Leaves out the rows Stata leaves out for a weight (missing or 0), after
/// Stata's checks of the values.
pub(super) const WROWS_HELPER: &str = r#"stata_wrows <- function(d, w, kind) {
  v <- unclass(d[[w]])
  attributes(v) <- NULL
  ok <- !is.na(v)
  if (kind == "f" && any(v[ok] != floor(v[ok]))) stop("may not use noninteger frequency weights")
  if (kind != "i" && any(v[ok] < 0)) stop("negative weights encountered")
  d[ok & v != 0, , drop = FALSE]
}"#;

/// The variance of a weighted least-squares fit as Stata's `regress` gives
/// it, with its residual degrees of freedom as `attr(, "df")`. Frequency
/// weights count as copies of the row (N is their sum); analytic and
/// sampling weights keep N as the rows; importance weights divide by their
/// sum. Robust and clustered sandwiches are scaled by `N / (N - k)` and
/// `G / (G - 1) (N - 1) / (N - k)`, with the scores `w e x`.
pub(super) const LM_WVCOV_HELPER: &str = r#"stata_lm_wvcov <- function(fit, kind, robust = FALSE, cluster = NULL) {
  X <- stats::model.matrix(fit)
  w <- stats::weights(fit)
  e <- stats::residuals(fit)
  k <- ncol(X)
  n <- nrow(X)
  N <- if (kind == "f") sum(w) else n
  B <- solve(crossprod(X * w, X))
  if (!robust && is.null(cluster)) {
    d <- switch(kind, f = N - k, i = sum(w) - k, n - k)
    V <- B * sum(w * e^2) / d
    df <- switch(kind, f = N - k, i = floor(sum(w)) - k, n - k)
  } else if (is.null(cluster)) {
    meat <- if (kind == "f") crossprod(X * (sqrt(w) * e)) else crossprod(X * (w * e))
    V <- B %*% meat %*% B * N / (N - k)
    df <- N - k
  } else {
    sg <- rowsum(X * (w * e), cluster)
    G <- nrow(sg)
    V <- B %*% crossprod(sg) %*% B * G / (G - 1) * (N - 1) / (N - k)
    df <- G - 1
  }
  dimnames(V) <- list(colnames(X), colnames(X))
  attr(V, "df") <- df
  V
}"#;

impl Session {
    /// The weight a command was given, if any: only the kinds `allowed`, and
    /// in Stata's words for the others.
    pub(super) fn weight_of(&self, c: &Command, allowed: &[W]) -> Result<Weighting, Untranslated> {
        let Some(w) = &c.parts.weight else {
            return Ok(None);
        };
        let kind =
            match w.kind {
                WeightKind::Frequency => W::F,
                WeightKind::Analytic => W::A,
                WeightKind::Sampling => W::P,
                WeightKind::Importance => W::I,
                WeightKind::Unspecified => return Err(no(
                    "[weight = ...] means a different kind for each command; name it as fweight, \
                     aweight, pweight or iweight",
                )),
            };
        if !allowed.contains(&kind) {
            return Err(no(format!("{} not allowed", kind.word())));
        }
        let v = w.exp.trim();
        if !is_name(v) {
            return Err(no(format!(
                "a weight that is an expression ({v}) is not translated yet; generate it as a \
                 variable first"
            )));
        }
        Ok(Some((kind, v.to_string())))
    }

    /// `src` without the rows the weight leaves out.
    pub(super) fn weighted_rows(&mut self, src: String, w: &Weighting) -> String {
        match w {
            None => src,
            Some((k, v)) => {
                self.helpers.insert("stata_wrows");
                format!("stata_wrows({src}, {}, {})", r_text(v), r_text(k.code()))
            }
        }
    }
}

/// `, w = rows$w, wtype = "f"` for a helper, or nothing.
pub(super) fn w_args(rows: &str, w: &Weighting) -> String {
    match w {
        None => String::new(),
        Some((k, v)) => format!(", w = {rows}${}, wtype = {}", r_name(v), r_text(k.code())),
    }
}

/// A weighted covariance matrix as Stata's `correlate` gives it: frequency
/// weights over their sum less one, analytic ones scaled to the rows.
pub(super) const WCOV_HELPER: &str = r#"stata_wcov <- function(m, w, kind) {
  W <- sum(w)
  n <- nrow(m)
  dev <- sweep(m, 2, colSums(m * w) / W)
  S <- crossprod(dev * sqrt(w))
  if (kind == "f") S / (W - 1) else S / W * n / (n - 1)
}"#;

impl Session {
    /// A statistics command's rows (`stata_rows`, kept in R when there is a
    /// weight) and its varlist's columns, with the weight arguments for its
    /// helper.
    pub(super) fn weighted_frame(
        &mut self,
        c: &Command,
        out: &mut Out,
        w: &Weighting,
    ) -> Result<(String, String, String), Untranslated> {
        let rows = self.rows_for(c, out)?;
        if w.is_none() {
            let frame = self.frame_for(c, out)?;
            return Ok((rows, frame, String::new()));
        }
        let rows = self.weighted_rows(rows, w);
        out.line(format!("stata_rows <- {rows}"));
        let frame = if c.parts.main.trim().is_empty() {
            "stata_rows".to_string()
        } else {
            out.package("dplyr");
            format!("dplyr::select(stata_rows, {})", select(&c.parts.main)?)
        };
        out.caveat(weight_caveat(w));
        Ok(("stata_rows".into(), frame, w_args("stata_rows", w)))
    }
}

/// What a weight does, in a line.
pub(super) fn weight_caveat(w: &Weighting) -> &'static str {
    match w {
        Some((W::F, _)) => "frequency weights count each row that many times, as Stata's do",
        Some((W::A, _)) => {
            "analytic weights are scaled to sum to the number of rows, as Stata's are"
        }
        Some((W::P, _)) => "sampling weights, with robust standard errors, as Stata's",
        Some((W::I, _)) => "importance weights are taken as they are, as Stata takes them",
        None => "",
    }
}
