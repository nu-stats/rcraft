//! The multivariate commands (batch MV, `plan/stata/multivariate.md`): `manova`,
//! `canon`, `discrim lda` and `mds`, each an R helper in `multi/` that makes
//! Stata 19's numbers and leaves its e(). The output the user sees is R's.

use super::*;

mod cmds;

pub(super) const MANOVA_HELPER: &str = include_str!("multi/manova.R");
pub(super) const CANON_HELPER: &str = include_str!("multi/canon.R");
pub(super) const DISCRIM_HELPER: &str = include_str!("multi/discrim.R");
pub(super) const MDS_HELPER: &str = include_str!("multi/mds.R");

/// What a command leaves as e(): its text names and the r() it clears.
fn keep_e(s: &mut Session, cmd: &str, texts: &[&str], extra: &[String], why: &str, out: &mut Out) {
    out.line("if (exists(\"stata_b\")) rm(stata_b)");
    out.line("stata_r <- list()");
    s.estimates = None;
    s.estimates_lost = Some(why.into());
    s.e_new = true;
    s.e_text.clear();
    for t in texts {
        s.e_text.insert((*t).to_string());
    }
    for t in extra {
        s.e_text.insert(t.clone());
    }
    s.e_ret = super::returned::Returned::Kept {
        cmd: cmd.into(),
        model: String::new(),
        names: &[],
    };
    s.r_new = Some((&[], cmd.into()));
}

/// The parts of one anova-style term as manova reads them: `g` and `i.g` are
/// categorical, `c.x` continuous; `a#b` is one term, `a##b` the main effects
/// and every interaction among them, in Stata's order.
fn manova_terms(rhs: &str) -> Result<Vec<Vec<(String, bool)>>, Untranslated> {
    let part = |w: &str| -> Result<(String, bool), Untranslated> {
        let (v, cont) = if let Some(v) = w.strip_prefix("c.") {
            (v, true)
        } else if let Some(v) = w.strip_prefix("i.") {
            (v, false)
        } else {
            (w, false)
        };
        if !is_name(v) {
            return Err(no(format!(
                "manova {w}: only variables written out, with c. or i., are translated"
            )));
        }
        Ok((v.to_string(), cont))
    };
    let mut terms: Vec<Vec<(String, bool)>> = Vec::new();
    for w in rhs.split_whitespace() {
        if w.contains("##") {
            let ps: Vec<(String, bool)> = w.split("##").map(part).collect::<Result<_, _>>()?;
            if ps.len() > 4 {
                return Err(no(
                    "manova: ## of more than four variables is not translated",
                ));
            }
            for size in 1..=ps.len() {
                for mask in 1u32..(1u32 << ps.len()) {
                    if mask.count_ones() as usize != size {
                        continue;
                    }
                    let t: Vec<(String, bool)> = ps
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| mask & (1 << i) != 0)
                        .map(|(_, p)| p.clone())
                        .collect();
                    terms.push(t);
                }
            }
        } else if w.contains('#') {
            terms.push(w.split('#').map(part).collect::<Result<_, _>>()?);
        } else {
            terms.push(vec![part(w)?]);
        }
    }
    Ok(terms)
}

fn r_terms(terms: &[Vec<(String, bool)>]) -> String {
    let t: Vec<String> = terms
        .iter()
        .map(|t| {
            let p: Vec<String> = t
                .iter()
                .map(|(v, c)| {
                    format!(
                        "list(v = {}, c = {})",
                        r_text(v),
                        if *c { "TRUE" } else { "FALSE" }
                    )
                })
                .collect();
            format!("list({})", p.join(", "))
        })
        .collect();
    format!("list({})", t.join(", "))
}

impl Session {
    /// `manova depvars = terms [if] [in]`.
    pub(super) fn manova(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "manova")?;
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "manova with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("manova with weights is not translated yet"));
        }
        let Some((lhs, rhs)) = c.parts.main.split_once('=') else {
            out.line(
                "stop(\"equal sign required between dependent variables and terms of the model (Stata: r(198))\", call. = FALSE)",
            );
            return Ok(());
        };
        let deps: Vec<String> = lhs.split_whitespace().map(str::to_string).collect();
        if deps.is_empty() || deps.iter().any(|v| !is_name(v)) {
            return Err(no(
                "manova with dependent variables not written out is not translated yet",
            ));
        }
        let terms = manova_terms(rhs)?;
        if terms.is_empty() {
            return Err(no("manova needs at least one term"));
        }
        let rows = self.rows_for(c, out)?;
        for h in ["stata_manova", "stata_fmt"] {
            self.helpers.insert(h);
        }
        let dv: Vec<String> = deps.iter().map(|v| r_text(v)).collect();
        out.line(format!(
            "stata_e <- stata_manova({rows}, c({}), {}, cmdline = {}{})",
            dv.join(", "),
            r_terms(&terms),
            r_text(&super::returned::cmdline(c)),
            if self.quiet(c) { ", show = FALSE" } else { "" }
        ));
        let extra: Vec<String> = (1..=terms.len()).map(|i| format!("term_{i}")).collect();
        keep_e(
            self,
            "manova",
            &[
                "cmdline",
                "cmd",
                "marginsnotok",
                "marginsdefault",
                "p_F",
                "F",
                "r2",
                "rmse",
                "predict",
                "estat_cmd",
                "properties",
                "depvar",
                "indepvars",
            ],
            &extra,
            "manova's results are not a model RCraft's predict, test or margins read",
            out,
        );
        Ok(())
    }
}
