//! `mprobit` and `slogit` (and predict after them): maximum likelihood on the numerical derivatives
//! of the log likelihood, `multi/choice.R`.

use super::super::*;
use super::keep_e;

fn plain(text: &str) -> Option<Vec<String>> {
    let v: Vec<String> = text.split_whitespace().map(str::to_string).collect();
    (!v.is_empty() && v.iter().all(|w| is_name(w))).then_some(v)
}

fn r_vec(v: &[String]) -> String {
    let q: Vec<String> = v.iter().map(|s| r_text(s)).collect();
    format!("c({})", q.join(", "))
}

/// The text names of e() after these commands, `out1` to `out20` included.
fn texts(extra: &[&str]) -> (Vec<String>, Vec<String>) {
    let t: Vec<String> = extra.iter().map(|s| (*s).to_string()).collect();
    let o: Vec<String> = (1..=20).map(|i| format!("out{i}")).collect();
    (t, o)
}

impl Session {
    /// `mprobit` and `slogit`: `depvar indepvars [if] [in] [, baseoutcome(#) nolog ...]`.
    pub(in crate::data_r) fn choice_fit(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        cmd: &'static str,
    ) -> Result<(), Untranslated> {
        self.no_by(c, cmd)?;
        let known: &[(&str, usize)] = if cmd == "mprobit" {
            &[("baseoutcome", 2), ("probitparam", 4), ("nolog", 3)]
        } else {
            &[("baseoutcome", 2), ("dimension", 3), ("nolog", 3)]
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "{cmd} with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no(format!("{cmd} with weights is not translated yet")));
        }
        let Some(vars) = plain(&c.parts.main) else {
            return Err(no(format!(
                "{cmd} with variables not written out (factor variables, ranges, abbreviations) is not translated yet"
            )));
        };
        let Some((y, xs)) = vars.split_first() else {
            return Err(no(format!("{cmd} needs a dependent variable")));
        };
        if xs.is_empty() {
            return Err(no(format!("{cmd} needs independent variables")));
        }
        let base = match opts.arg("baseoutcome", 2) {
            None => String::new(),
            Some(b) => {
                let v = b
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| no(format!("{cmd}: baseoutcome({b}) needs a number")))?;
                format!(", base = {}", r_number(v))
            }
        };
        let rows = self.rows_for(c, out)?;
        for h in ["stata_manova", "stata_choice", "stata_fmt"] {
            self.helpers.insert(h);
        }
        let show = if self.quiet(c) { ", show = FALSE" } else { "" };
        let cmdline = r_text(&super::super::returned::cmdline(c));
        if cmd == "mprobit" {
            out.line(format!(
                "stata_e <- stata_mprobit({rows}, {}, {}{base}, probitparam = {}, cmdline = {cmdline}{show})",
                r_text(y),
                r_vec(xs),
                if opts.has("probitparam", 4) { "TRUE" } else { "FALSE" },
            ));
        } else {
            let dim = match opts.arg("dimension", 3) {
                None => 1u32,
                Some(d) => d
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| no(format!("slogit: dimension({d}) needs a whole number")))?,
            };
            out.line(format!(
                "stata_e <- stata_slogit({rows}, {}, {}, dim = {dim}L{base}, cmdline = {cmdline}{show})",
                r_text(y),
                r_vec(xs),
            ));
        }
        let (mut t, o) = texts(if cmd == "mprobit" {
            &[
                "cmdline",
                "cmd",
                "marginsdefault",
                "marginsnotok",
                "predict",
                "title",
                "depvar",
                "indvars",
                "outeqs",
                "chi2type",
                "deriv_useminbound",
                "opt",
                "vce",
                "user",
                "ml_method",
                "technique",
                "which",
                "properties",
            ]
        } else {
            &[
                "cmdline",
                "cmd",
                "predict",
                "marginsnotok",
                "footnote",
                "marginsdefault",
                "title",
                "indvars",
                "chi2type",
                "deriv_useminbound",
                "opt",
                "vce",
                "user",
                "ml_method",
                "technique",
                "which",
                "depvar",
                "properties",
            ]
        });
        t.extend(o);
        let refs: Vec<&str> = t.iter().map(String::as_str).collect();
        keep_e(
            self,
            cmd,
            &refs,
            &[],
            &format!("{cmd}'s results are not a model RCraft's test, lincom or margins read; predict is translated [multi]"),
            out,
        );
        Ok(())
    }

    /// `predict newvars [if] [in] [, pr outcome(#) xb]` after `mprobit` or `slogit`.
    pub(in crate::data_r) fn choice_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        cmd: &'static str,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("pr", 2), ("outcome", 3), ("xb", 2)]) {
            return Err(no(format!(
                "predict after {cmd} with the option {o} is not translated yet"
            )));
        }
        let mut words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let ty = match words.first() {
            Some(t) if storage(t).is_some() => {
                let t = *t;
                words.remove(0);
                t
            }
            _ => "float",
        };
        let st = storage(ty).ok_or_else(|| no(format!("{ty} is not a storage type")))?;
        if st == Storage::Str {
            return Err(no("predict makes numeric variables, not strings"));
        }
        if words.is_empty() || words.iter().any(|w| !is_name(w)) {
            return Err(no(
                "predict after this model: plain new variable names are translated",
            ));
        }
        let xb = opts.has("xb", 2);
        let outcome = opts.arg("outcome", 3).map(|o| o.trim().to_string());
        if xb && cmd == "mprobit" {
            return Err(no("predict, xb after mprobit is not translated yet"));
        }
        let base = self.data()?;
        for w in &words {
            out.line(format!(
                "if ({} %in% names({base})) stop(\"variable {w} already defined (Stata: r(110))\", call. = FALSE)",
                r_text(w)
            ));
        }
        self.helpers.insert("stata_manova");
        self.helpers.insert("stata_choice");
        self.helpers.insert("stata_float");
        out.package("dplyr");
        let call = |outc: &str| {
            if cmd == "mprobit" {
                "stata_mprobit_pr(".to_string() + &base + ", stata_e)"
            } else if xb {
                format!("stata_slogit_pred({base}, stata_e, \"xb\", {outc})")
            } else {
                format!("stata_slogit_pred({base}, stata_e)")
            }
        };
        if !xb && !self.quiet(c) && !opts.has("pr", 2) {
            out.line("cat(\"(option pr assumed; predicted probabilities)\\n\")");
        }
        let cond = self.condition(c, out)?;
        let wrap = |v: String, helpers: &mut BTreeSet<&'static str>, out: &mut Out| {
            let v = stored(&v, st, helpers, out);
            match &cond {
                Some(cd) => format!("dplyr::if_else({cd}, {v}, NA_real_)"),
                None => v,
            }
        };
        if xb || outcome.is_some() {
            let [w] = words.as_slice() else {
                return Err(no("predict with outcome() or xb makes one new variable"));
            };
            let k = outcome.clone().unwrap_or_else(|| "1".to_string());
            let k = k.trim_start_matches('#').to_string();
            let Ok(kn) = k.parse::<u32>() else {
                return Err(no("predict outcome(): the outcome's number is translated"));
            };
            let col = format!("{}[, {kn}L]", {
                if xb {
                    // a vector already
                    String::new()
                } else {
                    call(&k)
                }
            });
            let value = if xb { call(&format!("{kn}L")) } else { col };
            let value = wrap(value, &mut self.helpers, out);
            out.line(format!(
                "{base} <- dplyr::mutate({base}, {} = {value})",
                r_name(w)
            ));
            self.kinds.insert((*w).to_string(), Kind::Number);
            self.storage.insert((*w).to_string(), st);
        } else {
            out.line(format!("stata_pr <- {}", call("1L")));
            out.line(format!(
                "if (ncol(stata_pr) != {}) stop(\"predict makes one new variable for each outcome (Stata: r(102))\", call. = FALSE)",
                words.len()
            ));
            for (i, w) in words.iter().enumerate() {
                let value = wrap(format!("stata_pr[, {}L]", i + 1), &mut self.helpers, out);
                out.line(format!(
                    "{base} <- dplyr::mutate({base}, {} = {value})",
                    r_name(w)
                ));
                self.kinds.insert((*w).to_string(), Kind::Number);
                self.storage.insert((*w).to_string(), st);
            }
            out.line("rm(stata_pr)");
        }
        Ok(())
    }
}
