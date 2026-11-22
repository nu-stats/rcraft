//! `canon`, `discrim lda` (with predict and estat classtable after it) and `mds`.

use super::super::*;
use super::keep_e;

/// A number option's value: a whole number above 0.
fn count(opts: &Opts, name: &str, min: usize, cmd: &str) -> Result<Option<u32>, Untranslated> {
    match opts.arg(name, min) {
        None => Ok(None),
        Some(k) => k
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|v| *v > 0)
            .map(Some)
            .ok_or_else(|| no(format!("{cmd}: {name}({k}) needs a whole number above 0"))),
    }
}

/// The names in `text` when each is a plain variable name.
fn plain_names(text: &str) -> Option<Vec<String>> {
    let v: Vec<String> = text.split_whitespace().map(str::to_string).collect();
    (!v.is_empty() && v.iter().all(|w| is_name(w))).then_some(v)
}

fn r_names(v: &[String]) -> String {
    let q: Vec<String> = v.iter().map(|s| r_text(s)).collect();
    format!("c({})", q.join(", "))
}

fn rlog(b: bool) -> &'static str {
    if b {
        "TRUE"
    } else {
        "FALSE"
    }
}

impl Session {
    /// The command whose results are active, when it is `discrim`: predict and estat are
    /// translated after it.
    pub(in crate::data_r) fn multi_active(&self) -> Option<&'static str> {
        if self.estimates.is_some() {
            return None;
        }
        match &self.e_ret {
            super::super::returned::Returned::Kept { cmd, .. }
                if cmd == "discrim"
                    && self
                        .estimates_lost
                        .as_deref()
                        .is_some_and(|w| w.contains("discrim's")) =>
            {
                Some("discrim")
            }
            _ => None,
        }
    }

    /// `canon (varlist1) (varlist2) [if] [in] [, lc(#) first(#) stdcoef]`.
    pub(in crate::data_r) fn canon(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "canon")?;
        if let Some(o) = opts.other(&[("lc", 2), ("first", 5), ("stdcoef", 4)]) {
            return Err(no(format!(
                "canon with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("canon with weights is not translated yet"));
        }
        let mut sets: Vec<Vec<String>> = Vec::new();
        let mut rest = c.parts.main.trim();
        while let Some(r) = rest.trim_start().strip_prefix('(') {
            let Some((inner, after)) = r.split_once(')') else {
                return Err(no("canon: a parenthesis is not closed"));
            };
            let Some(v) = plain_names(inner) else {
                return Err(no(
                    "canon with variables not written out is not translated yet",
                ));
            };
            sets.push(v);
            rest = after;
        }
        let [first_set, second_set] = sets.as_slice() else {
            return Err(no("canon takes two variable lists in parentheses"));
        };
        if !rest.trim().is_empty() {
            return Err(no("canon takes two variable lists in parentheses"));
        }
        let lc = count(opts, "lc", 2, "canon")?;
        let first = count(opts, "first", 5, "canon")?;
        let rows = self.rows_for(c, out)?;
        for h in ["stata_manova", "stata_canon", "stata_fmt"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_canon({rows}, {}, {}{}{}{}, cmdline = {}{})",
            r_names(first_set),
            r_names(second_set),
            lc.map(|v| format!(", lc = {v}")).unwrap_or_default(),
            first.map(|v| format!(", first = {v}")).unwrap_or_default(),
            if opts.has("stdcoef", 4) {
                ", stdcoef = TRUE"
            } else {
                ""
            },
            r_text(&super::super::returned::cmdline(c)),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        keep_e(
            self,
            "canon",
            &[
                "cmdline",
                "marginsnotok",
                "cmd",
                "predict",
                "estat_cmd",
                "properties",
            ],
            &[],
            "canon's results are not a model RCraft's predict, test or margins read",
            out,
        );
        Ok(())
    }

    /// `discrim lda varlist [if] [in], group(var) [priors(equal|proportional) notable]`.
    pub(in crate::data_r) fn discrim(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "discrim")?;
        if let Some(o) = opts.other(&[("group", 1), ("priors", 2), ("notable", 3)]) {
            return Err(no(format!(
                "discrim with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("discrim with weights is not translated yet"));
        }
        let (sub, vars) = first_word(&c.parts.main);
        if sub != "lda" {
            return Err(no(format!(
                "discrim {sub} is not translated yet (only discrim lda)"
            )));
        }
        let Some(vars) = plain_names(vars) else {
            return Err(no(
                "discrim with variables not written out is not translated yet",
            ));
        };
        let Some(group) = opts.arg("group", 1).map(|g| g.trim().to_string()) else {
            out.line("stop(\"option group() required (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        };
        if !is_name(&group) {
            return Err(no("discrim: group() takes one variable written out"));
        }
        let priors = match opts.arg("priors", 2) {
            None => r_text("equal"),
            Some(p) => {
                let p = p.trim();
                if !p.is_empty() && "equal".starts_with(p) {
                    r_text("equal")
                } else if !p.is_empty() && "proportional".starts_with(p) {
                    r_text("proportional")
                } else {
                    return Err(no(format!(
                        "discrim priors({p}) is not translated yet (equal and proportional are)"
                    )));
                }
            }
        };
        let rows = self.rows_for(c, out)?;
        for h in ["stata_manova", "stata_discrim", "stata_fmt"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_discrim({rows}, {}, {}, priors = {priors}, notable = {}, cmdline = {}{})",
            r_names(&vars),
            r_text(&group),
            rlog(opts.has("notable", 3)),
            r_text(&super::super::returned::cmdline(c)),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        keep_e(
            self,
            "discrim",
            &[
                "cmd",
                "marginsnotok",
                "cmdline",
                "subcmd",
                "title",
                "predict",
                "estat_cmd",
                "grouplabels",
                "ties",
                "varlist",
                "groupvar",
                "properties",
            ],
            &[],
            "discrim's results are not a model RCraft's test or margins read; predict and estat classtable are translated",
            out,
        );
        Ok(())
    }

    /// `predict [type] newvar [if] [in]` after `discrim lda`: the group classification.
    pub(in crate::data_r) fn discrim_predict(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("classification", 5)]) {
            return Err(no(format!(
                "predict after discrim with the option {o} is not translated yet"
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
        if !opts.has("classification", 5) && !self.quiet(c) {
            out.line("cat(\"(option classification assumed; group classification)\\n\")");
        }
        self.helpers.insert("stata_manova");
        self.helpers.insert("stata_discrim");
        self.helpers.insert("stata_float");
        let v = stored(
            &format!("stata_discrim_classify({base}, stata_e)"),
            st,
            &mut self.helpers,
            out,
        );
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

    /// `estat classtable` after `discrim lda`.
    pub(in crate::data_r) fn discrim_estat(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let (sub, rest) = first_word(&c.parts.main);
        if !(sub.len() >= 3 && "classtable".starts_with(sub))
            || !rest.is_empty()
            || opts.other(&[]).is_some()
        {
            return Err(no(
                "estat after discrim: only classtable without options is translated",
            ));
        }
        self.helpers.insert("stata_manova");
        self.helpers.insert("stata_discrim");
        if !self.quiet(c) {
            out.line(
                "stata_discrim_table(stata_e, attr(stata_e, \"class_counts\"), \"Resubstitution classification table\")",
            );
        }
        Ok(())
    }

    /// `mds varlist [if] [in], id(var) [dimension(#) std noplot]`.
    pub(in crate::data_r) fn mds(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "mds")?;
        if let Some(o) = opts.other(&[("id", 2), ("dimension", 3), ("std", 3), ("noplot", 3)]) {
            return Err(no(format!("mds with the option {o} is not translated yet")));
        }
        if c.parts.weight.is_some() {
            return Err(no("mds with weights is not translated yet"));
        }
        let Some(vars) = plain_names(&c.parts.main) else {
            return Err(no(
                "mds with variables not written out is not translated yet",
            ));
        };
        let Some(id) = opts.arg("id", 2).map(|g| g.trim().to_string()) else {
            out.line("stop(\"option id() required (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        };
        if !is_name(&id) {
            return Err(no("mds: id() takes one variable written out"));
        }
        let dims = count(opts, "dimension", 3, "mds")?.unwrap_or(2);
        let idtype = match self.storage.get(&id) {
            Some(Storage::Byte) => "byte",
            Some(Storage::Int) => "int",
            Some(Storage::Long) => "long",
            Some(Storage::Float) => "float",
            Some(Storage::Str) => {
                return Err(no("mds with a string id() is not translated yet"));
            }
            _ => "double",
        };
        let rows = self.rows_for(c, out)?;
        for h in ["stata_manova", "stata_mds", "stata_fmt"] {
            self.helpers.insert(h);
        }
        out.line(format!(
            "stata_e <- stata_mds({rows}, {}, {}, dims = {dims}, std = {}, cmdline = {}, idtype = {}{})",
            r_names(&vars),
            r_text(&id),
            rlog(opts.has("std", 3)),
            r_text(&super::super::returned::cmdline(c)),
            r_text(idtype),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        keep_e(
            self,
            "mds",
            &[
                "cmdline",
                "estat_cmd",
                "cmd",
                "marginsnotok",
                "predict",
                "dtype",
                "dname",
                "duplicates",
                "idtype",
                "id",
                "varlist",
                "method",
                "unique",
                "norm",
                "properties",
            ],
            &[],
            "mds's results are not a model RCraft's predict, test or margins read",
            out,
        );
        Ok(())
    }
}
