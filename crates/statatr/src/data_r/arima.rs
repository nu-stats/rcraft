//! `arima`, regression with ARMA disturbances, as Stata 19 fits it, and `predict` after it
//! (`plan/stata/arima.md`; the method is StatCraft's decoding of Stata's, probes 157, 271, 576,
//! 582 and 586 to 589, our own work, written here in R, `arima/arima.R`):
//!
//! - the exact likelihood by the Kalman filter, started from the stationary distribution;
//! - Stata's starting values (least squares; AR terms by least squares of the residuals on their
//!   lags; MA terms by Hannan and Rissanen's two regressions) and its path (BHHH for five
//!   iterations, then BFGS, with `ml`'s stopping rule);
//! - standard errors from the outer product of the scores (OPG).
//!
//! Not translated: time-series operators in the varlist, weights, `constraints()`,
//! `vce(robust)`, `from()`, `technique()`, `condition`, `diffuse`, seasonal differencing,
//! missing values or gaps inside the sample, forecasts beyond the data, `stdp`, `t0()`.

use super::{is_name, names, no, Opts, Out, Session};
use crate::expr_r::{r_number, r_text, Untranslated};
use crate::reader::Command;

pub(super) const ARIMA_HELPER: &str = include_str!("arima/arima.R");

/// A numlist of positive whole numbers (`1`, `1 2`, `1/3 5`).
fn lags(text: &str) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for t in text.split_whitespace() {
        match t.split_once('/') {
            Some((a, b)) => {
                let (a, b) = (a.trim().parse::<u32>().ok()?, b.trim().parse::<u32>().ok()?);
                if a == 0 || b < a || b - a > 1000 {
                    return None;
                }
                out.extend(a..=b);
            }
            None => {
                let v = t.parse::<u32>().ok()?;
                if v == 0 {
                    return None;
                }
                out.push(v);
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    Some(out)
}

fn r_ints(v: &[u32]) -> String {
    if v.is_empty() {
        "integer(0)".into()
    } else {
        format!(
            "c({})",
            v.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
        )
    }
}

/// The whole numbers in `a,b,c`.
fn comma_ints(text: &str) -> Option<Vec<u32>> {
    text.split(',').map(|t| t.trim().parse().ok()).collect()
}

fn bad(what: &str) -> Untranslated {
    no(format!("{what} invalid (Stata: r(198))"))
}

impl Session {
    /// Whether the active results are arima's: predict is translated after it, the other
    /// post-estimation commands are not yet.
    pub(super) fn arima_active(&self) -> bool {
        self.estimates.is_none()
            && matches!(&self.e_ret, super::returned::Returned::Kept { cmd, .. } if cmd == "arima")
            && self
                .estimates_lost
                .as_deref()
                .is_some_and(|w| w.contains("after arima"))
    }

    /// `arima depvar [indepvars] [if] [in] [, ar() ma() arima(p,d,q) sarima(P,D,Q,s) mar() mma()
    /// noconstant level() nolog trace gradient tolerance() ltolerance() nrtolerance() iterate()]`.
    pub(super) fn arima(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "arima")?;
        if let Some(o) = opts.other(&[
            ("ar", 2),
            ("ma", 2),
            ("arima", 5),
            ("sarima", 6),
            ("mar", 3),
            ("mma", 3),
            ("noconstant", 3),
            ("level", 3),
            ("nolog", 5),
            ("trace", 5),
            ("gradient", 4),
            ("tolerance", 3),
            ("ltolerance", 4),
            ("nrtolerance", 4),
            ("iterate", 4),
        ]) {
            return Err(no(format!(
                "arima with the option {o} is not translated yet"
            )));
        }
        if c.parts.main.trim().is_empty() {
            return Err(no(
                "arima alone (replaying the last fit) is not translated yet",
            ));
        }
        let vars = names(&c.parts.main).map_err(|e| {
            no(format!(
                "{} (factor variables and time-series operators with arima are not translated yet)",
                e.reason
            ))
        })?;
        let Some((dep, regs)) = vars.split_first() else {
            return Err(no("varlist required (Stata: r(100))"));
        };
        let time = match (&self.panel_time, &self.panel) {
            (Some(_), Some(_)) => return Err(no("arima on panel data is not translated yet")),
            (Some(t), None) => t.clone(),
            (None, _) => {
                return Err(no(if self.ts_unknown {
                    "the data were read from a file whose tsset settings are not known here; set them again with tsset"
                } else {
                    "time variable not set, use tsset varname ... (Stata: r(111))"
                }))
            }
        };
        if !is_name(&time) {
            return Err(no("the time variable must be a plain name"));
        }
        let list = |name: &str, min: usize| -> Result<Vec<u32>, Untranslated> {
            match opts.arg(name, min) {
                None => Ok(Vec::new()),
                Some(a) => lags(&a).ok_or_else(|| bad(&format!("{name}()"))),
            }
        };
        let mut ar = list("ar", 2)?;
        let mut ma = list("ma", 2)?;
        let mut nd = 0u32;
        if let Some(a) = opts.arg("arima", 5) {
            if !ar.is_empty() || !ma.is_empty() {
                return Err(no("arima() with ar() or ma() is not translated yet"));
            }
            let Some([p, d, q]) = comma_ints(&a)
                .as_deref()
                .and_then(|v| <[u32; 3]>::try_from(v).ok())
            else {
                return Err(bad("arima()"));
            };
            ar = (1..=p).collect();
            ma = (1..=q).collect();
            nd = d;
        }
        // seasonal terms: mar(numlist, s), mma(numlist, s), sarima(P,D,Q,s)
        let seasonal = |name: &str, min: usize| -> Result<Option<(Vec<u32>, u32)>, Untranslated> {
            let Some(a) = opts.arg(name, min) else {
                return Ok(None);
            };
            let (l, s) = a
                .rsplit_once(',')
                .ok_or_else(|| bad(&format!("{name}()")))?;
            let s: u32 = s.trim().parse().map_err(|_| bad(&format!("{name}()")))?;
            if s < 2 {
                return Err(bad(&format!("{name}()")));
            }
            let l = lags(l).ok_or_else(|| bad(&format!("{name}()")))?;
            Ok(Some((l, s)))
        };
        let mut period = 0u32;
        let mut sar = Vec::new();
        let mut sma = Vec::new();
        for (name, slot) in [("mar", &mut sar), ("mma", &mut sma)] {
            if let Some((l, s)) = seasonal(name, 3)? {
                if period != 0 && period != s {
                    return Err(no(
                        "mar() and mma() with different periods are not translated yet",
                    ));
                }
                period = s;
                *slot = l;
            }
        }
        if let Some(a) = opts.arg("sarima", 6) {
            if period != 0 {
                return Err(no("sarima() with mar() or mma() is not translated yet"));
            }
            let Some([p, d, q, s]) = comma_ints(&a)
                .as_deref()
                .and_then(|v| <[u32; 4]>::try_from(v).ok())
            else {
                return Err(bad("sarima()"));
            };
            if d > 0 {
                return Err(no(
                    "arima with seasonal differencing in sarima() is not translated yet",
                ));
            }
            if s < 2 {
                return Err(bad("sarima()"));
            }
            sar = (1..=p).collect();
            sma = (1..=q).collect();
            period = s;
        }
        let level = match opts.arg("level", 3) {
            None => 0.95,
            Some(l) => match l.trim().parse::<f64>() {
                Ok(v) if (10.0..=99.99).contains(&v) => v / 100.0,
                _ => {
                    return Err(no(format!(
                        "level({l}) is not a valid confidence level (Stata: r(198))"
                    )))
                }
            },
        };
        let number = |name: &str, min: usize, default: f64| -> Result<f64, Untranslated> {
            match opts.arg(name, min) {
                None => Ok(default),
                Some(a) => a
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .ok_or_else(|| bad(&format!("{name}()"))),
            }
        };
        let tol = number("tolerance", 3, 1e-6)?;
        let ltol = number("ltolerance", 4, 1e-7)?;
        let nrtol = number("nrtolerance", 4, 1e-5)?;
        let iterate = number("iterate", 4, 300.0)?;
        if iterate.fract() != 0.0 || iterate > 100_000.0 {
            return Err(bad("iterate()"));
        }
        let constant = !opts.has("noconstant", 3);
        let src = self.rows_for(c, out)?;
        for h in [
            "stata_arima",
            "stata_xt_common",
            "stata_fmt",
            "stata_float",
            "stata_coefs",
            "stata_returned",
            "stata_names",
        ] {
            self.helpers.insert(h);
        }
        let xs = regs
            .iter()
            .map(|v| r_text(v))
            .collect::<Vec<_>>()
            .join(", ");
        let delta = self.ts_delta.clone().unwrap_or_else(|| "1".into());
        let fmts = super::xtsum::formats(self, &[time.as_str()]);
        let fmts = fmts
            .strip_prefix(", fmts = ")
            .map_or_else(|| "character(0)".to_string(), str::to_string);
        let call = format!(
            "stata_arima({src}, {}, c({xs}), {}, delta = {delta}, ar = {}, ma = {}, sar = {}, sma = {}, period = {period}, nd = {nd}, \
             constant = {}, level = {}, tol = {}, ltol = {}, nrtol = {}, iterate = {}, log = {}, show = {}, \
             tmfmt = stata_xt_fmt_of({src}, {}, {fmts}))",
            r_text(dep),
            r_text(&time),
            r_ints(&ar),
            r_ints(&ma),
            r_ints(&sar),
            r_ints(&sma),
            if constant { "TRUE" } else { "FALSE" },
            r_number(level),
            r_number(tol),
            r_number(ltol),
            r_number(nrtol),
            r_number(iterate),
            if opts.has("nolog", 5) { "FALSE" } else { "TRUE" },
            if self.quiet(c) { "FALSE" } else { "TRUE" },
            r_text(&time),
        );
        out.line(format!("stata_estimates <- {call}"));
        out.line("stata_b <- stata_estimates$b");
        out.line("stata_vcov <- stata_estimates$vcov");
        out.line("stata_df <- stata_estimates$df");
        out.line(format!(
            "stata_e <- stata_arima_e(stata_estimates, {})",
            r_text(&super::returned::cmdline(c))
        ));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            &src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.e_new = true;
        self.e_ret = super::returned::Returned::Kept {
            cmd: "arima".into(),
            model: String::new(),
            names: &[],
        };
        self.estimates = None;
        self.estimates_lost = Some(
            "margins, test, estat and the rest after arima are not translated yet (predict is)"
                .into(),
        );
        out.caveat(
            "arima as Stata 19 fits it (StatCraft's decoding: exact likelihood, Stata's starting values and \
             BHHH then BFGS path, OPG standard errors); printed in R's layout, not Stata's table",
        );
        Ok(())
    }

    /// `predict newvar [, xb y residuals yresiduals mse structural dynamic(t)]` after arima.
    pub(super) fn arima_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("xb", 2),
            ("y", 1),
            ("residuals", 1),
            ("yresiduals", 2),
            ("mse", 3),
            ("structural", 3),
            ("dynamic", 3),
        ]) {
            // stdp and t0() are Stata's, not translated; any other word is not an option of it
            if ["stdp", "t0"]
                .iter()
                .any(|k| k.starts_with(o.as_str()) && !o.is_empty())
            {
                return Err(no(format!(
                    "predict after arima with the option {o} is not translated yet"
                )));
            }
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("option {o} not allowed (Stata: r(198))"))
            ));
            return Ok(());
        }
        let kinds = [
            ("xb", 2, "xb"),
            ("y", 1, "y"),
            ("residuals", 1, "residuals"),
            ("yresiduals", 2, "yresiduals"),
            ("mse", 3, "mse"),
        ];
        let named: Vec<&str> = kinds
            .iter()
            .filter(|(o, m, _)| opts.has(o, *m))
            .map(|(_, _, k)| *k)
            .collect();
        if named.len() > 1 {
            out.line("stop(\"invalid syntax (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        }
        let what = named.first().copied().unwrap_or("xb");
        let dynamic = match opts.arg("dynamic", 3) {
            None => "NULL".to_string(),
            Some(t) => match t.trim().parse::<f64>() {
                Ok(v) if v.is_finite() => r_number(v),
                _ => return Err(no(
                    "predict, dynamic() is translated with a number only (the time as its value)",
                )),
            },
        };
        let time = self
            .panel_time
            .clone()
            .ok_or_else(|| no("time variable not set, use tsset varname ... (Stata: r(111))"))?;
        let mut words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let ty = match words.first() {
            Some(t) if super::storage(t).is_some() => {
                let t = *t;
                words.remove(0);
                t
            }
            _ => "float",
        };
        let st = super::storage(ty).ok_or_else(|| no(format!("{ty} is not a storage type")))?;
        if st == super::Storage::Str {
            return Err(no("predict makes a numeric variable, not a string"));
        }
        let [w] = words.as_slice() else {
            return Err(no("predict makes one new variable"));
        };
        if !is_name(w) {
            return Err(no(format!(
                "{w}: only a plain new variable name is translated"
            )));
        }
        let base = self.data()?;
        out.line(format!(
            "if ({} %in% names({base})) stop(\"variable {w} already defined (Stata: r(110))\", call. = FALSE)",
            r_text(w)
        ));
        if named.is_empty() && !self.quiet(c) {
            out.line("cat(\"(option xb assumed; predicted values)\\n\")");
        }
        let v = format!(
            "stata_arima_predict(stata_estimates, {base}, {}, structural = {}, dynamic = {dynamic}, time = {})",
            r_text(what),
            if opts.has("structural", 3) { "TRUE" } else { "FALSE" },
            r_text(&time)
        );
        let v = super::stored(&v, st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = {v})",
            super::r_name(w)
        ));
        self.kinds.insert(w.to_string(), super::Kind::Number);
        self.storage.insert(w.to_string(), st);
        Ok(())
    }
}
