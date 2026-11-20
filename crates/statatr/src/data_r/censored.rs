//! Batch J of the StatCraft port: the censored and selection models, `tobit`, `intreg`, `truncreg`,
//! `cloglog`, `hetprobit` and `heckman`, as Stata 19 fits and reports them (`plan/stata/censored.md`).
//!
//! The fit is Newton-Raphson with ml's step rules written in R (`censored/cx.R`), from the starting
//! values and likelihoods StatCraft decoded (our own work); R runs as a separate process, never linked.

use super::model::{level, vce, Estimates, Fit, Spec};
use super::weights::{Weighting, W};
use super::*;

pub(super) const CX_HELPER: &str = include_str!("censored/cx.R");
pub(super) const TOBIT_HELPER: &str = include_str!("censored/tobit.R");
pub(super) const TRUNC_HELPER: &str = include_str!("censored/trunc.R");
pub(super) const BIN_HELPER: &str = include_str!("censored/bin.R");

/// What every command of the batch reads from its line.
struct Line {
    spec: Spec,
    /// intreg: the upper bound, the first word after the lower one
    hi: Option<String>,
    w: Weighting,
    level: f64,
    robust: bool,
    cluster: Option<String>,
    src: String,
    show: bool,
    /// the line stopped as Stata stops it (a weight kind the command does not take): nothing more to do
    stopped: bool,
}

impl Line {
    /// `, wvar = "w", wtype = "f"`, or nothing.
    fn wargs(&self) -> String {
        match &self.w {
            Some((k, v)) => format!(", wvar = {}, wtype = {}", r_text(v), r_text(k.code())),
            None => String::new(),
        }
    }

    fn cluster_arg(&self) -> String {
        match &self.cluster {
            Some(v) => format!(", cluster = {}", r_text(v)),
            None => ", cluster = NULL".to_string(),
        }
    }
}

fn names(terms: &[String]) -> String {
    format!(
        "c({})",
        terms
            .iter()
            .map(|t| r_text(t))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// A limit option: `ll`, `ll(#)`, or none. `NULL`, `TRUE` (the smallest or largest value) or the number.
fn limit(opts: &Opts, name: &str) -> Result<String, Untranslated> {
    if !opts.has(name, 2) {
        return Ok("NULL".into());
    }
    match opts.arg(name, 2).as_deref().map(str::trim) {
        Some("") | None => Ok("TRUE".into()),
        Some(v) => v
            .parse::<f64>()
            .map(r_number)
            .map_err(|_| no(format!("{name}({v}) is not a number"))),
    }
}

impl Session {
    /// Which of the batch's commands holds the active results.
    pub(super) fn censored_active(&self) -> Option<&'static str> {
        let fit = self.estimates.as_ref()?.fit;
        let cmd = match fit {
            Fit::Tobit => "tobit",
            Fit::Intreg => "intreg",
            Fit::Truncreg => "truncreg",
            Fit::Cloglog => "cloglog",
            Fit::Hetprobit => "hetprobit",
            _ => return None,
        };
        matches!(
            &self.e_ret,
            super::returned::Returned::Program { cmd: c, .. } if c == cmd
        )
        .then_some(cmd)
    }

    /// The pieces every command shares: the varlist, weights, level, variance, rows.
    fn cx_line(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        allowed: &[W],
        name: &str,
    ) -> Result<Line, Untranslated> {
        self.no_by(c, name)?;
        let mut spec = Spec::read(c, name)?;
        let mut hi = None;
        if name == "intreg" {
            if spec.terms.len() < 2 {
                return Err(no("intreg needs two outcome variables and at least one regressor"));
            }
            let h = spec.terms.remove(0);
            if !is_name(&h) {
                return Err(no(format!("intreg: {h} is not a variable name")));
            }
            hi = Some(h);
        }
        let factors_ok = matches!(name, "tobit" | "intreg");
        let plain = |t: &str| {
            is_name(t)
                || (factors_ok
                    && t.starts_with("factor(")
                    && t.ends_with(')')
                    && is_name(&t[7..t.len() - 1]))
        };
        if !is_name(&spec.dep) || spec.terms.is_empty() || spec.terms.iter().any(|t| !plain(t)) {
            return Err(no(format!(
                "{name} with interactions, ib. bases, operators or no independent variable is not translated yet"
            )));
        }
        if let Some(wt) = &c.parts.weight {
            use crate::lang::syntax::WeightKind;
            let kind = match wt.kind {
                WeightKind::Frequency => Some(W::F),
                WeightKind::Analytic => Some(W::A),
                WeightKind::Sampling => Some(W::P),
                WeightKind::Importance => Some(W::I),
                WeightKind::Unspecified => None,
            };
            if let Some(k) = kind {
                if !allowed.contains(&k) {
                    out.line(format!(
                        "stop({}, call. = FALSE)",
                        r_text(&format!("{} not allowed (Stata: r(101))", k.word()))
                    ));
                    return Ok(Line {
                        spec,
                        hi,
                        w: None,
                        level: 95.0,
                        robust: false,
                        cluster: None,
                        src: String::new(),
                        show: false,
                        stopped: true,
                    });
                }
            }
        }
        let w = self.weight_of(c, allowed)?;
        let lvl = level(opts)? * 100.0;
        let (robust, cluster) = vce(opts)?;
        let rows = self.rows_for(c, out)?;
        let src = self.weighted_rows(rows, &w);
        let robust = robust || matches!(w, Some((W::P, _)));
        let show = !self.quiet(c);
        Ok(Line {
            spec,
            hi,
            w,
            level: lvl,
            robust,
            cluster,
            src,
            show,
            stopped: false,
        })
    }

    /// What stays in R after a fit, and what the translator remembers of it.
    fn cx_after(
        &mut self,
        c: &Command,
        l: &Line,
        fit: Fit,
        cmd: &str,
        texts: &[&str],
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        out.line("stata_b <- attr(stata_e, \"stata_b\")");
        out.line("stata_vcov <- attr(stata_e, \"stata_V\")");
        out.line(if fit == Fit::Tobit {
            "stata_df <- stata_e$df_r"
        } else {
            "stata_df <- Inf"
        });
        out.line("stata_estimates <- list(rows = attr(stata_e, \"stata_rows\"))");
        out.line(self.keep_rows(&l.src, "stata_estimates$rows"));
        let d = self.data()?;
        out.package("dplyr");
        out.line(super::model::esample_rows_line(
            &d,
            &l.src,
            "stata_estimates$rows",
        ));
        self.esample_new = true;
        self.fit_robust = false;
        self.fit_cluster = false;
        self.estimates = Some(Estimates {
            fit,
            dep: l.spec.dep.clone(),
            factors: l.spec.factors.clone(),
        });
        self.e_new = true;
        self.e_ret = super::returned::Returned::Program {
            cmd: cmd.into(),
            texts: texts.iter().map(|s| s.to_string()).collect(),
        };
        let _ = c;
        Ok(())
    }

    /// `tobit depvar indepvars [if] [in] [weight] [, ll[(#)] ul[(#)] vce() level() nolog]`, or a replay.
    pub(super) fn tobit(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("ll", 2),
            ("ul", 2),
            ("robust", 1),
            ("vce", 3),
            ("cluster", 2),
            ("level", 3),
            ("nolog", 3),
        ]) {
            return Err(no(format!(
                "tobit with the option {o} is not translated yet"
            )));
        }
        if c.parts.main.trim().is_empty() {
            return self.tobit_replay(c, opts, out);
        }
        let l = self.cx_line(
            c,
            opts,
            out,
            &[W::F, W::A, W::P, W::I],
            "tobit",
        )?;
        if matches!(l.w, Some((W::A, _))) {
            return Err(no(
                "tobit with analytic weights is not translated yet (each row has its own variance, not reproduced)",
            ));
        }
        let ll = limit(opts, "ll")?;
        let ul = limit(opts, "ul")?;
        for h in ["stata_cx", "stata_tobit", "stata_fmt", "stata_tables", "stata_returned", "stata_eclass"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_tobit({}, {}, {}, ll = {ll}, ul = {ul}{}, robust = {}{}, level = {}, nolog = {}, cmdline = {}, show = {})",
            l.src,
            r_text(&l.spec.dep),
            names(&l.spec.terms),
            l.wargs(),
            super::stats::r_logical(l.robust),
            l.cluster_arg(),
            r_number(l.level),
            super::stats::r_logical(opts.has("nolog", 3)),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(l.show)
        ));
        self.cx_after(
            c,
            &l,
            Fit::Tobit,
            "tobit",
            &[
                "cmdline",
                "marginsok",
                "llopt",
                "ulopt",
                "predict",
                "cmd",
                "title",
                "chi2type",
                "method",
                "wtype",
                "wexp",
                "depvar",
                "footnote",
                "deriv_useminbound",
                "opt",
                "vcetype",
                "clustvar",
                "vce",
                "user",
                "ml_method",
                "technique",
                "which",
                "properties",
            ],
            out,
        )?;
        if l.w.is_some() {
            out.caveat(super::weights::weight_caveat(&l.w));
        }
        out.caveat(
            "tobit as Stata 19 fits it (its gsem-style log: least squares start, sigma^2 as the parameter, ml's Newton steps)",
        );
        Ok(())
    }

    /// `tobit` and `tobit, level(#)`: the last fit again.
    fn tobit_replay(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.cx_replay(c, opts, out, "tobit")
    }

    /// A bare command or one with `level()` only: the last fit of that command again.
    fn cx_replay(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
        cmd: &str,
    ) -> Result<(), Untranslated> {
        if opts.0.iter().any(|o| !o.is("level", 3)) {
            return Err(no(format!("{cmd}'s replay is translated with level() only")));
        }
        let lvl = level(opts)? * 100.0;
        if self.censored_active() != Some(cmd) {
            return Err(no(format!(
                "{cmd} with no variables repeats the last {cmd}, which is not known here"
            )));
        }
        for h in ["stata_cx", "stata_tobit", "stata_fmt", "stata_tables"] {
            self.helpers.insert(h);
        }
        if !self.quiet(c) {
            out.line(format!("stata_{cmd}_replay(level = {})", r_number(lvl)));
        }
        self.e_kept = true;
        Ok(())
    }

    /// `intreg lo hi indepvars [if] [in] [weight] [, vce() level() nolog]`, or a replay.
    pub(super) fn intreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("robust", 1),
            ("vce", 3),
            ("cluster", 2),
            ("level", 3),
            ("nolog", 3),
        ]) {
            return Err(no(format!(
                "intreg with the option {o} is not translated yet"
            )));
        }
        if c.parts.main.trim().is_empty() {
            return self.cx_replay(c, opts, out, "intreg");
        }
        let l = self.cx_line(c, opts, out, &[W::F, W::A, W::P, W::I], "intreg")?;
        if matches!(l.w, Some((W::A, _))) {
            return Err(no(
                "intreg with analytic weights is not translated yet (each row has its own variance, not reproduced)",
            ));
        }
        let hi = l.hi.clone().unwrap_or_default();
        for h in ["stata_cx", "stata_tobit", "stata_fmt", "stata_tables", "stata_returned", "stata_eclass"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_intreg({}, {}, {}, {}{}, robust = {}{}, level = {}, nolog = {}, cmdline = {}, show = {})",
            l.src,
            r_text(&l.spec.dep),
            r_text(&hi),
            names(&l.spec.terms),
            l.wargs(),
            super::stats::r_logical(l.robust),
            l.cluster_arg(),
            r_number(l.level),
            super::stats::r_logical(opts.has("nolog", 3)),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(l.show)
        ));
        self.cx_after(
            c,
            &l,
            Fit::Intreg,
            "intreg",
            &[
                "cmdline",
                "cmd",
                "ml_score",
                "depvar",
                "title",
                "marginsok",
                "predict",
                "wexp",
                "wtype",
                "chi2type",
                "deriv_useminbound",
                "opt",
                "vcetype",
                "clustvar",
                "vce",
                "user",
                "ml_method",
                "technique",
                "which",
                "properties",
            ],
            out,
        )?;
        if l.w.is_some() {
            out.caveat(super::weights::weight_caveat(&l.w));
        }
        out.caveat(
            "intreg as Stata 19 fits it (Olsen's parameters, ml's Newton steps from least squares on the midpoints)",
        );
        Ok(())
    }

    /// `truncreg depvar indepvars [if] [in] [weight] [, ll(#) ul(#) vce() level() nolog]`, or a replay.
    pub(super) fn truncreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("ll", 2),
            ("ul", 2),
            ("robust", 1),
            ("vce", 3),
            ("cluster", 2),
            ("level", 3),
            ("nolog", 3),
        ]) {
            return Err(no(format!(
                "truncreg with the option {o} is not translated yet"
            )));
        }
        if c.parts.main.trim().is_empty() {
            return self.cx_replay(c, opts, out, "truncreg");
        }
        let l = self.cx_line(c, opts, out, &[W::F, W::A, W::P, W::I], "truncreg")?;
        let ll = limit(opts, "ll")?;
        let ul = limit(opts, "ul")?;
        for h in [
            "stata_cx",
            "stata_trunc",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_truncreg({}, {}, {}, ll = {ll}, ul = {ul}{}, robust = {}{}, level = {}, nolog = {}, cmdline = {}, show = {})",
            l.src,
            r_text(&l.spec.dep),
            names(&l.spec.terms),
            l.wargs(),
            super::stats::r_logical(l.robust),
            l.cluster_arg(),
            r_number(l.level),
            super::stats::r_logical(opts.has("nolog", 3)),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(l.show)
        ));
        self.cx_after(
            c,
            &l,
            Fit::Truncreg,
            "truncreg",
            &[
                "cmdline",
                "cmd",
                "llopt",
                "ulopt",
                "marginsok",
                "predict",
                "chi2type",
                "deriv_useminbound",
                "opt",
                "wtype",
                "wexp",
                "vcetype",
                "clustvar",
                "vce",
                "title",
                "user",
                "ml_method",
                "technique",
                "which",
                "depvar",
                "properties",
            ],
            out,
        )?;
        if l.w.is_some() {
            out.caveat(super::weights::weight_caveat(&l.w));
        }
        out.caveat(
            "truncreg as Stata 19 fits it (least squares start on the rows kept, sigma as a parameter; Newton from exact derivatives where Stata's are numerical, so its later iterations can differ in the last digits)",
        );
        Ok(())
    }

    /// `cloglog depvar indepvars [if] [in] [weight] [, eform vce() level() nolog]`, or a replay.
    pub(super) fn cloglog(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("eform", 2),
            ("robust", 1),
            ("vce", 3),
            ("cluster", 2),
            ("level", 3),
            ("nolog", 3),
        ]) {
            return Err(no(format!(
                "cloglog with the option {o} is not translated yet"
            )));
        }
        let eform = opts.has("eform", 2);
        if c.parts.main.trim().is_empty() {
            if opts.0.iter().any(|o| !(o.is("level", 3) || o.is("eform", 2))) {
                return Err(no("cloglog's replay is translated with level() and eform only"));
            }
            let lvl = level(opts)? * 100.0;
            if self.censored_active() != Some("cloglog") {
                return Err(no(
                    "cloglog with no variables repeats the last cloglog, which is not known here",
                ));
            }
            for h in ["stata_cx", "stata_tobit", "stata_bin", "stata_fmt", "stata_tables"] {
                self.helpers.insert(h);
            }
            if !self.quiet(c) {
                out.line(format!(
                    "stata_cloglog_replay(level = {}, eform = {})",
                    r_number(lvl),
                    super::stats::r_logical(eform)
                ));
            }
            self.e_kept = true;
            return Ok(());
        }
        let l = self.cx_line(c, opts, out, &[W::F, W::P, W::I], "cloglog")?;
        if l.stopped {
            return Ok(());
        }
        for h in [
            "stata_cx",
            "stata_bin",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_cloglog({}, {}, {}{}, robust = {}{}, level = {}, nolog = {}, eform = {}, cmdline = {}, show = {})",
            l.src,
            r_text(&l.spec.dep),
            names(&l.spec.terms),
            l.wargs(),
            super::stats::r_logical(l.robust),
            l.cluster_arg(),
            r_number(l.level),
            super::stats::r_logical(opts.has("nolog", 3)),
            super::stats::r_logical(eform),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(l.show)
        ));
        self.cx_after(
            c,
            &l,
            Fit::Cloglog,
            "cloglog",
            &[
                "cmdline",
                "cmd",
                "predict",
                "marginsok",
                "marginsnotok",
                "chi2type",
                "wtype",
                "wexp",
                "deriv_useminbound",
                "opt",
                "vcetype",
                "clustvar",
                "vce",
                "title",
                "user",
                "ml_method",
                "technique",
                "which",
                "depvar",
                "properties",
            ],
            out,
        )?;
        if l.w.is_some() {
            out.caveat(super::weights::weight_caveat(&l.w));
        }
        out.caveat(
            "cloglog as Stata 19 fits it (Newton steps from the logit fit one iteration before its end; e(ll_0) at the constant-only estimate rounded to float)",
        );
        Ok(())
    }

    /// `hetprobit depvar indepvars [if] [in] [weight], het(varlist) [vce() level() nolog]`, or a replay.
    pub(super) fn hetprobit(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("het", 3),
            ("robust", 1),
            ("vce", 3),
            ("cluster", 2),
            ("level", 3),
            ("nolog", 3),
        ]) {
            return Err(no(format!(
                "hetprobit with the option {o} is not translated yet"
            )));
        }
        if c.parts.main.trim().is_empty() {
            return self.cx_replay(c, opts, out, "hetprobit");
        }
        let l = self.cx_line(c, opts, out, &[W::F, W::P, W::I], "hetprobit")?;
        if l.stopped {
            return Ok(());
        }
        let Some(het) = opts.arg("het", 3) else {
            out.line("stop(\"option het() required (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        };
        let het: Vec<String> = het.split_whitespace().map(str::to_string).collect();
        if het.is_empty() || het.iter().any(|v| !is_name(v)) {
            return Err(no(
                "hetprobit's het() is translated with plain variable names only",
            ));
        }
        for h in [
            "stata_cx",
            "stata_bin",
            "stata_fmt",
            "stata_tables",
            "stata_returned",
            "stata_eclass",
        ] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_hetprobit({}, {}, {}, {}{}, robust = {}{}, level = {}, nolog = {}, cmdline = {}, show = {})",
            l.src,
            r_text(&l.spec.dep),
            names(&l.spec.terms),
            names(&het),
            l.wargs(),
            super::stats::r_logical(l.robust),
            l.cluster_arg(),
            r_number(l.level),
            super::stats::r_logical(opts.has("nolog", 3)),
            r_text(&super::returned::cmdline(c)),
            super::stats::r_logical(l.show)
        ));
        self.cx_after(
            c,
            &l,
            Fit::Hetprobit,
            "hetprobit",
            &[
                "cmdline",
                "cmd",
                "predict",
                "method",
                "chi2_ct",
                "chi2type",
                "wtype",
                "wexp",
                "deriv_useminbound",
                "opt",
                "vcetype",
                "clustvar",
                "vce",
                "title",
                "user",
                "ml_method",
                "technique",
                "which",
                "depvar",
                "properties",
            ],
            out,
        )?;
        if l.w.is_some() {
            out.caveat(super::weights::weight_caveat(&l.w));
        }
        out.caveat(
            "hetprobit as Stata 19 fits it (from its own probit fit with the variance coefficients at zero; exact derivatives)",
        );
        Ok(())
    }

    /// `predict [type] newvar [if] [in] [, xb stdp pr(a,b) e(a,b) ystar(a,b)]` after the batch's models.
    pub(super) fn censored_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let cmd = self.censored_active().unwrap_or("tobit");
        let mut what: Option<(&str, Option<String>)> = None;
        for o in opts.0 {
            let found = if o.is("xb", 2) {
                Some("xb")
            } else if o.is("stdp", 3) {
                Some("stdp")
            } else if o.is("stdf", 4) {
                Some("stdf")
            } else if o.is("pr", 2) {
                Some("pr")
            } else if o.is("e", 1) {
                Some("e")
            } else if o.is("ystar", 5) {
                Some("ystar")
            } else if o.is("sigma", 5) {
                Some("sigma")
            } else {
                None
            };
            let Some(f) = found else {
                return Err(no(format!(
                    "predict after {cmd} with the option {} is not translated yet",
                    o.name
                )));
            };
            if what.is_some() {
                return Err(no("predict takes one statistic at a time"));
            }
            what = Some((f, Some(o.arg())));
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
        let binary = matches!(cmd, "cloglog" | "hetprobit");
        let (stat, arg) = what
            .clone()
            .unwrap_or((if binary { "pr" } else { "xb" }, None));
        let known: &[&str] = match cmd {
            "truncreg" => &["xb", "stdp"],
            "cloglog" => &["xb", "pr"],
            "hetprobit" => &["xb", "pr", "stdp", "sigma"],
            _ => &["xb", "stdp", "stdf", "pr", "e", "ystar"],
        };
        if !known.contains(&stat) {
            return Err(no(format!(
                "predict, {stat} after {cmd} is not translated yet"
            )));
        }
        if what.is_none() && !self.quiet(c) {
            out.line(if binary {
                "cat(\"(option pr assumed; Pr(y))\\n\")"
            } else {
                "cat(\"(option xb assumed; fitted values)\\n\")"
            });
        }
        // pr(a,b), e(a,b), ystar(a,b): each limit a number, a variable or . for none
        let limits = if !binary && matches!(stat, "pr" | "e" | "ystar") {
            let a = arg.unwrap_or_default();
            let Some((lo, hi)) = a.split_once(',') else {
                return Err(no(format!("{stat}(a,b) needs two limits")));
            };
            let one = |t: &str| -> Result<String, Untranslated> {
                let t = t.trim();
                if t == "." {
                    Ok("NULL".into())
                } else if let Ok(v) = t.parse::<f64>() {
                    Ok(r_number(v))
                } else if is_name(t) {
                    Ok(format!("stata_cx_num({base}[[{}]])", r_text(t)))
                } else {
                    Err(no(format!(
                        "{stat}() with the limit {t} is not translated (a number, a variable or .)"
                    )))
                }
            };
            format!(", a = {}, b = {}", one(lo)?, one(hi)?)
        } else {
            String::new()
        };
        for h in ["stata_cx", "stata_tobit", "stata_bin"] {
            self.helpers.insert(h);
        }
        let call = match cmd {
            "cloglog" => format!("stata_cloglog_predict(stata_e, {base}, {})", r_text(stat)),
            "hetprobit" => format!("stata_hetprobit_predict(stata_e, {base}, {})", r_text(stat)),
            _ => format!(
                "stata_tobit_predict(stata_e, {base}, {}{limits})",
                r_text(stat)
            ),
        };
        let v = stored(&call, st, &mut self.helpers, out);
        let v = match self.condition(c, out)? {
            Some(cond) => format!("dplyr::if_else({cond}, {v}, NA_real_)"),
            None => v,
        };
        out.package("dplyr");
        out.line(format!(
            "{base} <- dplyr::mutate({base}, {} = {v})",
            r_name(w)
        ));
        self.kinds.insert(w.to_string(), Kind::Number);
        self.storage.insert(w.to_string(), st);
        Ok(())
    }
}
