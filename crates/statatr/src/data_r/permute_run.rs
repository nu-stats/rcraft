//! The translator's side of the lines the runner writes for `permute` and
//! `simulate` (`dofile::resample`): `_rcraft_pm_begin`, `_rcraft_pm_collect`,
//! `_rcraft_pm_shuffle`, `_rcraft_pm_end`, and the same for `_rcraft_sm_`.
//! The rules are in `plan/stata/resampling.md`.

use super::resample::{exp_items, split_prefixed};
use super::*;

/// What a `permute` or `simulate` line in progress knows.
#[derive(Clone)]
pub(super) struct Loop {
    permvar: String,
    strata: Option<String>,
    items: Vec<String>,
    reps: usize,
    seed: Option<String>,
    nodots: bool,
    level: f64,
    saving: Option<(String, bool, bool)>,
    command: String,
    cmdname: String,
    program: bool,
    /// A program of class eclass: its model, if its last line fits one, is the command's.
    eprogram: bool,
    /// The data, and what is known of them, when the line began.
    before: Option<Box<Preserved>>,
    first: bool,
}

fn r_names(v: &[String]) -> String {
    v.iter()
        .map(|s| super::r_text(s))
        .collect::<Vec<_>>()
        .join(", ")
}

const RESAMPLING: [&str; 8] = [
    "bootstrap",
    "bsample",
    "jackknife",
    "permute",
    "simulate",
    "statsby",
    "rolling",
    "mi",
];

/// Splits `file [, replace double]` of `saving()`.
pub(super) fn saving_of(s: &str) -> Result<(String, bool, bool), Untranslated> {
    let (file, sub) = match super::random::top_level(s, ',') {
        Some(i) => (s.get(..i).unwrap_or(""), s.get(i + 1..).unwrap_or("")),
        None => (s, ""),
    };
    let (mut replace, mut double) = (false, false);
    for w in sub.split_whitespace() {
        match w {
            "replace" => replace = true,
            "double" => double = true,
            other => return Err(no(format!("saving(..., {other}) is not translated yet"))),
        }
    }
    Ok((super::file_name(file.trim(), "dta"), replace, double))
}

/// The name and expression of an item that is not `_b` or `_se`: `name=exp`
/// or `exp` (named `stem` and a number).
fn named_item(item: &str, unnamed: &mut usize, stem: &str) -> (String, String) {
    let (name, exp) = match item.split_once('=') {
        Some((n, e)) if super::is_name(n.trim()) && !e.starts_with('=') => {
            (n.trim().to_string(), e.trim().to_string())
        }
        _ => {
            *unnamed += 1;
            (format!("{stem}{unnamed}"), item.to_string())
        }
    };
    let exp = exp
        .strip_prefix('(')
        .and_then(|e| e.strip_suffix(')'))
        .map_or(exp.clone(), str::to_string);
    (name, exp)
}

/// The names and expressions of the items that are not `_b` or `_se`.
fn named_items(items: &[String], stem: &str) -> (Vec<String>, Vec<String>) {
    let mut unnamed = 0;
    let mut names = Vec::new();
    let mut exps = Vec::new();
    for i in items.iter().filter(|i| *i != "_b" && *i != "_se") {
        let (n, e) = named_item(i, &mut unnamed, stem);
        names.push(n);
        exps.push(e);
    }
    (names, exps)
}

impl Session {
    /// `_rcraft_rs_fail code message`: the line the runner knows Stata stops at.
    pub(super) fn rs_fail(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let (code, msg) = first_word(&c.parts.main);
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!("{msg} (Stata: r({code}))"))
        ));
        Ok(())
    }

    /// The pieces of a `permute` or `simulate` line.
    fn loop_parts(&mut self, c: &Command, permute: bool) -> Result<Loop, Untranslated> {
        // the line is `program|eprogram|builtin <what follows the word>`
        let text = super::returned::after_name(c);
        let (kindword, line) = first_word(text.trim());
        let name = if permute { "permute" } else { "simulate" };
        let cmd = crate::reader::read(&format!("{name} {line}"));
        let Some(cmd) = cmd.commands().first().map(|c| (*c).clone()) else {
            return Err(no("the line could not be read"));
        };
        let p = split_prefixed(&cmd, name)?;
        let opts = Opts(&p.options);
        let known: &[(&str, usize)] = if permute {
            &[
                ("reps", 1),
                ("seed", 4),
                ("nodots", 4),
                ("strata", 6),
                ("saving", 4),
                ("level", 1),
            ]
        } else {
            &[("reps", 1), ("seed", 4), ("nodots", 4), ("saving", 4)]
        };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "{name} with the option {o} is not translated yet"
            )));
        }
        let reps: usize = opts
            .arg("reps", 1)
            .and_then(|r| r.trim().parse().ok())
            .unwrap_or(100);
        let mut items = exp_items(&p.explist);
        let mut permvar = String::new();
        if permute {
            if items.is_empty() {
                return Err(no("permute needs the variable to permute (Stata: r(100))"));
            }
            permvar = items.remove(0);
            if !super::is_name(&permvar) {
                return Err(no("permute takes one variable to permute"));
            }
        }
        let strata = opts.arg("strata", 6).map(|v| v.trim().to_string());
        if let Some(v) = &strata {
            if !super::is_name(v) {
                return Err(no(
                    "permute, strata() of more than one variable is not translated yet",
                ));
            }
        }
        let saving = match opts.arg("saving", 4) {
            Some(s) => Some(saving_of(&s)?),
            None => None,
        };
        if !permute && saving.as_ref().is_some_and(|s| s.2) {
            return Err(no("simulate, saving(, double) is not translated yet"));
        }
        let cmdname = crate::reader::read(&p.inner)
            .commands()
            .first()
            .map(|c| c.name.clone())
            .unwrap_or_default();
        if RESAMPLING.contains(&cmdname.as_str()) {
            return Err(no(format!("{name} of {cmdname} is not translated yet")));
        }
        let level = if permute {
            super::model::level(&opts)? * 100.0
        } else {
            95.0
        };
        Ok(Loop {
            permvar,
            strata,
            items,
            reps,
            seed: opts.arg("seed", 4),
            nodots: opts.has("nodots", 4),
            level,
            saving,
            command: p.inner.trim().to_string(),
            cmdname,
            program: kindword == "program" || kindword == "eprogram",
            eprogram: kindword == "eprogram",
            before: None,
            first: true,
        })
    }

    /// The R that evaluates an item after one run of the command.
    #[allow(clippy::too_many_arguments)]
    fn loop_item(
        &mut self,
        item: &str,
        eclass: bool,
        cmdname: &str,
        stem: &str,
        unnamed: &mut usize,
        complete: bool,
        out: &mut Out,
    ) -> Result<String, Untranslated> {
        if item == "_b" || item == "_se" {
            if !eclass || !matches!(cmdname, "regress" | "logit" | "probit") {
                return Err(no(format!(
                    "{item} is translated after regress, logit and probit; not here yet"
                )));
            }
            if item == "_se" && stem == "_pm_" && matches!(cmdname, "logit" | "probit") {
                return Err(no(
                    "_se after logit and probit in permute is not translated yet (its equation names are not decoded)",
                ));
            }
            return Ok(format!(
                "stata_sb_vec({})",
                if item == "_se" { "TRUE" } else { "FALSE" }
            ));
        }
        let (name, exp) = named_item(item, unnamed, stem);
        let (code, kind) = match self.value(&exp, out) {
            Ok(v) => v,
            // a name of r() that a command whose r() RCraft keeps whole does not leave is
            // missing in Stata too (permute then stops with r(322))
            Err(e)
                if complete
                    && exp.starts_with("r(")
                    && exp.ends_with(')')
                    && e.reason.contains("is not kept in R yet (RCraft keeps r()") =>
            {
                ("NA_real_".to_string(), Kind::Number)
            }
            Err(e) => return Err(e),
        };
        if kind == Kind::Text {
            return Err(no(format!("{exp} is a string (Stata: type mismatch)")));
        }
        Ok(format!("stats::setNames({code}, {})", super::r_text(&name)))
    }

    /// The items as the R vector of one run's values.
    fn loop_values(
        &mut self,
        l: &Loop,
        eclass: bool,
        stem: &str,
        out: &mut Out,
    ) -> Result<String, Untranslated> {
        let items = if l.items.is_empty() && eclass {
            vec!["_b".to_string()]
        } else {
            l.items.clone()
        };
        if items.is_empty() {
            return Err(no("expression list required (Stata: r(100))"));
        }
        let vecs = items.iter().filter(|i| *i == "_b" || *i == "_se").count();
        if vecs > 0 && vecs != items.len() {
            return Err(no("_b or _se with other expressions is not translated yet"));
        }
        if vecs > 0 && !(items == ["_b"] || items == ["_b", "_se"] || items == ["_se"]) {
            return Err(no("this order of _b and _se is not translated yet"));
        }
        let mut unnamed = 0;
        let mut codes = Vec::new();
        // summarize without options leaves exactly the r() RCraft keeps
        let complete = l.cmdname == "summarize" && !l.command.contains(',');
        for item in &items {
            codes.push(self.loop_item(
                item,
                eclass,
                &l.cmdname,
                stem,
                &mut unnamed,
                complete,
                out,
            )?);
        }
        Ok(format!("c({})", codes.join(", ")))
    }

    fn resampling_helpers(&mut self, which: &'static str) {
        for h in [
            "stata_rng",
            which,
            "stata_statsby",
            "stata_fmt",
            "stata_float",
            "stata_names",
            "stata_returned",
            "stata_bootstrap",
        ] {
            self.helpers.insert(h);
        }
    }

    /// `_rcraft_pm_begin program|builtin <the permute line>`.
    pub(super) fn pm_begin(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let mut l = self.loop_parts(c, true)?;
        let d = self.data()?;
        self.resampling_helpers("stata_permute");
        if let Some(s) = l.seed.clone() {
            self.set_seed(s.trim(), out)?;
        }
        l.before = Some(Box::new(self.data_state()));
        let sv = l.saving.as_ref().map(|(p, r, _)| {
            format!(
                ", saving = {}, replace = {}",
                super::r_text(p),
                super::stats::r_logical(*r)
            )
        });
        if sv.is_some() {
            out.package("haven");
        }
        out.line(format!(
            "stata_rep_failed <- FALSE\nstata_pm_full <- {d}\nstata_pm_begin({d}, {}, {}, {}, {}{}, level = {})",
            super::r_text(&l.permvar),
            l.strata
                .as_ref()
                .map_or("NULL".into(), |s| super::r_text(s)),
            l.reps,
            super::stats::r_logical(l.nodots),
            sv.unwrap_or_default(),
            super::r_number(l.level)
        ));
        self.resampling = Some(Box::new(l));
        Ok(())
    }

    /// `_rcraft_pm_collect`: the values of the expressions after one run.
    pub(super) fn pm_collect(&mut self, _c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let Some(mut l) = self.resampling.take() else {
            return Err(no("permute: the line that began it is not known"));
        };
        let r = self.pm_collect_in(&mut l, out);
        self.resampling = Some(l);
        r
    }

    fn pm_collect_in(&mut self, l: &mut Loop, out: &mut Out) -> Result<(), Untranslated> {
        let eclass = (!l.program || l.eprogram) && self.prev_eclass;
        if l.eprogram {
            if let super::returned::Returned::Kept { cmd, .. } = &self.e_ret {
                l.cmdname = cmd.clone();
            }
        }
        let d = self.data()?;
        let vec = self.loop_values(l, eclass, "_pm_", out)?;
        if l.first {
            l.first = false;
            let (_, texts) = named_items(&l.items, "_pm_");
            out.line(format!(
                "stata_pm_vals <- if (is.null(stata_pm_st$err)) {vec} else numeric()\nstata_pm_first({d}, stata_pm_vals, {}, {}, {}{})",
                if texts.is_empty() {
                    "character()".to_string()
                } else {
                    format!("c({})", r_names(&texts))
                },
                super::stats::r_logical(eclass),
                super::r_text(&l.cmdname),
                if eclass { ", esample = stata_esample" } else { "" }
            ));
        } else {
            out.line(format!(
                "stata_pm_row(if (isTRUE(get0(\"stata_rep_failed\"))) NULL else {vec})
stata_rep_failed <- FALSE"
            ));
        }
        Ok(())
    }

    /// `_rcraft_pm_shuffle`: the permutation before a replication.
    pub(super) fn pm_shuffle(&mut self, _c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let d = self.data()?;
        out.line(format!("{d} <- stata_pm_shuffle({d})"));
        Ok(())
    }

    /// `_rcraft_pm_end`: the table, r(), the file, and the data as they were.
    pub(super) fn pm_end(&mut self, _c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let Some(l) = self.resampling.take() else {
            return Err(no("permute: the line that began it is not known"));
        };
        let items = if l.items.is_empty() {
            vec!["_b".to_string()]
        } else {
            l.items.clone()
        };
        let byvec = items.iter().any(|i| i == "_b" || i == "_se");
        let eq = items.iter().any(|i| i == "_b") && items.iter().any(|i| i == "_se");
        let (names, exps) = named_items(&items, "_pm_");
        let d = l
            .before
            .as_ref()
            .and_then(|p| p.data.clone())
            .or_else(|| self.data.clone())
            .ok_or_else(|| no("permute: no data"))?;
        out.line(format!(
            "{d} <- stata_pm_full\nstata_r <- stata_pm_end({}, {}, {}, {}, list(name = c({}), exp = c({})), saving_double = {})\nrm(stata_pm_full)",
            super::r_text(&l.permvar),
            super::r_text(&l.command),
            super::stats::r_logical(byvec),
            super::stats::r_logical(eq),
            r_names(&names),
            r_names(&exps),
            super::stats::r_logical(l.saving.as_ref().is_some_and(|s| s.2))
        ));
        if let Some(p) = l.before.clone() {
            self.set_data_state(*p);
        }
        self.clear_e_after_resampling(out);
        self.r_new = Some((R_PERMUTE, "permute".into()));
        self.esample = false;
        Ok(())
    }

    /// e() is empty after `permute` and `simulate`.
    fn clear_e_after_resampling(&mut self, out: &mut Out) {
        self.estimates = None;
        self.estimates_lost =
            Some("the line before it was permute or simulate, which leaves no model".into());
        self.active_store = None;
        self.e_text.clear();
        self.helpers.insert("stata_returned");
        out.line("stata_e <- structure(list(), stata_sample = FALSE)");
        self.e_ret = super::returned::Returned::Program {
            cmd: "ereturn".into(),
            texts: Vec::new(),
        };
    }

    /// `_rcraft_sm_begin program|builtin <the simulate line>`.
    pub(super) fn sm_begin(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let mut l = self.loop_parts(c, false)?;
        self.resampling_helpers("stata_simulate");
        if let Some(s) = l.seed.clone() {
            self.set_seed(s.trim(), out)?;
        }
        // the data stay as the program leaves them, replication after replication
        l.before = Some(Box::new(self.data_state()));
        let (names, exps) = named_items(&l.items, "_sim_");
        let sv = l.saving.as_ref().map(|(p, r, _)| {
            format!(
                ", saving = {}, replace = {}",
                super::r_text(p),
                super::stats::r_logical(*r)
            )
        });
        if sv.is_some() {
            out.package("haven");
        }
        out.line(format!(
            "stata_rep_failed <- FALSE
stata_sm_begin({}, {}, c({}), c({}), {}{})",
            l.reps,
            super::stats::r_logical(l.nodots),
            r_names(&names),
            r_names(&exps),
            super::r_text(&l.command),
            sv.unwrap_or_default()
        ));
        self.resampling = Some(Box::new(l));
        Ok(())
    }

    /// `_rcraft_sm_collect`: the values of the expressions after one run.
    pub(super) fn sm_collect(&mut self, _c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let Some(mut l) = self.resampling.clone() else {
            return Err(no("simulate: the line that began it is not known"));
        };
        // a program ends with the model it fitted, if its last command is one
        if l.program {
            if let super::returned::Returned::Kept { cmd, .. } = &self.e_ret {
                l.cmdname = cmd.clone();
            }
        }
        let vec = self.loop_values(&l, self.prev_eclass, "_sim_", out)?;
        out.line(format!(
            "stata_sm_row(if (isTRUE(get0(\"stata_rep_failed\"))) NULL else stata_sb_f({vec}))
stata_rep_failed <- FALSE"
        ));
        Ok(())
    }

    /// `_rcraft_sm_end`: the results are the data now.
    pub(super) fn sm_end(&mut self, _c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let Some(l) = self.resampling.take() else {
            return Err(no("simulate: the line that began it is not known"));
        };
        let (names, exps) = named_items(&l.items, "_sim_");
        let d = l
            .before
            .as_ref()
            .and_then(|p| p.data.clone())
            .unwrap_or_else(|| "data".to_string());
        let labels = names
            .iter()
            .zip(&exps)
            .map(|(n, e)| format!("{} = {}", super::r_text(n), super::r_text(e)))
            .collect::<Vec<_>>()
            .join(", ");
        out.line(format!(
            "{d} <- stata_sm_end(c({labels}){}, cmdname = {})\nrm(stata_sm_full)",
            l.saving.as_ref().map_or(String::new(), |(p, _, _)| format!(
                ", saving = {}",
                super::r_text(p)
            )),
            super::r_text(&l.cmdname)
        ));
        // the results are the data; r() is what the last run left
        self.data = Some(d);
        self.kinds.clear();
        self.storage.clear();
        for n in &names {
            self.kinds.insert(n.clone(), Kind::Number);
            self.storage.insert(n.clone(), Storage::Float);
        }
        self.forget_ts();
        self.file = None;
        self.esample = false;
        self.chars_lost = Some("simulate replaces the data".into());
        self.clear_e_after_resampling(out);
        Ok(())
    }
}

/// The names `permute` keeps in r().
const R_PERMUTE: &[&str] = &[
    "N",
    "n_reps",
    "n_strata",
    "k_exp",
    "k_eexp",
    "level",
    "cmd",
    "command",
    "permvar",
    "strata",
    "title",
    "b",
    "n",
    "c_lower",
    "c_upper",
    "p_lower",
    "p_upper",
    "p_twosided",
    "se_p_lower",
    "se_p_upper",
    "se_p_twosided",
    "ci_p_lower",
    "ci_p_upper",
    "ci_p_twosided",
];
