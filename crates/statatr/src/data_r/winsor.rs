//! `winsor` (Nick Cox's winsor, SSC), as observed in Stata 19 on made-up
//! data (`plan/stata/winsor.md`); the add-on's program was not read.
//!
//! - Over the nonmissing values of the rows of `if`/`in` (N of them), the h
//!   lowest become the (h+1)th smallest and the h highest the (h+1)th
//!   largest (`high` or `low` for one end only); h is `h()` or
//!   floor(`p()` N). Ties change nothing in that rule.
//! - The new variable has the old one's type and format and is missing
//!   outside `if`/`in`; its label is "x, Winsorized fraction .1" or
//!   "x, Winsorized extreme 2", with ", high only" or ", low only".
//! - Errors (r(198)): no generate(), both or neither of h() and p() (or 0),
//!   p() of .5 or more ("p() too high"), h of 0 ("0 values to be
//!   Winsorized"), 2h of N or more ("11 values to be Winsorized, 22 in
//!   data"); a generate() name in use is r(110).

use super::*;

pub(super) const WINSOR_HELPER: &str = r##"stata_winsor <- function(d, v, gen, rows = NULL, p = NULL, h = NULL, side = "both", label = "") {
  if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  if (gen %in% names(d)) stop("generate() should give new variable name (Stata: r(110))", call. = FALSE)
  x <- d[[v]]
  if (is.character(x)) stop("winsor of a string variable is not translated yet", call. = FALSE)
  y <- as.numeric(unclass(x))
  use <- if (is.null(rows)) rep(TRUE, nrow(d)) else (!is.na(rows) & rows)
  ok <- use & !is.na(y)
  n <- sum(ok)
  if (!is.null(p)) h <- floor(p * n)
  if (h < 1) stop("0 values to be Winsorized (Stata: r(198))", call. = FALSE)
  if (2 * h >= n) stop(h, " values to be Winsorized, ", n, " in data (Stata: r(198))", call. = FALSE)
  s <- sort(y[ok])
  out <- y
  if (side != "high") out <- pmax(out, s[h + 1])
  if (side != "low") out <- pmin(out, s[n - h])
  out[!use] <- NA
  attributes(out) <- NULL
  d[[gen]] <- structure(out, label = label, format.stata = attr(x, "format.stata", exact = TRUE))
  d
}"##;

impl Session {
    /// `winsor varname [if] [in], generate(newvar) {p(#)|h(#)} [high|low]`.
    pub(super) fn winsor(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("generate", 3),
            ("p", 1),
            ("h", 1),
            ("highonly", 4),
            ("lowonly", 3),
        ]) {
            return Err(no(format!("option {o} not allowed (Stata: r(198))")));
        }
        let stop = |out: &mut Out, m: &str| {
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{m} (Stata: r(198))"))
            ));
        };
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        let v = match vars.as_slice() {
            [v] if is_name(v) => v.to_string(),
            [] => return Err(no("varname required (Stata: r(100))")),
            [_] => {
                return Err(no(
                    "winsor: a variable's name is translated (patterns are when the do-file runs)",
                ))
            }
            _ => {
                out.line("stop(\"too many variables specified (Stata: r(103))\", call. = FALSE)");
                return Ok(());
            }
        };
        let Some(gen) = opts.arg("generate", 3).map(|g| g.trim().to_string()) else {
            stop(out, "option generate() required");
            return Ok(());
        };
        if !is_name(&gen) {
            return Err(no(format!(
                "winsor, generate({gen}): one new name is needed"
            )));
        }
        let num = |o: &str| -> Result<Option<f64>, Untranslated> {
            match opts.arg(o, 1) {
                None => Ok(None),
                Some(a) => a
                    .trim()
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| no(format!("winsor, {o}({a}): only a number is translated"))),
            }
        };
        let (p, h) = (num("p")?, num("h")?);
        let (p_arg, h_arg) = match (p, h) {
            (Some(_), Some(_)) => {
                stop(out, "use either h() option or p() option");
                return Ok(());
            }
            (Some(p), None) if p > 0.0 => {
                if p >= 0.5 {
                    stop(out, "p() too high");
                    return Ok(());
                }
                (r_number(p), "NULL".to_string())
            }
            (None, Some(h)) if h > 0.0 => {
                if h.fract() != 0.0 {
                    return Err(no(format!("winsor, h({h}): a whole number is translated")));
                }
                ("NULL".to_string(), r_number(h))
            }
            _ => {
                stop(out, "h() or p() option required, h( ) or p() > 0");
                return Ok(());
            }
        };
        let side = match (opts.has("highonly", 4), opts.has("lowonly", 3)) {
            (true, true) => return Err(no("winsor with both high and low is not translated yet")),
            (true, false) => "high",
            (false, true) => "low",
            _ => "both",
        };
        // the label: "x, Winsorized fraction .1" or "... extreme 2", one end said
        let amount = match (p, h) {
            (Some(p), _) => format!("fraction {}", crate::dofile::format::g(p, 10).trim()),
            (_, Some(h)) => format!("extreme {h}"),
            _ => String::new(),
        };
        let only = match side {
            "high" => ", high only",
            "low" => ", low only",
            _ => "",
        };
        let label = format!("{v}, Winsorized {amount}{only}");
        let d = self.data()?;
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                format!(", rows = dplyr::pull(dplyr::mutate({d}, stata_rows = {cond}), stata_rows)")
            }
            None => String::new(),
        };
        self.helpers.insert("stata_winsor");
        out.package("dplyr");
        out.line(format!(
            "{d} <- stata_winsor({d}, {}, {}{rows}, p = {p_arg}, h = {h_arg}, side = {}, label = {})",
            r_text(&v),
            r_text(&gen),
            r_text(side),
            r_text(&label)
        ));
        self.kinds.insert(gen.clone(), Kind::Number);
        match self.storage.get(&v).copied() {
            Some(s) => {
                self.storage.insert(gen, s);
            }
            None => {
                self.storage.remove(&gen);
            }
        }
        Ok(())
    }
}
