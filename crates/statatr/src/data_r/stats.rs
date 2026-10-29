//! P2B.4: Stata's statistics commands in R: `summarize`, `tabulate`,
//! `correlate`, `ttest`, `regress`, `logit`, `probit`, `poisson`, `predict`,
//! `test`, `lincom` and `estat ic`.
//!
//! The R is small functions of our own (`stats.R`, defined once at the top of
//! the script) that follow Stata's rules as StatCraft implements them (each
//! checked there against Stata): the estimation sample drops a row with any
//! variable missing; factor variables (`i.g`, `ib2.g`, `#`, `##`) get Stata's
//! base levels and coefficient names; a collinear regressor is dropped the
//! way Stata drops it; `vce(robust)` and `vce(cluster)` use Stata's
//! small-sample factors; the ML commands use the observed information.
//! Each function stores what the Stata command returns, under Stata's names,
//! in `stata_r` (r-class) or `stata_e` (e-class), so `r(mean)` is
//! `stata_r$mean` and `_b[x]` is `stata_e$b[["x"]]`.

use super::*;

/// The R functions the statistics commands call.
pub(super) const MODEL_HELPERS: &str = include_str!("stats.R");

/// A model's terms as Stata names them, with `##` expanded: `x`, `i.g`,
/// `ib2.g`, `i.g#c.x`.
pub(super) fn terms(words: &[&str]) -> Result<Vec<String>, Untranslated> {
    let mut out: Vec<String> = Vec::new();
    for w in words {
        let full = w.contains("##");
        let parts: Vec<&str> = if full {
            w.split("##").collect()
        } else {
            w.split('#').collect()
        };
        let inter = parts.len() > 1;
        let comps = parts
            .iter()
            .map(|p| component(p, inter))
            .collect::<Result<Vec<_>, _>>()?;
        let mut add = |t: Vec<String>| {
            let t = if t.len() == 1 {
                // a continuous variable alone is named without `c.`
                t.join("").trim_start_matches("c.").to_string()
            } else {
                t.join("#")
            };
            if !out.contains(&t) {
                out.push(t);
            }
        };
        if full {
            // every subset, the smaller first, in the order typed
            let k = comps.len();
            if k > 8 {
                return Err(no(format!("{w}: too many variables in one interaction")));
            }
            for size in 1..=k {
                for mask in 1u32..(1 << k) {
                    if mask.count_ones() as usize == size {
                        add((0..k)
                            .filter(|i| mask & (1 << i) != 0)
                            .filter_map(|i| comps.get(i).cloned())
                            .collect());
                    }
                }
            }
        } else {
            add(comps);
        }
    }
    Ok(out)
}

/// One variable of a term, as `c.x` or `i.g` (`ib2.g`, `ibn.g`, ...). A
/// bare name is continuous alone and a factor inside an interaction.
fn component(p: &str, inter: bool) -> Result<String, Untranslated> {
    let bad = || no(format!("{p}: this variable form is not translated yet"));
    let Some((pre, var)) = p.split_once('.') else {
        if !is_name(p) {
            return Err(no(format!(
                "{p}: only plain variable names are translated in a model, not patterns or ranges"
            )));
        }
        return Ok(if inter {
            format!("i.{p}")
        } else {
            format!("c.{p}")
        });
    };
    if !is_name(var) {
        return Err(bad());
    }
    let pre = match pre {
        "c" => "c".to_string(),
        "i" => "i".to_string(),
        "ibn" => "ibn".to_string(),
        "ib(first)" => "ibfirst".to_string(),
        "ib(last)" => "iblast".to_string(),
        "ib(freq)" => "ibfreq".to_string(),
        p if p
            .strip_prefix("ib")
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) =>
        {
            p.to_string()
        }
        "L" | "F" | "D" | "S" => {
            return Err(no(format!(
                "{p}: time-series operators are translated with xtset and tsset, in a later step"
            )))
        }
        _ => return Err(bad()),
    };
    Ok(format!("{pre}.{var}"))
}

/// A linear combination of coefficients: weights by name and a constant
/// (`x + 2*z - _b[w]/2 + 3`).
pub(super) fn linear(text: &str) -> Result<(Vec<(String, f64)>, f64), Untranslated> {
    let bad = |why: &str| no(format!("`{}`: {why}", text.trim()));
    let mut toks: Vec<String> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c.is_whitespace() {
            i += 1;
        } else if "+-*/".contains(c) {
            toks.push(c.to_string());
            i += 1;
        } else if c == '(' || c == ')' {
            return Err(bad("parentheses are not translated yet"));
        } else {
            let start = i;
            let mut depth = 0;
            while let Some(&c) = chars.get(i) {
                if c == '[' {
                    depth += 1;
                } else if c == ']' {
                    depth -= 1;
                } else if depth == 0 && (c.is_whitespace() || "+-*/()".contains(c)) {
                    // `1e-3`: the sign belongs to the number
                    let exp = (c == '-' || c == '+')
                        && i > start
                        && chars.get(i - 1).is_some_and(|p| *p == 'e' || *p == 'E')
                        && chars
                            .get(start..i - 1)
                            .is_some_and(|s| s.iter().all(|c| c.is_ascii_digit() || *c == '.'));
                    if !exp {
                        break;
                    }
                }
                i += 1;
            }
            toks.push(chars.get(start..i).unwrap_or_default().iter().collect());
        }
    }
    let mut weights: Vec<(String, f64)> = Vec::new();
    let mut constant = 0.0;
    let mut sign = 1.0;
    let mut k = 0;
    let mut expect_term = true;
    while k < toks.len() {
        let t = toks.get(k).map(String::as_str).unwrap_or("");
        if expect_term {
            if t == "-" {
                sign = -sign;
                k += 1;
                continue;
            }
            if t == "+" {
                k += 1;
                continue;
            }
            // a term: factors joined by * and /
            let mut coef = sign;
            let mut name: Option<String> = None;
            let mut op = "*";
            loop {
                let f = toks.get(k).map(String::as_str).unwrap_or("");
                if f.is_empty() || "+-*/".contains(f) {
                    return Err(bad("an operator is missing a value"));
                }
                match f.parse::<f64>() {
                    Ok(v) if op == "*" => coef *= v,
                    Ok(v) if v != 0.0 => coef /= v,
                    Ok(_) => return Err(bad("division by zero")),
                    Err(_) => {
                        if op == "/" || name.is_some() {
                            return Err(bad("not a linear combination of coefficients"));
                        }
                        name = Some(coef_name(f).ok_or_else(|| bad("not a coefficient name"))?);
                    }
                }
                k += 1;
                match toks.get(k).map(String::as_str) {
                    Some(o @ ("*" | "/")) => {
                        op = if o == "*" { "*" } else { "/" };
                        k += 1;
                    }
                    _ => break,
                }
            }
            match name {
                Some(n) => match weights.iter_mut().find(|(m, _)| *m == n) {
                    Some((_, w)) => *w += coef,
                    None => weights.push((n, coef)),
                },
                None => constant += coef,
            }
            sign = 1.0;
            expect_term = false;
        } else {
            match t {
                "+" => sign = 1.0,
                "-" => sign = -1.0,
                _ => return Err(bad("an operator is missing between two values")),
            }
            k += 1;
            expect_term = true;
        }
    }
    if expect_term {
        return Err(bad("the expression is incomplete"));
    }
    Ok((weights, constant))
}

/// A coefficient as written: `x`, `_b[x]`, `2.g`, `_cons`, `1.g#c.x`.
fn coef_name(t: &str) -> Option<String> {
    let t = t.trim();
    let inner = t
        .strip_prefix("_b[")
        .and_then(|r| r.strip_suffix(']'))
        .unwrap_or(t)
        .trim();
    if inner.starts_with('[') || inner.contains(':') {
        return None;
    }
    let ok = !inner.is_empty()
        && inner
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.#".contains(c))
        && inner.split('#').all(|p| {
            let v = p.rsplit('.').next().unwrap_or("");
            is_name(v)
        });
    if !ok {
        return None;
    }
    // `c.x` alone is the coefficient `x`
    Some(if inner.contains('#') {
        inner.to_string()
    } else {
        inner.trim_start_matches("c.").to_string()
    })
}

fn r_weights(w: &[(String, f64)]) -> String {
    let parts: Vec<String> = w
        .iter()
        .map(|(n, v)| format!("{} = {}", r_text(n), r_number(*v)))
        .collect();
    format!("c({})", parts.join(", "))
}

/// `vce()` and the old `robust` and `cluster()` options: the kind and the
/// cluster variable.
fn vce(opts: &Opts, ml: bool) -> Result<(&'static str, Option<String>), Untranslated> {
    let default = if ml { "oim" } else { "ols" };
    if let Some(v) = opts.arg("vce", 3) {
        let (kind, rest) = first_word(&v);
        return match kind {
            "robust" | "r" | "rob" | "robu" | "robus" => Ok(("robust", None)),
            "cluster" | "cl" | "clu" | "clus" | "clust" | "cluste" => {
                let var = rest.trim();
                if !is_name(var) {
                    return Err(no("vce(cluster) needs one variable"));
                }
                Ok(("cluster", Some(var.to_string())))
            }
            "ols" if !ml => Ok(("ols", None)),
            "oim" if ml => Ok(("oim", None)),
            other => Err(no(format!("vce({other}) is not translated yet"))),
        };
    }
    if let Some(var) = opts.arg("cluster", 2) {
        let var = var.trim();
        if !is_name(var) {
            return Err(no("cluster() needs one variable"));
        }
        return Ok(("cluster", Some(var.to_string())));
    }
    if opts.has("robust", 1) {
        return Ok(("robust", None));
    }
    Ok((default, None))
}

impl Session {
    /// `, cond = ...` for a helper when the command has `if` or `in`.
    fn cond_arg(&self, c: &Command, out: &mut Out) -> Result<String, Untranslated> {
        let d = self.data()?;
        Ok(match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!(", cond = dplyr::transmute({d}, .t = {cond})$.t")
            }
            None => String::new(),
        })
    }

    fn stats_start(&mut self, c: &Command, out: &mut Out) -> Result<String, Untranslated> {
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no(format!("by with {} is not translated yet", c.name)));
        }
        self.helpers.insert("stata_models");
        let _ = out;
        self.data()
    }

    fn show(c: &Command) -> &'static str {
        if Self::quiet(c) {
            ", show = FALSE"
        } else {
            ""
        }
    }

    /// The variables of a `summarize`, `tabulate` or `correlate`.
    fn varlist(&self, c: &Command, d: &str) -> Result<String, Untranslated> {
        if c.parts.main.trim().is_empty() {
            return Ok(format!("names({d})"));
        }
        let n = names(&c.parts.main)?;
        Ok(format!(
            "c({})",
            n.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", ")
        ))
    }

    pub(super) fn summarize(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("detail", 1), ("meanonly", 4), ("separator", 3)]) {
            return Err(no(format!(
                "summarize with the option {o} is not translated yet"
            )));
        }
        let d = self.stats_start(c, out)?;
        let vars = self.varlist(c, &d)?;
        let cond = self.cond_arg(c, out)?;
        let mut args = String::new();
        if opts.has("detail", 1) {
            args.push_str(", detail = TRUE");
        }
        if opts.has("meanonly", 4) {
            args.push_str(", meanonly = TRUE");
        }
        out.line(format!(
            "stata_r <- stata_summarize({d}, {vars}{cond}{args}{})",
            Self::show(c)
        ));
        Ok(())
    }

    pub(super) fn tabulate(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let known = [
            ("missing", 1),
            ("chi2", 2),
            ("lrchi2", 2),
            ("exact", 5),
            ("V", 1),
            ("row", 3),
            ("column", 3),
            ("cell", 4),
            ("nofreq", 6),
            ("nolabel", 5),
        ];
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "tabulate with the option {o} is not translated yet"
            )));
        }
        let n = names(&c.parts.main)?;
        if n.len() > 2 {
            return Err(no("tabulate takes one or two variables"));
        }
        let d = self.stats_start(c, out)?;
        let vars = self.varlist(c, &d)?;
        let cond = self.cond_arg(c, out)?;
        let mut args = String::new();
        for (o, m, a) in [
            ("missing", 1, "missing"),
            ("chi2", 2, "chi2"),
            ("lrchi2", 2, "lrchi2"),
            ("exact", 5, "exact"),
            ("V", 1, "V"),
        ] {
            if opts.has(o, m) {
                if n.len() == 1 && o != "missing" {
                    return Err(no(format!("{o} needs a two-way table")));
                }
                args.push_str(&format!(", {a} = TRUE"));
            }
        }
        if ["row", "column", "cell", "nofreq"]
            .iter()
            .zip([3, 3, 4, 6])
            .any(|(o, m)| opts.has(o, m))
        {
            out.caveat(
                "the R table shows counts; row, column and cell percentages are not printed",
            );
        }
        out.line(format!(
            "stata_r <- stata_tabulate({d}, {vars}{cond}{args}{})",
            Self::show(c)
        ));
        Ok(())
    }

    pub(super) fn correlate(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("covariance", 1)]) {
            return Err(no(format!(
                "correlate with the option {o} is not translated yet"
            )));
        }
        let d = self.stats_start(c, out)?;
        let vars = self.varlist(c, &d)?;
        let cond = self.cond_arg(c, out)?;
        let cov = if opts.has("covariance", 1) {
            ", covariance = TRUE"
        } else {
            ""
        };
        out.line(format!(
            "stata_r <- stata_correlate({d}, {vars}{cond}{cov}{})",
            Self::show(c)
        ));
        Ok(())
    }

    pub(super) fn ttest(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("by", 2), ("unequal", 3), ("welch", 1), ("unpaired", 3)]) {
            return Err(no(format!(
                "ttest with the option {o} is not translated yet"
            )));
        }
        let d = self.stats_start(c, out)?;
        let cond = self.cond_arg(c, out)?;
        let main = c.parts.main.trim();
        let mut args = String::new();
        let x = match main.split_once("==").or_else(|| main.split_once('=')) {
            Some((x, rhs)) => {
                let rhs = rhs.trim();
                if let Some(v) = number_literal(rhs).filter(|v| v != "NA_real_") {
                    args.push_str(&format!(", mu = {v}"));
                } else if is_name(rhs) {
                    args.push_str(&format!(", y = {}", r_text(rhs)));
                } else {
                    return Err(no(format!("ttest: {rhs} is not a number or a variable")));
                }
                x.trim().to_string()
            }
            None => {
                let by = opts.arg("by", 2).ok_or_else(|| {
                    no("ttest needs `== value`, `== variable` or the by() option")
                })?;
                if !is_name(by.trim()) {
                    return Err(no("ttest: by() needs one variable"));
                }
                args.push_str(&format!(", by = {}", r_text(by.trim())));
                main.to_string()
            }
        };
        if !is_name(&x) {
            return Err(no(format!("ttest: {x} is not a variable name")));
        }
        for (o, m, a) in [
            ("unequal", 3, "unequal"),
            ("welch", 1, "welch"),
            ("unpaired", 3, "unpaired"),
        ] {
            if opts.has(o, m) {
                args.push_str(&format!(", {a} = TRUE"));
            }
        }
        out.line(format!(
            "stata_r <- stata_ttest({d}, {}{cond}{args}{})",
            r_text(&x),
            Self::show(c)
        ));
        Ok(())
    }

    /// The dependent variable and the terms of an estimation command.
    fn model_spec(c: &Command) -> Result<(String, String), Untranslated> {
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let Some((y, rest)) = words.split_first() else {
            return Err(no(format!("{} needs a dependent variable", c.name)));
        };
        if !is_name(y) {
            return Err(no(format!(
                "{y}: the dependent variable must be a plain variable name"
            )));
        }
        let t = terms(rest)?;
        let t: Vec<String> = t.iter().map(|t| r_text(t)).collect();
        Ok((
            r_text(y),
            if t.is_empty() {
                "character()".to_string()
            } else {
                format!("c({})", t.join(", "))
            },
        ))
    }

    pub(super) fn regress(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("vce", 3), ("robust", 1), ("cluster", 2), ("noconstant", 6)])
        {
            return Err(no(format!(
                "regress with the option {o} is not translated yet"
            )));
        }
        let d = self.stats_start(c, out)?;
        let (y, t) = Self::model_spec(c)?;
        let cond = self.cond_arg(c, out)?;
        let (kind, cluster) = vce(opts, false)?;
        let mut args = String::new();
        if kind != "ols" {
            args.push_str(&format!(", vce = \"{kind}\""));
        }
        if let Some(v) = cluster {
            args.push_str(&format!(", cluster = {}", r_text(&v)));
        }
        if opts.has("noconstant", 6) {
            args.push_str(", cons = FALSE");
        }
        out.line(format!(
            "stata_e <- stata_regress({d}, {y}, {t}{cond}{args}{})",
            Self::show(c)
        ));
        self.estimates = Some("regress");
        Ok(())
    }

    pub(super) fn ml_model(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let model: &'static str = match c.name.as_str() {
            "logit" => "logit",
            "probit" => "probit",
            _ => "poisson",
        };
        let mut known = vec![("vce", 3), ("robust", 1), ("cluster", 2)];
        match model {
            "logit" => known.push(("or", 2)),
            "poisson" => known.extend([("irr", 3), ("offset", 3), ("exposure", 1)]),
            _ => {}
        }
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "{model} with the option {o} is not translated yet"
            )));
        }
        let d = self.stats_start(c, out)?;
        let (y, t) = Self::model_spec(c)?;
        let cond = self.cond_arg(c, out)?;
        let (kind, cluster) = vce(opts, true)?;
        let mut args = String::new();
        if kind != "oim" {
            args.push_str(&format!(", vce = \"{kind}\""));
        }
        if let Some(v) = cluster {
            args.push_str(&format!(", cluster = {}", r_text(&v)));
        }
        for (o, m, a) in [("offset", 3, "offset"), ("exposure", 1, "exposure")] {
            if let Some(v) = opts.arg(o, m) {
                if !is_name(v.trim()) {
                    return Err(no(format!("{o}() needs one variable")));
                }
                args.push_str(&format!(", {a} = {}", r_text(v.trim())));
            }
        }
        if model == "logit" && opts.has("or", 2) {
            args.push_str(", eform = \"Odds ratio\"");
        }
        if model == "poisson" && opts.has("irr", 3) {
            args.push_str(", eform = \"IRR\"");
        }
        out.line(format!(
            "stata_e <- stata_ml(\"{model}\", {d}, {y}, {t}{cond}{args}{})",
            Self::show(c)
        ));
        self.estimates = Some(model);
        Ok(())
    }

    pub(super) fn predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let est = self
            .estimates
            .ok_or_else(|| no("predict needs an estimation command before it"))?;
        let allowed: &[(&str, usize, &str)] = match est {
            "regress" => &[
                ("xb", 2, "xb"),
                ("stdp", 4, "stdp"),
                ("residuals", 1, "residuals"),
            ],
            "poisson" => &[("n", 1, "n"), ("xb", 2, "xb"), ("stdp", 4, "stdp")],
            _ => &[("pr", 2, "pr"), ("xb", 2, "xb"), ("stdp", 4, "stdp")],
        };
        let known: Vec<(&str, usize)> = allowed.iter().map(|(o, m, _)| (*o, *m)).collect();
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "predict with the option {o} is not translated after {est} yet"
            )));
        }
        let picked: Vec<&str> = allowed
            .iter()
            .filter(|(o, m, _)| opts.has(o, *m))
            .map(|(_, _, a)| *a)
            .collect();
        if picked.len() > 1 {
            return Err(no("predict: only one statistic may be asked for"));
        }
        let what = picked
            .first()
            .copied()
            .unwrap_or(allowed.first().map_or("xb", |a| a.2));
        let d = self.stats_start(c, out)?;
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (storage, var) = match words.as_slice() {
            [v] => (Storage::Float, *v),
            [t, v] => (
                storage(t).ok_or_else(|| no(format!("predict: {t} is not a storage type")))?,
                *v,
            ),
            _ => return Err(no("predict needs one new variable name")),
        };
        if !is_name(var) || matches!(storage, Storage::Str) {
            return Err(no(format!("predict: {var} is not a new numeric variable")));
        }
        let cond = self.cond_arg(c, out)?;
        let value = stored(
            &format!("stata_predict({d}, stata_e, \"{what}\"{cond})"),
            storage,
            &mut self.helpers,
            out,
        );
        out.package("dplyr");
        out.line(format!(
            "if ({} %in% names({d})) stop(\"variable {var} already defined\")",
            r_text(var)
        ));
        out.line(format!("{d}[[{}]] <- {value}", r_text(var)));
        self.kinds.insert(var.to_string(), Kind::Number);
        Ok(())
    }

    pub(super) fn test(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "test with the option {o} is not translated yet"
            )));
        }
        if self.estimates.is_none() {
            return Err(no("test needs an estimation command before it"));
        }
        self.stats_start(c, out)?;
        let main = c.parts.main.trim();
        let mut eqs: Vec<String> = Vec::new();
        if main.starts_with('(') {
            let mut depth = 0;
            let mut cur = String::new();
            for ch in main.chars() {
                match ch {
                    '(' => {
                        if depth > 0 {
                            cur.push(ch);
                        }
                        depth += 1;
                    }
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            eqs.push(std::mem::take(&mut cur));
                        } else {
                            cur.push(ch);
                        }
                    }
                    c if depth > 0 => cur.push(c),
                    c if c.is_whitespace() => {}
                    _ => return Err(no("test: text outside the parentheses")),
                }
            }
        } else if main.contains('=') {
            eqs.push(main.to_string());
        } else {
            for w in main.split_whitespace() {
                let n = coef_name(w)
                    .ok_or_else(|| no(format!("test: {w} is not a coefficient name")))?;
                eqs.push(n);
            }
        }
        if eqs.is_empty() {
            return Err(no("test needs coefficients or an equation"));
        }
        let mut rs = Vec::new();
        let mut vals = Vec::new();
        for e in &eqs {
            let (lhs, rhs) = match e.split_once('=') {
                Some((l, r)) => (l, r.trim_start_matches('=')),
                None => (e.as_str(), "0"),
            };
            let (lw, lc) = linear(lhs)?;
            let (rw, rc) = linear(rhs)?;
            let mut w = lw;
            for (n, v) in rw {
                match w.iter_mut().find(|(m, _)| *m == n) {
                    Some((_, x)) => *x -= v,
                    None => w.push((n, -v)),
                }
            }
            if w.is_empty() {
                return Err(no(format!("test: {e} has no coefficient")));
            }
            rs.push(r_weights(&w));
            vals.push(r_number(rc - lc));
        }
        out.line(format!(
            "stata_r <- stata_test(stata_e, list({}), c({}){})",
            rs.join(", "),
            vals.join(", "),
            Self::show(c)
        ));
        Ok(())
    }

    pub(super) fn lincom(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let est = self
            .estimates
            .ok_or_else(|| no("lincom needs an estimation command before it"))?;
        let mut known = vec![("or", 2), ("irr", 3), ("eform", 2)];
        known.retain(|(o, _)| match est {
            "logit" => *o != "irr",
            "poisson" => *o != "or",
            _ => *o == "eform",
        });
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "lincom with the option {o} is not translated yet"
            )));
        }
        self.stats_start(c, out)?;
        let (w, k) = linear(&c.parts.main)?;
        if w.is_empty() {
            return Err(no("lincom needs at least one coefficient"));
        }
        let eform = if opts.has("or", 2) {
            ", eform = \"Odds ratio\""
        } else if opts.has("irr", 3) {
            ", eform = \"IRR\""
        } else if opts.has("eform", 2) {
            ", eform = \"exp(b)\""
        } else {
            ""
        };
        let constant = if k == 0.0 {
            String::new()
        } else {
            format!(", const = {}", r_number(k))
        };
        out.line(format!(
            "stata_r <- stata_lincom(stata_e, {}{constant}{eform}{})",
            r_weights(&w),
            Self::show(c)
        ));
        Ok(())
    }

    pub(super) fn estat(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        if self.estimates.is_none() {
            return Err(no("estat needs an estimation command before it"));
        }
        let (sub, _) = first_word(&c.parts.main);
        match sub {
            "ic" => {
                self.stats_start(c, out)?;
                out.line(format!(
                    "stata_r <- stata_estat_ic(stata_e{})",
                    Self::show(c)
                ));
                Ok(())
            }
            other => Err(no(format!("estat {other} is not translated yet"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_follow_stata() {
        assert_eq!(terms(&["x", "c.z"]).unwrap(), vec!["x", "z"]);
        assert_eq!(terms(&["i.g##c.x"]).unwrap(), vec!["i.g", "x", "i.g#c.x"]);
        assert_eq!(terms(&["g#h"]).unwrap(), vec!["i.g#i.h"]);
        assert_eq!(
            terms(&["ib(last).g", "ib2.h"]).unwrap(),
            vec!["iblast.g", "ib2.h"]
        );
        assert_eq!(
            terms(&["a##b##c"]).unwrap(),
            vec![
                "i.a",
                "i.b",
                "i.c",
                "i.a#i.b",
                "i.a#i.c",
                "i.b#i.c",
                "i.a#i.b#i.c"
            ]
        );
        assert!(terms(&["x*"]).is_err());
        assert!(terms(&["L.x"]).is_err());
    }

    #[test]
    fn linear_combinations() {
        let (w, k) = linear("x + 2*z - _b[w]/2 + 3").unwrap();
        assert_eq!(
            w,
            vec![("x".into(), 1.0), ("z".into(), 2.0), ("w".into(), -0.5)]
        );
        assert_eq!(k, 3.0);
        let (w, k) = linear("-2.g + 1e-3").unwrap();
        assert_eq!(w, vec![("2.g".into(), -1.0)]);
        assert_eq!(k, 1e-3);
        let (w, _) = linear("x - x + c.z").unwrap();
        assert_eq!(w, vec![("x".into(), 0.0), ("z".into(), 1.0)]);
        assert!(linear("x * z").is_err());
        assert!(linear("(x + z)").is_err());
        assert!(linear("x +").is_err());
    }
}
