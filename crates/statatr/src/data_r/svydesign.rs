//! `svyset`: the survey design, as Stata 19 prints and keeps it
//! (`plan/stata/svy.md`). Stages written with `||`, each with its own
//! `strata()` and `fpc()`; `singleunit()`; `[pweight=]` or `[iweight=]`; the
//! bare `svyset` that shows the design and `svyset, clear`. The variance
//! methods other than linearized, replicate weights and poststrata are
//! refused.

use super::*;
use crate::lang::syntax::{take_weight, WeightKind};

/// The most stages kept (r(strata1) ... r(fpc5)).
pub(super) const MAX_STAGES: usize = 5;

/// What `svyset` leaves in r(): the names it may hold (any it does not set is missing).
pub(super) const R_SVYSET: &[&str] = &[
    "stages",
    "settings",
    "strata1",
    "su1",
    "fpc1",
    "strata2",
    "su2",
    "fpc2",
    "strata3",
    "su3",
    "fpc3",
    "strata4",
    "su4",
    "fpc4",
    "strata5",
    "su5",
    "fpc5",
    "vce",
    "singleunit",
    "wexp",
    "wvar",
    "wtype",
];

/// One stage of the design.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SvyStage {
    /// `strata()`: none means one stratum.
    pub strata: Option<String>,
    /// The sampling unit variable: none means each observation.
    pub su: Option<String>,
    /// `fpc()`: none means no correction.
    pub fpc: Option<String>,
}

/// The design `svyset` sets.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SvyDesign {
    pub stages: Vec<SvyStage>,
    /// `pweight` or `iweight`, with the weight variable.
    pub weight: Option<(&'static str, String)>,
    /// `missing`, `certainty`, `scaled` or `centered`.
    pub singleunit: String,
    /// Stages after the first one with no fpc were dropped (a note was due).
    pub dropped: Option<usize>,
}

/// What a `svyset` line asks for.
pub(super) enum Svyset {
    /// No arguments: show the design.
    Show,
    /// `svyset, clear`.
    Clear,
    Set(SvyDesign),
}

impl SvyDesign {
    /// The design's lines as Stata prints them, the labels right-aligned.
    pub(super) fn lines(&self) -> String {
        let mut rows: Vec<(String, String)> = Vec::new();
        match &self.weight {
            Some((kind, w)) => rows.push((
                if *kind == "iweight" {
                    "Importance weights"
                } else {
                    "Sampling weights"
                }
                .to_string(),
                w.clone(),
            )),
            None => rows.push(("Sampling weights".to_string(), "<none>".to_string())),
        }
        rows.push(("VCE".into(), "linearized".into()));
        rows.push(("Subpop. df".into(), "off".into()));
        rows.push(("Single unit".into(), self.singleunit.clone()));
        for (k, s) in self.stages.iter().enumerate() {
            let k = k + 1;
            rows.push((
                format!("Strata {k}"),
                s.strata.clone().unwrap_or_else(|| "<one>".into()),
            ));
            rows.push((
                format!("Sampling unit {k}"),
                s.su.clone().unwrap_or_else(|| "<observations>".into()),
            ));
            rows.push((
                format!("FPC {k}"),
                s.fpc.clone().unwrap_or_else(|| "<zero>".into()),
            ));
        }
        let width = rows.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
        rows.iter()
            .map(|(l, v)| format!("{l:>width$}: {v}\n"))
            .collect()
    }

    /// r(settings): the stages as written, with the single-unit rule and
    /// the variance method after the last one.
    pub(super) fn settings(&self) -> String {
        let mut out = String::new();
        let last = self.stages.len().saturating_sub(1);
        for (k, s) in self.stages.iter().enumerate() {
            if k > 0 {
                out.push_str(" || ");
            }
            out.push_str(s.su.as_deref().unwrap_or("_n"));
            if k == 0 {
                if let Some((kind, w)) = &self.weight {
                    out.push_str(&format!(" [{kind}= {w}]"));
                }
            }
            let mut opts: Vec<String> = Vec::new();
            if let Some(v) = &s.strata {
                opts.push(format!("strata({v})"));
            }
            if let Some(v) = &s.fpc {
                opts.push(format!("fpc({v})"));
            }
            if k == last {
                opts.push(format!("singleunit({})", self.singleunit));
                opts.push("vce(linearized)".into());
            }
            if !opts.is_empty() {
                out.push_str(", ");
                out.push_str(&opts.join(" "));
            }
        }
        out
    }

    /// The R that sets r() as `svyset` leaves it: Stata's `return list` shows
    /// the macros last made first.
    pub(super) fn r_list(&self) -> String {
        let mut items: Vec<String> = vec![
            format!("stages = {}", self.stages.len()),
            format!("settings = {}", r_text(&self.settings())),
        ];
        for (k, s) in self.stages.iter().enumerate().rev() {
            let k = k + 1;
            if let Some(f) = &s.fpc {
                items.push(format!("fpc{k} = {}", r_text(f)));
            }
            if let Some(v) = &s.strata {
                items.push(format!("strata{k} = {}", r_text(v)));
            }
            if let Some(v) = &s.su {
                items.push(format!("su{k} = {}", r_text(v)));
            }
        }
        items.push("vce = \"linearized\"".into());
        items.push(format!("singleunit = {}", r_text(&self.singleunit)));
        if let Some((kind, w)) = &self.weight {
            items.push(format!("wexp = {}", r_text(&format!("= {w}"))));
            items.push(format!("wvar = {}", r_text(w)));
            items.push(format!("wtype = {}", r_text(kind)));
        }
        format!("list({})", items.join(", "))
    }

    /// The variables the design reads.
    pub(super) fn variables(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for s in &self.stages {
            for x in [&s.strata, &s.su, &s.fpc].into_iter().flatten() {
                if !v.contains(x) {
                    v.push(x.clone());
                }
            }
        }
        if let Some((_, w)) = &self.weight {
            if !v.contains(w) {
                v.push(w.clone());
            }
        }
        v
    }
}

/// The text split at `sep` where it is outside parentheses and brackets.
fn split_top<'a>(text: &'a str, sep: &str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let (mut depth, mut from) = (0i32, 0usize);
    let b = text.as_bytes();
    let n = sep.len();
    let mut i = 0;
    while i < b.len() {
        match b.get(i) {
            Some(b'(') | Some(b'[') => depth += 1,
            Some(b')') | Some(b']') => depth -= 1,
            _ => {}
        }
        if depth == 0 && text.get(i..).is_some_and(|t| t.starts_with(sep)) {
            parts.push(text.get(from..i).unwrap_or(""));
            i += n;
            from = i;
            continue;
        }
        i += 1;
    }
    parts.push(text.get(from..).unwrap_or(""));
    parts
}

/// A stage's options: `name(arg)` and bare words.
pub(super) fn read_options(text: &str) -> Vec<(String, Option<String>)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        let start = i;
        while chars
            .get(i)
            .is_some_and(|c| !c.is_whitespace() && *c != '(')
        {
            i += 1;
        }
        let name: String = chars.get(start..i).unwrap_or(&[]).iter().collect();
        if name.is_empty() {
            break;
        }
        if chars.get(i) == Some(&'(') {
            let mut depth = 0;
            let from = i + 1;
            while let Some(c) = chars.get(i) {
                if *c == '(' {
                    depth += 1;
                } else if *c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                i += 1;
            }
            let arg: String = chars.get(from..i).unwrap_or(&[]).iter().collect();
            i += 1;
            out.push((name, Some(arg.trim().to_string())));
        } else {
            out.push((name, None));
        }
    }
    out
}

/// Reads what follows `svyset`.
pub(super) fn parse_svyset(text: &str) -> Result<Svyset, Untranslated> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Svyset::Show);
    }
    if text == ", clear" || text == ",clear" || text == ", cle" || text == ", clea" {
        return Ok(Svyset::Clear);
    }
    let mut stages_text = split_top(text, "||");
    // a trailing || with nothing after it adds no stage
    if stages_text.len() > 1 && stages_text.last().is_some_and(|t| t.trim().is_empty()) {
        stages_text.pop();
    }
    if stages_text.len() > MAX_STAGES {
        return Err(no(
            "svyset with more than five stages is not translated yet",
        ));
    }
    let mut stages: Vec<SvyStage> = Vec::new();
    let mut weight: Option<(&'static str, String)> = None;
    let mut singleunit: Option<String> = None;
    for (k, st) in stages_text.iter().enumerate() {
        let parts = split_top(st, ",");
        let head = parts.first().copied().unwrap_or("");
        let opts = if parts.len() > 1 {
            st.get(head.len() + 1..).unwrap_or("")
        } else {
            ""
        };
        let (head, w) = take_weight(head);
        if let Some(w) = w {
            if k > 0 {
                return Err(no(
                    "svyset with a weight after the first stage is not translated",
                ));
            }
            let exp = w.exp.trim().to_string();
            if !is_name(&exp) {
                return Err(no(
                    "svyset with a weight that is not a variable name is not translated yet",
                ));
            }
            weight = Some((
                match w.kind {
                    WeightKind::Sampling => "pweight",
                    WeightKind::Importance => "iweight",
                    _ => return Err(no(
                        "svyset with weights other than pweight and iweight is not translated yet",
                    )),
                },
                exp,
            ));
        }
        let words: Vec<&str> = head.split_whitespace().collect();
        if words.len() > 1 {
            return Err(no(
                "svyset with more than one sampling unit variable in a stage is not translated",
            ));
        }
        let su = match words.first() {
            None => {
                if k > 0 {
                    return Err(no(
                        "svyset with a later stage that names no variable is not translated",
                    ));
                }
                None
            }
            Some(&"_n") => None,
            Some(w) if is_name(w) => Some((*w).to_string()),
            Some(w) => {
                return Err(no(format!(
                    "svyset with the sampling unit {w}: only a variable name or _n is translated"
                )))
            }
        };
        let mut stage = SvyStage {
            strata: None,
            su,
            fpc: None,
        };
        let mut order = 0;
        for (name, arg) in read_options(opts) {
            let arg = arg.unwrap_or_default();
            match name.as_str() {
                "strata" | "fpc" => {
                    if !is_name(&arg) {
                        return Err(no(format!(
                            "svyset, {name}({arg}): only a variable name is translated"
                        )));
                    }
                    let (slot, rank) = if name == "strata" {
                        (&mut stage.strata, 1)
                    } else {
                        (&mut stage.fpc, 2)
                    };
                    if rank < order || slot.is_some() {
                        return Err(no(format!(
                            "svyset with {name}() written in an order or twice that is not translated"
                        )));
                    }
                    order = rank;
                    *slot = Some(arg);
                }
                "singleunit" => {
                    if !["missing", "certainty", "scaled", "centered"].contains(&arg.as_str()) {
                        return Err(no(format!(
                            "svyset, singleunit({arg}): only missing, certainty, scaled and centered written out are translated"
                        )));
                    }
                    singleunit = Some(arg);
                }
                "vce" if arg == "linearized" => {}
                other => {
                    return Err(no(format!(
                        "svyset with the option {other} is not translated yet"
                    )))
                }
            }
        }
        stages.push(stage);
    }
    // a stage with no fpc is sampled with replacement: the stages after it are dropped
    let mut dropped = None;
    for k in 0..stages.len().saturating_sub(1) {
        if stages.get(k).is_some_and(|s| s.fpc.is_none()) {
            dropped = Some(k + 1);
            stages.truncate(k + 1);
            break;
        }
    }
    Ok(Svyset::Set(SvyDesign {
        stages,
        weight,
        singleunit: singleunit.unwrap_or_else(|| "missing".into()),
        dropped,
    }))
}

impl Session {
    /// `svyset`, in all the forms above.
    pub(super) fn svyset(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let text = returned::after_name(c);
        let parsed = parse_svyset(&text)?;
        let quiet = self.quiet(c);
        match parsed {
            Svyset::Clear => {
                self.svy_design = None;
                out.line("stata_r <- list(settings = \", clear\")");
                self.r_new = Some((R_SVYSET, "svyset".into()));
            }
            Svyset::Show => match self.svy_design.clone() {
                None => {
                    if !quiet {
                        out.line("cat(\"no survey characteristics are set\\n\")");
                    }
                }
                Some(d) => {
                    if !quiet {
                        out.line(format!("cat({})", r_text(&format!("\n{}", d.lines()))));
                    }
                    out.line(format!("stata_r <- {}", d.r_list()));
                    self.r_new = Some((R_SVYSET, "svyset".into()));
                }
            },
            Svyset::Set(d) => {
                let data = self.data()?;
                for v in d.variables() {
                    out.line(format!(
                        "if (!{0} %in% names({data})) stop(\"variable {1} not found (Stata: r(111))\", call. = FALSE)",
                        r_text(&v),
                        v
                    ));
                }
                if !quiet {
                    if let Some(k) = d.dropped {
                        // the note wraps at the line size, the rest indented
                        out.line(format!(
                            "cat(paste(strwrap({}, width = (if (exists(\"stata_linesize\")) stata_linesize else 80) + 1, exdent = 6), collapse = \"\\n\"), \"\\n\", sep = \"\")",
                            r_text(&format!("note: stage {k} is sampled with replacement; further stages will be ignored for variance estimation."))
                        ));
                    }
                    out.line(format!("cat({})", r_text(&format!("\n{}", d.lines()))));
                }
                out.line(format!("stata_r <- {}", d.r_list()));
                self.r_new = Some((R_SVYSET, "svyset".into()));
                self.svy_design = Some(d);
            }
        }
        out.caveat(
            "the design is kept for svy: and svydescribe; the variables are read when they run",
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(text: &str) -> SvyDesign {
        match parse_svyset(text) {
            Ok(Svyset::Set(d)) => d,
            _ => panic!("{text} should set a design"),
        }
    }

    #[test]
    fn the_settings_are_written_as_stata_keeps_them() {
        let d = set(" psu [pw=w], strata(s) fpc(f) || ssu, fpc(g) || _n");
        assert_eq!(d.stages.len(), 3);
        assert_eq!(
            d.settings(),
            "psu [pweight= w], strata(s) fpc(f) || ssu, fpc(g) || _n, singleunit(missing) vce(linearized)"
        );
        assert_eq!(
            set(" psu").settings(),
            "psu, singleunit(missing) vce(linearized)"
        );
        assert_eq!(
            set(" [iweight=w], strata(s) singleunit(centered)").settings(),
            "_n [iweight= w], strata(s) singleunit(centered) vce(linearized)"
        );
    }

    #[test]
    fn a_stage_without_fpc_drops_the_stages_after_it() {
        let d = set(" psu [pw=w], strata(s) || ssu, fpc(g)");
        assert_eq!(d.stages.len(), 1);
        assert_eq!(d.dropped, Some(1));
        let d = set(" psu [pw=w], fpc(f) || ssu || _n");
        assert_eq!(d.stages.len(), 2);
        assert_eq!(d.dropped, Some(2));
    }

    #[test]
    fn the_design_lines_align_their_labels() {
        let d = set(" psu [pw=w], strata(s)");
        assert!(d
            .lines()
            .starts_with("Sampling weights: w\n             VCE: linearized\n"));
        let d = set(" [iweight=w]");
        assert!(d
            .lines()
            .starts_with("Importance weights: w\n               VCE: linearized\n"));
    }

    #[test]
    fn what_is_not_translated_is_refused() {
        for t in [
            " psu [fw=w]",
            " psu [pw=w], vce(brr)",
            " psu [pw=w], bsrweight(b)",
            " psu [pw=w], strata(s) singleunit(cert)",
            " psu [pw=w*2]",
            " psu [pw=w], fpc(f) strata(s)",
            " a b [pw=w]",
        ] {
            assert!(parse_svyset(t).is_err(), "{t}");
        }
        assert!(matches!(parse_svyset(" , clear"), Ok(Svyset::Clear)));
        assert!(matches!(parse_svyset(""), Ok(Svyset::Show)));
    }
}
