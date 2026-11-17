//! `lroc [depvar] [if] [in] [weight] [, nograph all beta(matname)
//! twoway_options]` after logit or probit: the area under the ROC curve as
//! Stata 19 prints it and the r() it leaves (`plan/stata/lroc.md`). The sample
//! is e(sample), or the rows if and in choose (they replace it), or every row
//! with all or beta(); the predicted probability is a float, as `predict`
//! makes it, and so are the curve's points, whose trapezoids are summed in
//! double: that is where Stata's r(area) gets its residue (.5000000074505806
//! for a half). The curve itself is a graph and is not drawn here. Checked
//! against Stata 19 on made-up data.

use super::model::Fit;
use super::*;

pub(super) const LROC_HELPER: &str = r##"stata_lroc <- function(y, p, keep, w = NULL, iw = FALSE, model = "Logistic", dep, note = FALSE,
                       show = TRUE) {
  if (is.null(y)) stop("variable ", dep, " not found (Stata: r(111))", call. = FALSE)
  if (note && show) cat("(frequency weights assumed)\n")
  # a string outcome: no row has one Stata can use
  if (is.character(y)) stop("no observations (Stata: r(2000))", call. = FALSE)
  num <- function(v) { u <- unclass(v); attributes(u) <- NULL; as.numeric(u) }
  y <- num(y); p <- num(p)
  w <- if (is.null(w)) rep(1, length(y)) else num(w)
  if (length(w) != length(y)) stop("lroc: the weight variable was not found", call. = FALSE)
  ok <- keep %in% TRUE & !is.na(y) & !is.na(p) & !is.na(w)
  if (iw) {
    # the fit's iweights weigh the curve; N counts the rows
    if (any(w[ok] < 0)) stop("lroc: negative iweights, where Stata 19's output is not matched; not translated", call. = FALSE)
  } else {
    if (any(w[ok] != round(w[ok]))) stop("may not use noninteger frequency weights (Stata: r(401))", call. = FALSE)
    if (any(w[ok] < 0)) stop("negative weights encountered (Stata: r(402))", call. = FALSE)
  }
  ok <- ok & w > 0
  if (!any(ok)) stop("no observations (Stata: r(2000))", call. = FALSE)
  fl <- function(v) readBin(writeBin(as.numeric(v), raw(), size = 4), "double", size = 4, n = length(v))
  D <- y[ok] != 0; p <- fl(p[ok]); w <- w[ok]
  N <- if (iw) length(w) else sum(w)
  n1 <- sum(w[D]); n0 <- sum(w[!D])
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
# beta(): the score of a row vector, its first equation's columns: variables, factor levels
# (k.x, kb.x; o.x and ko.x add nothing) and _cons; a missing coefficient leaves every score missing
stata_lroc_score <- function(d, b) {
  b <- as.matrix(b)
  if (nrow(b) != 1) stop("conformability error (Stata: r(503))", call. = FALSE)
  nm <- colnames(b)
  if (is.null(nm)) nm <- paste0("c", seq_len(ncol(b)))
  eq <- ifelse(grepl(":", nm, fixed = TRUE), sub(":.*$", "", nm), "")
  nm <- sub("^[^:]*:", "", nm)
  use <- eq == eq[1]
  refuse <- function(n) stop("lroc, beta(): the column ", n, ", where Stata 19's output is not matched; not translated", call. = FALSE)
  col <- function(v) {
    if (!v %in% names(d)) stop("variable ", v, " not found\nvariable ", v, " not found (Stata: r(111))", call. = FALSE)
    u <- unclass(d[[v]]); attributes(u) <- NULL
    if (!is.numeric(u)) refuse(v)
    as.numeric(u)
  }
  s <- rep(0, nrow(d))
  for (j in which(use)) {
    n <- nm[j]
    if (n == "_cons") { s <- s + b[1, j]; next }
    f <- regmatches(n, regexec("^([0-9]*)(b|bn|o|bo|ob)?\\.([A-Za-z_][A-Za-z0-9_]*)$", n))[[1]]
    if (length(f)) {
      if (grepl("o", f[3], fixed = TRUE)) next
      if (f[2] == "") refuse(n)
      s <- s + b[1, j] * (col(f[4]) == as.numeric(f[2]))
    } else if (grepl("^[A-Za-z_][A-Za-z0-9_]*$", n)) {
      s <- s + b[1, j] * col(n)
    } else refuse(n)
  }
  stats::plogis(s)
}"##;

const R_LROC: &[&str] = &["area", "N"];

/// The twoway options that only change how the curve looks: taken (and the
/// curve still not drawn). Full names only, and `ti`, as seen in Stata 19.
const LOOK: &[(&str, usize)] = &[
    ("title", 2),
    ("subtitle", 8),
    ("note", 4),
    ("caption", 7),
    ("xtitle", 6),
    ("ytitle", 6),
    ("xlabel", 6),
    ("ylabel", 6),
    ("xscale", 6),
    ("yscale", 6),
    ("legend", 6),
    ("scheme", 6),
    ("nodraw", 6),
    ("aspectratio", 11),
    ("xsize", 5),
    ("ysize", 5),
    ("graphregion", 11),
    ("plotregion", 10),
];

impl Session {
    /// `lroc [depvar] [if] [in] [weight] [, nograph all beta(matname)]`.
    pub(super) fn lroc(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        use crate::lang::syntax::WeightKind;
        let stop = |msg: &str, rc: u32| {
            format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{msg} (Stata: r({rc}))"))
            )
        };
        // by: Stata's own error once the rows are in order, `not sorted` before
        // (bysort would sort the data first)
        if let Some((varlist, sort)) = c.prefixes.iter().find_map(|p| match p {
            Prefix::By { varlist, sort, .. } => Some((varlist.clone(), *sort)),
            _ => None,
        }) {
            let vars: Vec<&str> = varlist.split_whitespace().collect();
            if sort || vars.is_empty() || vars.iter().any(|v| !is_name(v)) {
                return Err(no(
                    "bysort or by (...) with lroc is not translated (Stata sorts, then stops)",
                ));
            }
            let d = self.data()?;
            self.helpers.insert("stata_by_groups");
            out.line(format!(
                "invisible(stata_by_groups({d}, c({})))",
                vars.iter()
                    .map(|v| r_text(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            out.line(stop("lroc may not be combined with by", 190));
            return Ok(());
        }
        let known = [("nograph", 3), ("all", 3), ("beta", 4)];
        let graph = !opts.has("nograph", 3);
        // with nograph, the first other option is Stata's error; without it the curve's look
        let other = |o: &&crate::lang::syntax::Opt| !known.iter().any(|(f, m)| o.is(f, *m));
        if let (false, Some(o)) = (graph, c.options.iter().find(other)) {
            let paren = if o.arg.is_some() { "()" } else { "" };
            out.line(stop(&format!("option {}{paren} not allowed", o.name), 198));
            return Ok(());
        }
        if let Some(o) = c
            .options
            .iter()
            .filter(other)
            .find(|o| !LOOK.iter().any(|(f, m)| o.is(f, *m)))
        {
            return Err(no(format!(
                "lroc with the option {} is not translated yet (its graph's options are the \
                 graph translation's)",
                o.name
            )));
        }
        let beta = opts.arg("beta", 4).map(|b| b.trim().to_string());
        let all = opts.has("all", 3);
        let if_in = c.parts.if_exp.is_some() || c.parts.in_range.is_some();
        // the model: logit and probit; after every other model seen Stata 19 finds no estimates
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
            _ => {
                if beta.is_none() {
                    out.line(stop("last estimates not found", 301));
                    return Ok(());
                }
                // beta() does not read the model; its title is logit's
                "Logistic"
            }
        };
        // weights: lroc's own, or (without beta(), which reads no model) the fit's
        let mut note = false;
        let mut iw = false;
        let wvar = match &c.parts.weight {
            Some(w) => match w.kind {
                WeightKind::Frequency => Some(w.exp.trim().to_string()),
                WeightKind::Unspecified => {
                    note = true;
                    Some(w.exp.trim().to_string())
                }
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
            },
            None if beta.is_some() => None,
            None => match &self.fit_weight {
                None => None,
                Some(w) => match w.kind {
                    WeightKind::Frequency => Some(w.exp.trim().to_string()),
                    WeightKind::Importance => {
                        iw = true;
                        Some(w.exp.trim().to_string())
                    }
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
                    "lroc with a weight other than a variable is not translated",
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
            // (the do-file reader writes an abbreviation out in full when it knows the data)
            (None, [v]) if est.dep.starts_with(v) => {
                return Err(no(format!(
                    "lroc {v}: an abbreviated variable name is not translated here (the data's \
                     names are not known)"
                )))
            }
            (None, [_]) => {
                out.line(stop(
                    "varlist not allowed (except with beta() option)\ninvalid syntax",
                    198,
                ));
                return Ok(());
            }
            (_, [_, _, ..]) => {
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
        if iw {
            args.push_str(", iw = TRUE");
        }
        args.push_str(&format!(
            ", model = {}, dep = {}",
            r_text(model),
            r_text(&dep)
        ));
        if note {
            args.push_str(", note = TRUE");
        }
        if self.quiet(c) {
            args.push_str(", show = FALSE");
        }
        if graph {
            // the ROC curve: a graph, left to the graph translation; what is drawn
            // last is no longer the graph a graph export would write
            self.graph_drawn = false;
            out.caveat(
                "lroc draws the ROC curve in Stata; RCraft prints the area and does not draw \
                 the curve, so its look options change nothing (graph export after it is not \
                 translated)",
            );
        }
        for h in ["stata_lroc", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
        out.line(format!("stata_r <- stata_lroc({args})"));
        if graph {
            // Stata's curve replaces the unnamed graph: the one kept before is not shown again
            out.line(
                "if (exists(\"stata_graphs\")) stata_graphs[[\"Graph\"]] <- NULL\nif (exists(\"stata_graph\")) rm(stata_graph)",
            );
        }
        self.r_new = Some((R_LROC, "lroc".into()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::data_r::translate;
    use crate::reader::read;

    fn r(source: &str) -> Vec<String> {
        translate(&read(source))
            .lines
            .into_iter()
            .map(|l| match l.result {
                Ok(r) => r.code,
                Err(e) => format!("NOT: {}", e.reason),
            })
            .collect()
    }

    #[test]
    fn the_curve_replaces_the_unnamed_graph_and_nograph_leaves_it() {
        let out =
            r("use d\nlogit y x\nlroc\nlroc, nograph\nlroc, name(r1)\nlroc, nograph name(r1)");
        assert!(
            out[2].contains("stata_graphs[[\"Graph\"]] <- NULL"),
            "{out:?}"
        );
        assert!(!out[3].contains("stata_graphs"), "{out:?}");
        // name() is not a look option: it would leave a graph to refer to
        assert!(
            out[4].starts_with("NOT: lroc with the option name"),
            "{out:?}"
        );
        assert!(
            out[5].contains("option name() not allowed (Stata: r(198))"),
            "{out:?}"
        );
    }

    #[test]
    fn errors_are_statas_before_the_data_is_read() {
        let out =
            r("use d\nregress y x\nlroc\nlroc y, beta(b)\nlogit y x\nby g: lroc\nbysort g: lroc");
        assert!(
            out[2].contains("last estimates not found (Stata: r(301))"),
            "{out:?}"
        );
        assert!(out[3].contains("stata_lroc_score"), "{out:?}");
        assert!(
            out[5].contains("lroc may not be combined with by (Stata: r(190))"),
            "{out:?}"
        );
        assert!(out[6].starts_with("NOT: bysort"), "{out:?}");
    }
}
