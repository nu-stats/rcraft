//! `sspace`, `ucm` and `dfactor`: linear Gaussian state-space models as Stata 19 fits them, and `predict` after them
//! (`plan/stata/sspace.md`; the method is StatCraft's decoding of Stata's, probes 549 to 551, our own work, written here
//! in R, `sspace/sspace.R`):
//!
//! - the exact likelihood by the Kalman filter, started from the stationary distribution, or diffuse for a unit root
//!   fixed by constraints (and every `ucm` trend), each diffuse step adding -ln(2 pi)/2;
//! - a search of our own (BFGS, then Newton, the variances on the log scale) to Stata's optimum;
//! - standard errors from the observed information, numerical second derivatives with Richardson steps.
//!
//! Translated: `sspace` with first-order state equations (`state noconstant`) and observation equations with
//! states, a constant and exogenous variables, `constraints()` of the form `[eq]term = #`; `ucm` with `llevel`,
//! `rwalk` or `lltrend` and regressors; `dfactor` with one factor (identity covariance, AR(1) or none) and
//! `ar(1)` on the observation equations. `predict` takes `xb`, `states, equation()`, `trend` and `factors`, each
//! with `smethod(onestep)` (the default) or `smethod(filter)`.
//! Not translated: constants or exogenous variables in state equations, lags beyond the first, `covstate()` and the
//! other covariance options, cycles, seasonals, more than one factor, `from()`, `technique()`, `estat`, `smethod(smooth)`.
//! Stata's own iteration log and e(ic), e(tech_steps), e(technique), e(ilog) are not reproduced.

use super::{is_name, names, no, Opts, Out, Session};
use crate::expr_r::{r_text, Untranslated};
use crate::reader::Command;

pub(super) const SSPACE_HELPER: &str = include_str!("sspace/sspace.R");

/// The macros e() holds as text after the three commands.
const TEXT: &[&str] = &[
    "cmd",
    "title",
    "cmdline",
    "estat_cmd",
    "predict",
    "covariates",
    "marginsnotok",
    "eqnames",
    "state_deps",
    "obser_deps",
    "tvar",
    "depvar",
    "datasignaturevars",
    "tmaxs",
    "tmins",
    "chi2type",
    "opt",
    "vce",
    "method",
    "R_structure",
    "Q_structure",
    "initial_values",
    "properties",
    "model",
    "observ_cov",
    "factor_cov",
    "f_ar",
    "o_ar",
    "factor_deps",
];

/// The parenthesised groups of a command's text, balanced, and what follows them.
fn groups(text: &str) -> Option<(Vec<String>, String)> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while rest.starts_with('(') {
        let mut depth = 0i32;
        let mut end = None;
        for (i, ch) in rest.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let end = end?;
        out.push(rest.get(1..end)?.to_string());
        rest = rest.get(end + 1..)?.trim_start();
    }
    Some((out, rest.to_string()))
}

/// The words of an option list, a word with its parentheses kept whole (`ar(1)`).
fn option_words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for ch in text.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            c if c.is_whitespace() && depth == 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Whether `word` is `full` written out at least `min` letters, with no argument.
fn is_word(word: &str, full: &str, min: usize) -> bool {
    word.len() >= min && full.starts_with(word)
}

/// A state's term: a name, or its first lag `L.name`, `L1.name`.
fn state_term(t: &str) -> bool {
    match t.split_once('.') {
        Some((op, base)) => {
            op.starts_with('L')
                && op
                    .get(1..)
                    .is_some_and(|d| d.chars().all(|c| c.is_ascii_digit()))
                && is_name(base)
        }
        None => is_name(t),
    }
}

fn r_strings(v: &[String]) -> String {
    format!(
        "c({})",
        v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
    )
}

fn r_bool(b: bool) -> &'static str {
    if b {
        "TRUE"
    } else {
        "FALSE"
    }
}

impl Session {
    /// Whether the active results are `sspace`'s, `ucm`'s or `dfactor`'s: predict is translated after them.
    pub(super) fn ss_active(&self) -> Option<(String, String)> {
        if self.estimates.is_some() {
            return None;
        }
        match &self.e_ret {
            super::returned::Returned::Kept { cmd, model, .. }
                if matches!(cmd.as_str(), "sspace" | "ucm" | "dfactor")
                    && self
                        .estimates_lost
                        .as_deref()
                        .is_some_and(|w| w.contains("after sspace, ucm and dfactor")) =>
            {
                Some((cmd.clone(), model.clone()))
            }
            _ => None,
        }
    }

    /// The pieces every one of the three shares: the time variable, the data (after `if` and `in`), and the
    /// arguments the R functions take after the model.
    fn ss_common(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        name: &str,
        plain: bool,
    ) -> Result<(String, String, String), Untranslated> {
        self.no_by(c, name)?;
        let time = match (&self.panel_time, &self.panel) {
            (Some(_), Some(_)) => {
                return Err(no(format!("{name} on panel data is not translated yet")))
            }
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
        for h in [
            "stata_sspace",
            "stata_lsfam",
            "stata_tables",
            "stata_xt_common",
            "stata_fmt",
            "stata_float",
            "stata_coefs",
            "stata_returned",
            "stata_names",
        ] {
            self.helpers.insert(h);
        }
        let delta = self.ts_delta.clone().unwrap_or_else(|| "1".into());
        let fmts = super::xtsum::formats(self, &[time.as_str()]);
        let fmts = fmts
            .strip_prefix(", fmts = ")
            .map_or_else(|| "character(0)".to_string(), str::to_string);
        // a plain varlist (ucm) goes through rows_for, equations in parentheses do not
        let src = if plain {
            self.rows_for(c, out)?
        } else {
            self.ss_frame(c, out)?
        };
        let extra = format!(
            "delta = {delta}, show = {}, tmfmt = stata_xt_fmt_of({src}, {}, {fmts}), log = {}",
            r_bool(!self.quiet(c)),
            r_text(&time),
            r_bool(!opts.has("nolog", 5)),
        );
        Ok((time, src, extra))
    }

    /// The data after `if` and `in`; the equations in parentheses are not a varlist.
    fn ss_frame(&mut self, c: &Command, out: &mut Out) -> Result<String, Untranslated> {
        let d = self.data()?;
        match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                Ok(format!("dplyr::filter({d}, {cond})"))
            }
            None => Ok(d),
        }
    }

    /// What every fit leaves: `b`, `V`, e(), the estimation sample, and what predict needs.
    fn ss_keep(
        &mut self,
        c: &Command,
        out: &mut Out,
        src: &str,
        cmd: &str,
        model: &str,
    ) -> Result<(), Untranslated> {
        out.line("stata_b <- stata_estimates$b");
        out.line("stata_vcov <- stata_estimates$vcov");
        out.line("stata_df <- stata_estimates$df");
        out.line(format!(
            "stata_e <- stata_ss_e(stata_estimates, {})",
            r_text(&super::returned::cmdline(c))
        ));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.e_new = true;
        self.e_text.clear();
        for t in TEXT {
            self.e_text.insert((*t).to_string());
        }
        self.e_ret = super::returned::Returned::Kept {
            cmd: cmd.into(),
            model: model.into(),
            names: &[],
        };
        self.estimates = None;
        self.estimates_lost = Some(
            "margins, test, estat and the rest after sspace, ucm and dfactor are not translated yet (predict is)"
                .into(),
        );
        out.caveat(format!(
            "{cmd} as Stata 19 fits it (StatCraft's decoding: exact Kalman likelihood, observed-information standard errors); \
             a search of RCraft's own, so Stata's iteration log and e(ic) are not reproduced; printed in R's layout"
        ));
        Ok(())
    }

    /// `sspace (state equations) (observation equations) [if] [in] [, constraints(numlist) nolog]`.
    pub(super) fn sspace(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("constraints", 5), ("nolog", 5)]) {
            return Err(no(format!(
                "sspace with the option {o} is not translated yet (covstate(), covobserved(), from(), technique() and the rest)"
            )));
        }
        let Some((eqs, tail)) = groups(&c.parts.main) else {
            return Err(no("sspace: the equations are not in balanced parentheses"));
        };
        if !tail.trim().is_empty() || eqs.is_empty() {
            return Err(no("sspace: only equations in parentheses are translated"));
        }
        let mut r_eqs = Vec::new();
        for body in &eqs {
            let (main, o) = body.split_once(',').unwrap_or((body, ""));
            let words: Vec<&str> = main.split_whitespace().collect();
            let Some((name, terms)) = words.split_first() else {
                return Err(no("sspace: an equation with no name"));
            };
            if !is_name(name) {
                return Err(no(format!("sspace: {name} is not a plain equation name")));
            }
            let (mut state, mut constant, mut error) = (false, true, true);
            for w in option_words(o) {
                if is_word(&w, "state", 5) {
                    state = true;
                } else if is_word(&w, "noconstant", 4) {
                    constant = false;
                } else if is_word(&w, "noerror", 5) {
                    error = false;
                } else {
                    return Err(no(format!(
                        "sspace with the equation option {w} is not translated yet"
                    )));
                }
            }
            for t in terms {
                let ok = if state { state_term(t) } else { is_name(t) };
                if !ok {
                    return Err(no(format!(
                        "sspace: the term {t} is not translated (time-series operators are only the first lags of the states)"
                    )));
                }
            }
            let ts: Vec<String> = terms.iter().map(|t| (*t).to_string()).collect();
            r_eqs.push(format!(
                "list(name = {}, terms = {}, state = {}, constant = {}, error = {})",
                r_text(name),
                if ts.is_empty() {
                    "character(0)".to_string()
                } else {
                    r_strings(&ts)
                },
                r_bool(state),
                r_bool(constant),
                r_bool(error)
            ));
        }
        let (time, src, extra) = self.ss_common(c, opts, out, "sspace", false)?;
        let cons = match opts.arg("constraints", 5) {
            Some(a) => format!("stata_ss_cons(stata_numlist({}))", r_text(a.trim())),
            None => "character(0)".to_string(),
        };
        out.line(format!(
            "stata_estimates <- stata_sspace({src}, {}, list({}), {cons}, {extra})",
            r_text(&time),
            r_eqs.join(", ")
        ));
        self.ss_keep(c, out, &src, "sspace", "")
    }

    /// `ucm depvar [indepvars] [if] [in], model(llevel | rwalk | lltrend) [nolog]`.
    pub(super) fn ucm(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("model", 3), ("nolog", 5)]) {
            return Err(no(format!(
                "ucm with the option {o} is not translated yet (cycles, seasonals, vce(), from() and the rest)"
            )));
        }
        let model = match opts.arg("model", 3).map(|m| m.trim().to_string()) {
            Some(m) if matches!(m.as_str(), "llevel" | "rwalk" | "lltrend") => m,
            Some(m) => {
                return Err(no(format!(
                    "ucm with model({m}) is not translated yet (llevel, rwalk and lltrend are)"
                )))
            }
            None => return Err(no("ucm without model() is not translated yet")),
        };
        let vars = names(&c.parts.main).map_err(|e| {
            no(format!(
                "{} (factor variables and time-series operators with ucm are not translated yet)",
                e.reason
            ))
        })?;
        let Some((dep, regs)) = vars.split_first() else {
            return Err(no("varlist required (Stata: r(100))"));
        };
        let (time, src, extra) = self.ss_common(c, opts, out, "ucm", true)?;
        out.line(format!(
            "stata_estimates <- stata_ucm({src}, {}, {}, {}, {}, {extra})",
            r_text(&time),
            r_text(dep),
            if regs.is_empty() {
                "character(0)".to_string()
            } else {
                r_strings(regs)
            },
            r_text(&model)
        ));
        self.ss_keep(c, out, &src, "ucm", &model)
    }

    /// `dfactor (depvars = , noconstant [ar(1)]) [(factor = , ar(1))] [if] [in] [, nolog]`.
    pub(super) fn dfactor(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("nolog", 5)]) {
            return Err(no(format!(
                "dfactor with the option {o} is not translated yet (covariance(), constraints(), from() and the rest)"
            )));
        }
        let Some((eqs, tail)) = groups(&c.parts.main) else {
            return Err(no("dfactor: the equations are not in balanced parentheses"));
        };
        if !tail.trim().is_empty() || eqs.is_empty() {
            return Err(no("dfactor: only equations in parentheses are translated"));
        }
        if eqs.len() > 2 {
            return Err(no(
                "dfactor with more than one factor equation is not translated yet",
            ));
        }
        // each equation: its left side, right side and options
        let read = |body: &str| -> (String, String, Vec<String>) {
            let (main, o) = body.split_once(',').unwrap_or((body, ""));
            let (lhs, rhs) = main.split_once('=').unwrap_or((main, ""));
            (
                lhs.trim().to_string(),
                rhs.trim().to_string(),
                option_words(o),
            )
        };
        let ar_of = |words: &[String]| -> Result<(u32, bool), Untranslated> {
            let (mut ar, mut nocons) = (0u32, false);
            for w in words {
                if is_word(w, "noconstant", 4) {
                    nocons = true;
                } else if let Some(a) = w.strip_prefix("ar(").and_then(|r| r.strip_suffix(')')) {
                    if a.trim() != "1" {
                        return Err(no("dfactor with ar() above 1 is not translated yet"));
                    }
                    ar = 1;
                } else {
                    return Err(no(format!(
                        "dfactor with the equation option {w} is not translated yet"
                    )));
                }
            }
            Ok((ar, nocons))
        };
        let first = eqs.first().map(String::as_str).unwrap_or("");
        let (lhs, rhs, words) = read(first);
        if !rhs.is_empty() || !first.contains('=') {
            return Err(no(
                "dfactor with exogenous variables in the observation equations is not translated yet",
            ));
        }
        let (o_ar, nocons) = ar_of(&words)?;
        if !nocons {
            return Err(no(
                "dfactor with constants in the observation equations is not translated yet; specify noconstant",
            ));
        }
        let ys = names(&lhs).map_err(|e| {
            no(format!(
                "{} (only plain variable names are translated with dfactor)",
                e.reason
            ))
        })?;
        if ys.is_empty() {
            return Err(no("dfactor: no dependent variables"));
        }
        let mut factor = "NULL".to_string();
        let mut f_ar = 0u32;
        let mut has_factor = false;
        if let Some(second) = eqs.get(1) {
            let (flhs, frhs, fwords) = read(second);
            if !second.contains('=') || !frhs.is_empty() {
                return Err(no("dfactor: the factor equation is `(f = , ar(1))`"));
            }
            let f: Vec<&str> = flhs.split_whitespace().collect();
            let [f] = f.as_slice() else {
                return Err(no(
                    "dfactor with more than one factor is not translated yet",
                ));
            };
            if !is_name(f) {
                return Err(no(format!("dfactor: {f} is not a plain factor name")));
            }
            let (a, fcons) = ar_of(&fwords)?;
            if fcons || o_ar != 0 {
                return Err(no(
                    "dfactor with a factor and ar() on the observation equations together is not translated yet",
                ));
            }
            f_ar = a;
            factor = r_text(f);
            has_factor = true;
        } else if o_ar == 0 {
            return Err(no(
                "dfactor with neither a factor nor ar() is not translated yet",
            ));
        }
        let (time, src, extra) = self.ss_common(c, opts, out, "dfactor", false)?;
        out.line(format!(
            "stata_estimates <- stata_dfactor({src}, {}, {}, factor = {factor}, f_ar = {f_ar}, o_ar = {o_ar}, {extra})",
            r_text(&time),
            r_strings(&ys)
        ));
        self.ss_keep(
            c,
            out,
            &src,
            "dfactor",
            if has_factor { "DF" } else { "SUR" },
        )
    }

    /// `predict [type] newvar [if] [in], xb | states equation(name) | trend | factors [smethod(onestep | filter)]`.
    pub(super) fn ss_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        cmd: &str,
        model: &str,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("xb", 2),
            ("states", 3),
            ("trend", 3),
            ("factors", 3),
            ("equation", 2),
            ("smethod", 3),
        ]) {
            return Err(no(format!(
                "predict after {cmd} with the option {o} is not translated yet"
            )));
        }
        let named: Vec<&str> = [("xb", 2), ("states", 3), ("trend", 3), ("factors", 3)]
            .iter()
            .filter(|(o, m)| opts.has(o, *m))
            .map(|(o, _)| *o)
            .collect();
        let what = match named.as_slice() {
            [w] => *w,
            [] => {
                return Err(no(format!(
                    "predict after {cmd} without a statistic is not translated yet (name xb, states, trend or factors)"
                )))
            }
            _ => {
                out.line("stop(\"invalid syntax (Stata: r(198))\", call. = FALSE)");
                return Ok(());
            }
        };
        let allowed = match cmd {
            "sspace" => ["xb", "states"].as_slice(),
            "ucm" => ["xb", "trend"].as_slice(),
            _ => ["xb", "factors"].as_slice(),
        };
        if !allowed.contains(&what) {
            return Err(no(format!(
                "predict, {what} after {cmd} is not translated (it is not a statistic of that command, or not translated yet)"
            )));
        }
        if what == "factors" && model != "DF" {
            return Err(no(
                "predict, factors after a dfactor without a factor is not translated",
            ));
        }
        let equation = match opts.arg("equation", 2) {
            Some(e) => {
                let e = e.trim().to_string();
                if what != "states" || !is_name(&e) {
                    return Err(no(
                        "predict with equation() is translated for states and a state's name only",
                    ));
                }
                r_text(&e)
            }
            None if what == "states" => {
                return Err(no("predict, states needs equation(state name) here"))
            }
            None => "NULL".to_string(),
        };
        let filtered = match opts.arg("smethod", 3).map(|m| m.trim().to_string()) {
            None => false,
            Some(m) if m == "onestep" => false,
            Some(m) if m == "filter" => true,
            Some(m) => {
                return Err(no(format!(
                    "predict with smethod({m}) is not translated yet (onestep and filter are)"
                )))
            }
        };
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
        let v = format!(
            "stata_ss_predict(stata_estimates, {base}, {}, equation = {equation}, filtered = {})",
            r_text(what),
            r_bool(filtered)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_are_balanced_and_keep_nested_parentheses() {
        let (g, rest) = groups("(y1 y2 = , noconstant) (f = , ar(1))").unwrap_or_default();
        assert_eq!(g, vec!["y1 y2 = , noconstant", "f = , ar(1)"]);
        assert!(rest.is_empty());
        assert!(groups("(u L.u, state").is_none());
        let (g, rest) = groups("(u L.u) tail").unwrap_or_default();
        assert_eq!(g.len(), 1);
        assert_eq!(rest, "tail");
    }

    #[test]
    fn option_words_keep_their_arguments_whole() {
        assert_eq!(
            option_words("noconstant ar(1)"),
            vec!["noconstant", "ar(1)"]
        );
        assert_eq!(option_words("ar(1 2) x"), vec!["ar(1 2)", "x"]);
        assert!(is_word("noconst", "noconstant", 4));
        assert!(!is_word("nocon", "noconstant", 8));
    }

    #[test]
    fn a_state_term_is_a_name_or_its_first_lag_form() {
        assert!(state_term("u"));
        assert!(state_term("L.u"));
        assert!(state_term("L2.u"));
        assert!(!state_term("D.u"));
        assert!(!state_term("L.u.v"));
    }
}
