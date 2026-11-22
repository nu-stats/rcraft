//! `nlogitgen`, `nlogittree` and `nlogit` (a two-level, RUM-consistent nested logit), and predict
//! after `nlogit`: `multi/nlogit.R`.

use super::super::*;
use super::keep_e;

/// The words of an option list, `name(arg)` kept whole.
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

fn plain(text: &str) -> Option<Vec<String>> {
    let v: Vec<String> = text.split_whitespace().map(str::to_string).collect();
    (v.iter().all(|w| is_name(w))).then_some(v)
}

fn r_vec(v: &[String]) -> String {
    if v.is_empty() {
        return "character(0)".to_string();
    }
    let q: Vec<String> = v.iter().map(|s| r_text(s)).collect();
    format!("c({})", q.join(", "))
}

/// `word(arg)`: the argument when the word is that option.
fn arg_of<'a>(word: &'a str, name: &str) -> Option<&'a str> {
    word.strip_prefix(name)?
        .strip_prefix('(')?
        .strip_suffix(')')
}

impl Session {
    /// `nlogitgen newvar = altvar(group: alt | alt, group: alt | alt)`.
    pub(in crate::data_r) fn nlogitgen(
        &mut self,
        c: &Command,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "nlogitgen")?;
        let text = super::super::returned::after_name(c);
        let Some((newvar, rest)) = text.split_once('=') else {
            return Err(no("nlogitgen needs newvar = altvar(groups)"));
        };
        let newvar = newvar.trim();
        let rest = rest.trim();
        let Some((alt, groups)) = rest.split_once('(') else {
            return Err(no("nlogitgen needs newvar = altvar(groups)"));
        };
        let alt = alt.trim();
        let Some(groups) = groups.trim().strip_suffix(')') else {
            return Err(no("nlogitgen: a parenthesis is not closed"));
        };
        if !is_name(newvar) || !is_name(alt) {
            return Err(no("nlogitgen: variable names written out are translated"));
        }
        let mut gs: Vec<String> = Vec::new();
        for g in groups.split(',') {
            let Some((name, members)) = g.split_once(':') else {
                return Err(no("nlogitgen: each group is name: alt | alt"));
            };
            let name = name.trim();
            let ms: Vec<String> = members.split('|').map(|m| m.trim().to_string()).collect();
            if !is_name(name)
                || ms
                    .iter()
                    .any(|m| m.is_empty() || m.contains(char::is_whitespace))
            {
                return Err(no(
                    "nlogitgen: group and alternative names written out are translated",
                ));
            }
            gs.push(format!(
                "list(name = {}, members = {})",
                r_text(name),
                r_vec(&ms)
            ));
        }
        let base = self.data()?;
        for h in ["stata_manova", "stata_choice", "stata_nlogit"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "{base} <- stata_nlogitgen({base}, {}, {}, list({}))",
            r_text(newvar),
            r_text(alt),
            gs.join(", ")
        ));
        self.kinds.insert(newvar.to_string(), Kind::Number);
        self.storage.insert(newvar.to_string(), Storage::Byte);
        Ok(())
    }

    /// `nlogittree altvar nestvar, choice(var)`.
    pub(in crate::data_r) fn nlogittree(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "nlogittree")?;
        if let Some(o) = opts.other(&[("choice", 1)]) {
            return Err(no(format!(
                "nlogittree with the option {o} is not translated yet"
            )));
        }
        let Some(vars) = plain(&c.parts.main) else {
            return Err(no("nlogittree: variable names written out are translated"));
        };
        let [alt, nest] = vars.as_slice() else {
            return Err(no(
                "nlogittree translates one grouping: nlogittree altvar nestvar",
            ));
        };
        let Some(choice) = opts.arg("choice", 1).map(|s| s.trim().to_string()) else {
            return Err(no("nlogittree without choice() is not translated yet"));
        };
        let base = self.data()?;
        for h in ["stata_manova", "stata_choice", "stata_nlogit"] {
            self.helpers.insert(h);
        }
        if !self.quiet(c) {
            out.line(format!(
                "stata_nlogit_print_tree({base}, {}, {}, {}, {})",
                r_text(&choice),
                r_text(alt),
                r_text(nest),
                r_text(alt)
            ));
        }
        Ok(())
    }

    /// `nlogit depvar [indepvars] || nestvar: [nestvars], base(group) || altvar:, case(var) [noconstant technique(nr) nolog]`.
    pub(in crate::data_r) fn nlogit(
        &mut self,
        c: &Command,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "nlogit")?;
        if c.parts.weight.is_some() || c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("nlogit with weights, if or in is not translated yet"));
        }
        let text = super::super::returned::after_name(c);
        let segs: Vec<&str> = text.split("||").collect();
        let [first, mid, last] = segs.as_slice() else {
            return Err(no(
                "nlogit is translated for a tree of two levels: nlogit depvar vars || nestvar: [vars], base() || altvar:, case()",
            ));
        };
        let Some(dv) = plain(first) else {
            return Err(no("nlogit: variable names written out are translated"));
        };
        let Some((depvar, indep)) = dv.split_first() else {
            return Err(no("nlogit needs a dependent variable"));
        };
        // the nesting level: nestvar: vars, base(group)
        let Some((nestvar, nrest)) = mid.split_once(':') else {
            return Err(no(
                "nlogit: the nesting level is nestvar: [vars], base(group)",
            ));
        };
        let nestvar = nestvar.trim();
        let (nvars, nopts) = match nrest.split_once(',') {
            Some((a, b)) => (a, b),
            None => (nrest, ""),
        };
        let Some(nestx) = plain(nvars) else {
            return Err(no("nlogit: variable names written out are translated"));
        };
        let mut base: Option<String> = None;
        for w in option_words(nopts) {
            if let Some(b) = arg_of(&w, "base") {
                base = Some(b.trim().to_string());
            } else {
                return Err(no(format!(
                    "nlogit with the option {w} at the nesting level is not translated yet"
                )));
            }
        }
        // the alternative level: altvar: , options
        let Some((altvar, arest)) = last.split_once(':') else {
            return Err(no("nlogit: the last level is altvar:, case(var)"));
        };
        let altvar = altvar.trim();
        let (avars, aopts) = match arest.split_once(',') {
            Some((a, b)) => (a, b),
            None => (arest, ""),
        };
        if !avars.trim().is_empty() {
            return Err(no(
                "nlogit with variables at the alternative level is not translated yet",
            ));
        }
        let (mut case, mut noconst, mut technique, mut nolog) =
            (None, false, "bfgs".to_string(), false);
        for w in option_words(aopts) {
            if let Some(v) = arg_of(&w, "case") {
                case = Some(v.trim().to_string());
            } else if w.len() >= 5 && "noconstant".starts_with(w.as_str()) {
                noconst = true;
            } else if let Some(t) = arg_of(&w, "technique") {
                let t = t.trim();
                if t == "nr" || t == "bfgs" {
                    technique = t.to_string();
                } else {
                    return Err(no(format!("nlogit technique({t}) is not translated yet")));
                }
            } else if w == "nolog" {
                nolog = true;
            } else {
                return Err(no(format!(
                    "nlogit with the option {w} is not translated yet"
                )));
            }
        }
        let _ = nolog;
        let Some(case) = case else {
            out.line("stop(\"option case() required (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        };
        if !is_name(nestvar) || !is_name(altvar) || !is_name(&case) {
            return Err(no("nlogit: variable names written out are translated"));
        }
        if !nestx.is_empty() && !noconst {
            return Err(no(
                "nlogit with variables at the nesting level and alternative constants is not translated yet",
            ));
        }
        let base_arg = base
            .as_ref()
            .map(|b| format!(", base = {}", r_text(b)))
            .unwrap_or_default();
        let data = self.data()?;
        for h in ["stata_manova", "stata_choice", "stata_nlogit", "stata_fmt"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_nlogit({data}, {}, {}, {}, {}, {}, {}{base_arg}, noconst = {}, technique = {}, cmdline = {}{})",
            r_text(depvar),
            r_vec(indep),
            r_text(nestvar),
            r_vec(&nestx),
            r_text(altvar),
            r_text(&case),
            if noconst { "TRUE" } else { "FALSE" },
            r_text(&technique),
            r_text(&super::super::returned::cmdline(c)),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        if technique == "bfgs" {
            out.caveat(
                "nlogit's default technique stops where Stata's BFGS stops; RCraft finds the optimum, which Stata's technique(nr) reaches \
                 (the estimates agree to about 1e-4)",
            );
        }
        let mut t: Vec<String> = [
            "cmdline",
            "cmd",
            "marginsnotok",
            "predict",
            "estat_cmd",
            "marktype",
            "ind2vars1",
            "alteqs1",
            "altvar1",
            "alteqs",
            "altvar",
            "key_N_ic",
            "case",
            "depvar",
            "indvars",
            "chi2type",
            "deriv_useminbound",
            "opt",
            "vce",
            "title",
            "user",
            "ml_method",
            "technique",
            "which",
            "properties",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        t.extend((1..=20).flat_map(|i| [format!("alt{i}"), format!("alt1_{i}")]));
        let refs: Vec<&str> = t.iter().map(String::as_str).collect();
        keep_e(
            self,
            "nlogit",
            &refs,
            &[],
            "nlogit's results are not a model RCraft's test, lincom or margins read; predict is translated [multi]",
            out,
        );
        Ok(())
    }

    /// `predict stub*` (or two names) after `nlogit`: the probabilities of the group and of the alternative.
    pub(in crate::data_r) fn nlogit_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("pr", 2)]) {
            return Err(no(format!(
                "predict after nlogit with the option {o} is not translated yet"
            )));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let names: Vec<String> =
            match words.as_slice() {
                [w] if w.ends_with('*') && is_name(w.trim_end_matches('*')) => {
                    let s = w.trim_end_matches('*');
                    vec![format!("{s}1"), format!("{s}2")]
                }
                [a, b] if is_name(a) && is_name(b) => vec![(*a).to_string(), (*b).to_string()],
                _ => return Err(no(
                    "predict after nlogit: a stub (p*) or two new variable names are translated",
                )),
            };
        let base = self.data()?;
        for n in &names {
            out.line(format!(
                "if ({} %in% names({base})) stop(\"variable {n} already defined (Stata: r(110))\", call. = FALSE)",
                r_text(n)
            ));
        }
        for h in [
            "stata_manova",
            "stata_choice",
            "stata_nlogit",
            "stata_float",
        ] {
            self.helpers.insert(h);
        }
        out.package("dplyr");
        if !opts.has("pr", 2) && !self.quiet(c) {
            out.line("cat(\"(option pr assumed)\\n\")");
        }
        out.line(format!("stata_pr <- stata_nlogit_pr({base}, stata_e)"));
        let cond = self.condition(c, out)?;
        for (i, n) in names.iter().enumerate() {
            let v = stored(
                &format!("stata_pr[, {}L]", i + 1),
                Storage::Float,
                &mut self.helpers,
                out,
            );
            let v = match &cond {
                Some(cd) => format!("dplyr::if_else({cd}, {v}, NA_real_)"),
                None => v,
            };
            out.line(format!(
                "{base} <- dplyr::mutate({base}, {} = {v})",
                r_name(n)
            ));
            self.kinds.insert(n.clone(), Kind::Number);
            self.storage.insert(n.clone(), Storage::Float);
        }
        out.line("rm(stata_pr)");
        Ok(())
    }
}
