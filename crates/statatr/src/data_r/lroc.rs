//! `lroc [depvar] [if] [in] [fweight] [, nograph all beta(matname)]` after
//! logit or probit: the area under the ROC curve as Stata 19 prints it and the
//! r() it leaves (`plan/stata/lroc.md`). The sample is e(sample), or the rows
//! if and in choose (they replace it), or every row with all or beta(); the
//! predicted probability is a float, as `predict` makes it, and so are the
//! curve's points, whose trapezoids are summed in double: that is where
//! Stata's r(area) gets its residue (.5000000074505806 for a half). The curve
//! itself is a graph and is not drawn here. Checked against Stata 19 on
//! made-up data.

use super::model::Fit;
use super::*;

pub(super) const LROC_HELPER: &str = r##"stata_lroc <- function(y, p, keep, w = NULL, model = "Logistic", dep, show = TRUE) {
  if (is.null(y)) stop("variable ", dep, " not found (Stata: r(111))", call. = FALSE)
  if (is.character(y)) stop("lroc: the string variable ", dep, ", where Stata 19's output is not matched; not translated", call. = FALSE)
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  y <- num(y); p <- num(p)
  w <- if (is.null(w)) rep(1, length(y)) else num(w)
  if (length(w) != length(y)) stop("lroc: the weight variable was not found", call. = FALSE)
  ok <- keep %in% TRUE & !is.na(y) & !is.na(p) & !is.na(w)
  if (any(w[ok] < 0)) stop("lroc: negative frequency weights, where Stata 19's output is not matched; not translated", call. = FALSE)
  if (any(w[ok] != round(w[ok]))) stop("may not use noninteger frequency weights (Stata: r(401))", call. = FALSE)
  ok <- ok & w > 0
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  fl <- function(v) readBin(writeBin(as.numeric(v), raw(), size = 4), "double", size = 4, n = length(v))
  D <- y[ok] != 0; p <- fl(p[ok]); w <- w[ok]
  N <- sum(w); n1 <- sum(w[D]); n0 <- sum(w[!D])
  area <- 0
  if (n1 > 0 && n0 > 0) {
    # the curve's points from the highest cut down: sensitivity and 1 - specificity, each a float
    o <- order(-p); ps <- p[o]
    tp <- cumsum(w[o] * D[o]); fp <- cumsum(w[o] * !D[o])
    last <- c(ps[-1] != ps[-length(ps)], TRUE)
    se <- fl(c(0, tp[last] / n1)); sp <- fl(c(0, fp[last] / n0))
    area <- sum(diff(sp) * (se[-1] + se[-length(se)]) / 2)
  }
  if (show) cat("\n", model, " model for ", dep, "\n\nNumber of observations = ", stata_fmt(N, "%8.0f"),
                "\nArea under ROC curve   = ", stata_fmt(area, "%8.4f"), "\n", sep = "")
  invisible(list(area = area, N = N))
}
# beta(): the score from a row vector named by variables and _cons
stata_lroc_score <- function(d, b) {
  b <- as.matrix(b)
  nm <- colnames(b)
  if (nrow(b) != 1 || is.null(nm) || any(grepl("[:.#]", nm)) || !all(nm %in% c(names(d), "_cons")))
    stop("lroc, beta(): a row vector whose columns are the data's variables and _cons is translated; this one is not", call. = FALSE)
  s <- rep(0, nrow(d))
  for (j in seq_along(nm)) {
    v <- if (nm[j] == "_cons") 1 else { u <- unclass(d[[nm[j]]]); attributes(u) <- NULL; as.numeric(u) }
    s <- s + b[1, j] * v
  }
  stats::plogis(s)
}"##;

const R_LROC: &[&str] = &["area", "N"];

impl Session {
    /// `lroc [depvar] [if] [in] [fweight] [, nograph all beta(matname)]`.
    pub(super) fn lroc(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        use crate::lang::syntax::WeightKind;
        self.no_by(c, "lroc")?;
        let stop = |msg: &str, rc: u32| {
            format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{msg} (Stata: r({rc}))"))
            )
        };
        if let Some(o) = opts.other(&[("nograph", 3), ("all", 3), ("beta", 4)]) {
            return Err(no(format!(
                "lroc with the option {o} is not translated yet (its graph's options \
                 are the graph translation's)"
            )));
        }
        let beta = opts.arg("beta", 4).map(|b| b.trim().to_string());
        let all = opts.has("all", 3);
        let if_in = c.parts.if_exp.is_some() || c.parts.in_range.is_some();
        // the model: logit and probit; Stata 19 finds no estimates after the others seen
        let est = match (&self.estimates, &self.e_ret) {
            (Some(e), _) => e.clone(),
            (None, super::returned::Returned::Nothing) => {
                return Err(no(
                    "lroc needs a model fitted earlier in this do-file (logit or probit), \
                     or a known one for beta()'s title",
                ))
            }
            (None, _) => {
                return Err(self
                    .estimates_for("lroc")
                    .err()
                    .unwrap_or_else(|| no("lroc: the model is not known here")))
            }
        };
        let model = match est.fit {
            Fit::Logit => "Logistic",
            Fit::Probit => "Probit",
            Fit::Regress | Fit::Poisson | Fit::Glm | Fit::Ologit => {
                if beta.is_none() {
                    out.line(stop("last estimates not found", 301));
                    return Ok(());
                }
                // beta() does not read the model; its title is logit's
                "Logistic"
            }
            f => return Err(no(format!("lroc after {} is not translated yet", f.name()))),
        };
        // weights: lroc's own, or the fit's frequency weights
        let wvar = match &c.parts.weight {
            Some(w) => match w.kind {
                WeightKind::Frequency => Some(w.exp.trim().to_string()),
                WeightKind::Analytic => {
                    out.line(stop("aweights not allowed", 101));
                    return Ok(());
                }
                WeightKind::Sampling => {
                    out.line(stop("pweights not allowed", 101));
                    return Ok(());
                }
                WeightKind::Importance => {
                    out.line(stop("iweights not allowed", 101));
                    return Ok(());
                }
                WeightKind::Unspecified => {
                    return Err(no(
                        "lroc [weight = ...] is not translated; name it as fweight",
                    ))
                }
            },
            None if beta.is_some() => {
                if self.fit_weight.is_some() {
                    return Err(no(
                        "lroc, beta() after a weighted fit is not translated yet",
                    ));
                }
                None
            }
            None => match &self.fit_weight {
                None => None,
                Some(w) => match w.kind {
                    WeightKind::Frequency => Some(w.exp.trim().to_string()),
                    WeightKind::Sampling => {
                        out.line(stop("pweights not allowed", 101));
                        return Ok(());
                    }
                    _ => {
                        return Err(no(
                            "lroc after a fit with these weights is not translated yet",
                        ))
                    }
                },
            },
        };
        if let Some(w) = &wvar {
            if !is_name(w) {
                return Err(no(
                    "lroc with a frequency weight other than a variable is not translated",
                ));
            }
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "lroc {v}: the variable written out is translated"
            )));
        }
        let d = self.data()?;
        let dep = match (&beta, vars.as_slice()) {
            (None, []) => est.dep.clone(),
            (None, [v]) if *v == est.dep => est.dep.clone(),
            (None, [v]) if est.dep.starts_with(v) => {
                return Err(no(format!(
                    "lroc {v}: an abbreviated variable name is not translated here"
                )))
            }
            (None, [_]) => {
                out.line(stop(
                    "varlist not allowed (except with beta() option)\ninvalid syntax",
                    198,
                ));
                return Ok(());
            }
            (None, _) => {
                out.line(stop("too many variables specified", 103));
                return Ok(());
            }
            (Some(_), []) => {
                out.line(stop(
                    "dependent variable required with beta() option\ninvalid syntax",
                    198,
                ));
                return Ok(());
            }
            (Some(_), [v]) => v.to_string(),
            (Some(_), _) => {
                return Err(no(
                    "lroc with beta() and more than one variable is not translated",
                ))
            }
        };
        if all && if_in && beta.is_none() {
            out.line(stop(
                "all option not allowed with if or in\ninvalid syntax",
                198,
            ));
            return Ok(());
        }
        // the base data, with the columns a model's L.x and the like added to it
        let base = match (&self.fit_ts, &beta) {
            (Some(cols), None) => {
                out.package("dplyr");
                format!("dplyr::mutate({d}, {cols})")
            }
            _ => d.clone(),
        };
        let score = match &beta {
            Some(b) => {
                if !is_name(b) {
                    return Err(no(format!("lroc, beta({b}) needs a matrix's name")));
                }
                let m = self.user_matrix(b)?;
                format!("stata_lroc_score({base}, {m})")
            }
            None => format!(
                "unname(stats::predict(stata_estimates, newdata = {base}, type = \"response\"))"
            ),
        };
        // the rows: e(sample), unless if or in, all or beta() say otherwise
        let keep = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("dplyr::transmute({base}, stata_v = {cond})$stata_v")
            }
            None if all || beta.is_some() => format!("rep(TRUE, nrow({d}))"),
            None => {
                if !self.esample {
                    return Err(no(
                        "lroc: the model's estimation sample is not known here (a line since \
                         the fit may have changed the rows)",
                    ));
                }
                "stata_esample".to_string()
            }
        };
        let mut args = format!("{d}[[{}]], {score}, {keep}", r_text(&dep));
        if let Some(w) = &wvar {
            args.push_str(&format!(", w = {d}[[{}]]", r_text(w)));
        }
        args.push_str(&format!(
            ", model = {}, dep = {}",
            r_text(model),
            r_text(&dep)
        ));
        if self.quiet(c) {
            args.push_str(", show = FALSE");
        }
        if !opts.has("nograph", 3) {
            // the ROC curve: a graph, left to the graph translation; what is drawn
            // last is no longer the graph a graph export would write
            self.graph_drawn = false;
            out.caveat(
                "lroc draws the ROC curve in Stata; RCraft prints the area and does not draw \
                 the curve (graph export after it is not translated)",
            );
        }
        for h in ["stata_lroc", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- stata_lroc({args})"));
        self.r_new = Some((R_LROC, "lroc".into()));
        Ok(())
    }
}
