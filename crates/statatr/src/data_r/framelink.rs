//! `frlink`, `frget` and `frval()` (P2C.12aq), as Stata 19 does them on
//! made-up data: `plan/stata/frlink.md`.
//!
//! A link variable holds, for each row, the row number of the linked frame
//! whose key values match (missing when none does). `frlink` sorts the linked
//! frame by its keys, so the numbers are positions in that order. Stata checks
//! at each use that the linked frame still looks as it did; RCraft follows
//! the linked frame through the do-file instead, and refuses `frget` and
//! `frval()` once a line may have changed it (`Session::links_after`).

use super::*;

/// A link variable made by `frlink`: the frame it points to and that frame's key
/// variables.
#[derive(Clone)]
pub(super) struct Link {
    pub(super) target: String,
    pub(super) there: Vec<String>,
}

/// `frlink`: the link numbers for the rows of `here`, the linked frame sorted by
/// its keys, and Stata's note on how many rows matched. Stops (r(459)) when a
/// key does not identify the rows of the frame that must be unique.
pub(super) const FRLINK_HELPER: &str = r#"stata_frlink <- function(here, there, kh, kt, one, frame, quiet) {
  key <- function(d, k) {
    l <- lapply(k, function(v) {
      x <- d[[v]]
      if (is.character(x)) x else as.numeric(x)
    })
    names(l) <- paste0("k", seq_along(l))
    vctrs::new_data_frame(l, n = nrow(d))
  }
  hk <- key(here, kh)
  if (one && vctrs::vec_duplicate_any(hk)) stop(paste0("the match variables do not identify the rows of frame ", frame, " one to one: use m:1 (Stata: r(459))"), call. = FALSE)
  there <- dplyr::arrange(there, dplyr::pick(dplyr::all_of(kt)))
  tk <- key(there, kt)
  if (vctrs::vec_duplicate_any(tk)) stop("the match variables do not identify the rows of the linked frame (Stata: r(459))", call. = FALSE)
  link <- as.numeric(vctrs::vec_match(hk, tk))
  n <- sum(is.na(link))
  if (!quiet) {
    if (n == 0) cat("(all observations in frame ", frame, " matched)\n", sep = "")
    else if (n == length(link)) cat("(no observations in ", frame, " matched!)\n", sep = "")
    else cat("(", n, " observation", if (n == 1) "" else "s", " in frame ", frame, " unmatched)\n", sep = "")
  }
  list(link = link, there = there)
}"#;

/// What `frget` copies for a variable: the linked frame's value for each row
/// (missing, or empty for text, where the link is missing), with its label and
/// format. A variable with value labels is not copied (Stata renames a label
/// set the frame already holds; not followed).
pub(super) const FRGET_HELPER: &str = r#"stata_frget_col <- function(col, link) {
  if (inherits(col, "haven_labelled")) stop("frget of a variable with value labels is not translated", call. = FALSE)
  out <- vctrs::vec_slice(col, link)
  if (is.character(out)) out[is.na(out)] <- ""
  out
}"#;

/// `frval(link, var [, default])`: the linked frame's value of `var` for each
/// row, the default (else missing, or empty for text) where the link is missing.
pub(super) const FRVAL_HELPER: &str = r#"stata_frval <- function(link, col, default = NULL) {
  out <- as.vector(unclass(col))[link]
  miss <- is.na(link)
  if (!is.null(default)) out[miss] <- default
  else if (is.character(out)) out[is.na(out)] <- ""
  out
}"#;

/// Commands that leave a frame's data as it was (and so the links to it).
const READ_ONLY: &[&str] = &[
    "summarize",
    "list",
    "describe",
    "display",
    "count",
    "tabulate",
    "codebook",
    "correlate",
    "ttest",
    "return",
    "ereturn",
    "estimates",
    "scalar",
    "local",
    "global",
    "matrix",
    "mata",
    "assert",
    "isid",
    "inspect",
];

/// Commands that rebuild the data of the frame they run in, and with it any
/// link variable it holds.
const REBUILDS: &[&str] = &[
    "clear", "use", "merge", "append", "reshape", "collapse", "contract", "xpose", "joinby",
    "expand", "stack", "rename", "import", "insheet", "infile", "input", "restore",
];

impl Session {
    /// `frlink 1:1 | m:1 varlist, frame(name [varlist]) [generate(newvar)]`.
    pub(super) fn frlink(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("frame", 5), ("generate", 8)]) {
            return Err(no(format!(
                "frlink with the option {o} is not translated yet"
            )));
        }
        // a failed frlink leaves the linked frame sorted by a sort that puts
        // tied rows in no set order: under capture the line is not translated
        if c.prefixes
            .iter()
            .any(|p| !matches!(p, Prefix::Quietly | Prefix::Noisily))
        {
            return Err(no(
                "frlink under capture, by or another prefix is not translated yet: when the keys do not identify rows Stata leaves the linked frame sorted, tied rows in no set order",
            ));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("frlink with if or in is not translated yet"));
        }
        if self.tags.get() {
            return Err(no(
                "frlink with extended missing values among the keys is not translated yet",
            ));
        }
        let mut words = c.parts.main.split_whitespace();
        let one = match words.next() {
            Some("1:1") => true,
            Some("m:1") => false,
            Some(w) => {
                return Err(no(format!(
                    "frlink {w} is not translated (1:1 and m:1 are)"
                )))
            }
            None => return Err(no("frlink needs 1:1 or m:1")),
        };
        let here: Vec<String> = words.map(str::to_string).collect();
        if here.is_empty() || here.iter().any(|v| !is_name(v)) {
            return Err(no(
                "frlink: the match variables are not plain variable names",
            ));
        }
        let Some(frame_arg) = opts.arg("frame", 5) else {
            return Err(no("frlink needs frame(name)"));
        };
        let mut fw = frame_arg.split_whitespace();
        let target = fw.next().unwrap_or("").to_string();
        if !is_name(&target) {
            return Err(no(format!("frlink: {target} is not a frame name")));
        }
        let mut there: Vec<String> = fw.map(str::to_string).collect();
        if there.is_empty() {
            there = here.clone();
        }
        if there.len() != here.len() || there.iter().any(|v| !is_name(v)) {
            return Err(no(
                "frlink: the linked frame's match variables are not as many plain names as the current frame's",
            ));
        }
        let holder = self.current_frame();
        if target == holder {
            return Err(no("frlink to the frame in use is not translated yet"));
        }
        let link = match opts.arg("generate", 8) {
            Some(g) => g.trim().to_string(),
            None => target.clone(),
        };
        if !is_name(&link) {
            return Err(no(format!("frlink: {link} is not a variable name")));
        }
        if !self.frames.contains_key(&target) {
            out.line(stop_text(&format!(
                "frame {target} not found (Stata: r(111))"
            )));
            return Ok(());
        }
        let d = self.data()?;
        let Some(t) = self.frame_data_expr(&target) else {
            return Err(no(format!(
                "frlink: the linked frame {target} holds no data known here"
            )));
        };
        let there_kinds = self
            .frame_state(&target)
            .map(|p| p.kinds)
            .unwrap_or_default();
        for (h, k) in here.iter().zip(&there) {
            match (self.kind_of(h), there_kinds.get(k).copied()) {
                (Some(a), Some(b)) if a == b && matches!(a, Kind::Number | Kind::Text) => {}
                (Some(_), Some(_)) => {
                    return Err(no(format!(
                        "frlink: {h} and {k} are not both numbers or both text"
                    )))
                }
                _ => {
                    return Err(no(format!(
                        "frlink: {h} or {k} is not a variable known here (names written out are translated)"
                    )))
                }
            }
        }
        self.helpers.insert("stata_frlink");
        out.package("dplyr");
        out.package("vctrs");
        out.line(format!(
            "if ({} %in% names({d})) stop(\"variable {link} already defined (Stata: r(110))\", call. = FALSE)",
            r_text(&link)
        ));
        let names = |v: &[String]| {
            format!(
                "c({})",
                v.iter().map(|n| r_text(n)).collect::<Vec<_>>().join(", ")
            )
        };
        out.line(format!(
            "stata_fl <- stata_frlink({d}, {t}, {}, {}, {}, {}, {})",
            names(&here),
            names(&there),
            if one { "TRUE" } else { "FALSE" },
            r_text(&holder),
            if self.quiet(c) { "TRUE" } else { "FALSE" }
        ));
        out.line(format!("{t} <- stata_fl$there"));
        out.line(format!("{d}[[{}]] <- stata_fl$link", r_text(&link)));
        out.line("rm(stata_fl)");
        self.kinds.insert(link.clone(), Kind::Number);
        self.storage.remove(&link);
        // the linked frame is now sorted by these keys: links built on other keys no longer hold
        self.links.retain(|(h, v), l| {
            !(l.target == target && l.there != there) && !(h == &holder && v == &link)
        });
        self.links.insert((holder, link), Link { target, there });
        Ok(())
    }

    /// `frget varlist | newvar = var ..., from(linkvar) [prefix() | suffix()]`.
    pub(super) fn frget(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("from", 4), ("prefix", 6), ("suffix", 6)]) {
            return Err(no(format!(
                "frget with the option {o} is not translated yet"
            )));
        }
        if c.prefixes
            .iter()
            .any(|p| !matches!(p, Prefix::Quietly | Prefix::Noisily))
        {
            return Err(no(
                "frget under capture, by or another prefix is not translated yet",
            ));
        }
        let Some(from) = opts.arg("from", 4).map(|s| s.trim().to_string()) else {
            return Err(no("frget needs from(linkvar)"));
        };
        // Stata allows neither an if nor an in here (r(101))
        if c.parts.if_exp.is_some() {
            out.line(stop_text("if not allowed (Stata: r(101))"));
            return Ok(());
        }
        if c.parts.in_range.is_some() {
            out.line(stop_text("in range not allowed (Stata: r(101))"));
            return Ok(());
        }
        let prefix = opts.arg("prefix", 6).unwrap_or_default();
        let suffix = opts.arg("suffix", 6).unwrap_or_default();
        if !prefix.is_empty() && !suffix.is_empty() {
            return Err(no(
                "frget with both prefix() and suffix() is not translated yet",
            ));
        }
        // the words: `v` or `new = v`
        let text = c.parts.main.replace('=', " = ");
        let toks: Vec<&str> = text.split_whitespace().collect();
        if toks.is_empty() {
            return Err(no("frget needs variables to copy"));
        }
        // (the name in the linked frame, the name it gets here, whether renamed)
        let mut wanted: Vec<(String, String, bool)> = Vec::new();
        let mut i = 0;
        while let Some(&tok) = toks.get(i) {
            if toks.get(i + 1) == Some(&"=") {
                let Some(&old) = toks.get(i + 2) else {
                    return Err(no("frget: a name is missing after ="));
                };
                if !prefix.is_empty() || !suffix.is_empty() {
                    return Err(no(
                        "frget newvar = var with prefix() or suffix() is not translated yet",
                    ));
                }
                if !is_name(tok) || !is_name(old) {
                    return Err(no("frget: only plain variable names are translated"));
                }
                wanted.push((old.to_string(), tok.to_string(), true));
                i += 3;
            } else {
                if !is_name(tok) {
                    return Err(no(format!(
                        "frget {tok}: only plain variable names are translated (_all and patterns are not)"
                    )));
                }
                wanted.push((tok.to_string(), format!("{prefix}{tok}{suffix}"), false));
                i += 1;
            }
        }
        let holder = self.current_frame();
        let Some(link) = self.links.get(&(holder.clone(), from.clone())).cloned() else {
            return Err(no(format!(
                "frget from({from}): {from} is not a link variable made by frlink in this do-file, or a line since may have changed the linked frame (Stata checks and may stop with r(459))"
            )));
        };
        let Some(state) = self.frame_state(&link.target) else {
            return Err(no(format!(
                "frget: frame {} is not known here",
                link.target
            )));
        };
        let Some(t) = self.frame_data_expr(&link.target) else {
            return Err(no(format!(
                "frget: the linked frame {} holds no data known here",
                link.target
            )));
        };
        if !state.label_sets.is_empty() {
            return Err(no(
                "frget from a frame with value labels is not translated yet: Stata renames a label set the frame in use already holds",
            ));
        }
        let d = self.data()?;
        // a variable named twice is copied once; a match variable is not copied under its own name
        let mut copy: Vec<(String, String)> = Vec::new();
        let mut skipped: Vec<String> = Vec::new();
        for (old, new, renamed) in &wanted {
            let Some(kind) = state.kinds.get(old).copied() else {
                return Err(no(format!(
                    "frget: {old} is not a variable of frame {} known here",
                    link.target
                )));
            };
            if !matches!(kind, Kind::Number | Kind::Text) {
                return Err(no(format!("frget: {old} is not a number or text")));
            }
            if !renamed && link.there.contains(old) {
                if !skipped.contains(old) {
                    skipped.push(old.clone());
                }
                continue;
            }
            if copy.iter().any(|(_, n)| n == new) {
                if copy.iter().any(|(o, n)| n == new && o == old) {
                    continue;
                }
                return Err(no(format!("frget: two variables would be named {new}")));
            }
            copy.push((old.clone(), new.clone()));
        }
        let quiet = self.quiet(c);
        self.helpers.insert("stata_frget_col");
        out.package("vctrs");
        for (_, new) in &copy {
            out.line(format!(
                "if ({} %in% names({d})) stop(\"variable {new} already defined (Stata: r(110))\", call. = FALSE)",
                r_text(new)
            ));
        }
        if !quiet {
            for v in &skipped {
                out.line(format!(
                    "cat({})",
                    r_text(&format!("(variable not copied from linked frame: {v})\n"))
                ));
            }
        }
        let lv = r_text(&from);
        for (old, new) in &copy {
            let col = r_text(new);
            out.line(format!(
                "{d}[[{col}]] <- stata_frget_col({t}[[{}]], {d}[[{lv}]])",
                r_text(old)
            ));
            if !quiet {
                self.helpers.insert("stata_chg");
                self.helpers.insert("stata_mv");
                out.line(format!("stata_gen_note({d}[[{col}]])"));
            }
        }
        if !quiet {
            let n = copy.len();
            out.line(format!(
                "cat({})",
                r_text(&format!(
                    "({n} variable{} copied from linked frame)\n",
                    if n == 1 { "" } else { "s" }
                ))
            ));
        }
        for (old, new) in &copy {
            if let Some(k) = state.kinds.get(old) {
                self.kinds.insert(new.clone(), *k);
            }
            match state.storage.get(old) {
                Some(s) => {
                    self.storage.insert(new.clone(), *s);
                }
                None => {
                    self.storage.remove(new);
                }
            }
        }
        Ok(())
    }

    /// `frval(link, var [, default])` in an expression: the R that looks the
    /// value up through the link variable, and the kind of `var`.
    pub(super) fn frval_value(&self, name: &str) -> Result<(String, Kind), Untranslated> {
        let mut parts = name.splitn(3, '\u{1}');
        let (l, v) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        let default = parts.next();
        let holder = self.current_frame();
        let Some(link) = self.links.get(&(holder, l.to_string())) else {
            return Err(no(format!(
                "frval({l}, {v}): {l} is not a link variable made by frlink in this do-file, or a line since may have changed the linked frame (Stata checks and may stop)"
            )));
        };
        let Some(state) = self.frame_state(&link.target) else {
            return Err(no(format!(
                "frval: frame {} is not known here",
                link.target
            )));
        };
        let Some(kind) = state.kinds.get(v).copied() else {
            return Err(no(format!(
                "frval: {v} is not a variable of frame {} known here",
                link.target
            )));
        };
        if !matches!(kind, Kind::Number | Kind::Text) {
            return Err(no(format!("frval: {v} is not a number or text")));
        }
        let Some(t) = self.frame_data_expr(&link.target) else {
            return Err(no(format!(
                "frval: the linked frame {} holds no data known here",
                link.target
            )));
        };
        let mut code = format!("stata_frval({}, {t}[[{}]]", r_name(l), r_text(v));
        if let Some(def) = default {
            code.push_str(&format!(", {def}"));
        }
        code.push(')');
        Ok((code, kind))
    }

    /// What a line did to the links: the linked frame changed, the variable that
    /// holds a link went, or the frames themselves were rearranged.
    pub(super) fn links_after(&mut self, c: &Command, ok: bool) {
        if self.links.is_empty() || matches!(c.name.as_str(), "frlink" | "frget") {
            return;
        }
        let name = c.name.as_str();
        if matches!(name, "frame" | "frames" | "cwf" | "pwf") {
            let sub = c.parts.main.split_whitespace().next().unwrap_or("");
            let keeps = match name {
                "cwf" | "pwf" => true,
                "frame" => matches!(sub, "change" | "pwf" | "dir" | "create" | "put"),
                _ => sub == "dir",
            };
            if !keeps || !ok {
                self.links.clear();
            }
            return;
        }
        let cur = self.current_frame();
        if !ok {
            self.links.retain(|(h, _), l| h != &cur && l.target != cur);
            return;
        }
        let reads = READ_ONLY.contains(&name) || MODELS.contains(&name);
        if !reads {
            self.links.retain(|_, l| l.target != cur);
        }
        if REBUILDS.contains(&name) {
            self.links.retain(|(h, _), _| h != &cur);
        }
        let kinds = &self.kinds;
        self.links
            .retain(|(h, v), _| h != &cur || kinds.contains_key(v));
    }
}

fn stop_text(msg: &str) -> String {
    format!("stop({}, call. = FALSE)", r_text(msg))
}
