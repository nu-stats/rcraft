//! `fdasave` and `fdause`, SAS transport (XPORT version 5) files as Stata 19
//! writes and reads them (`plan/stata/fdasave.md`). The R is in `xport/`:
//! `xport.R` (IBM floats, 80-byte records, a reader), `fdasave.R`, `fdause.R`.

use super::*;

pub(super) const XPORT_HELPER: &str = include_str!("xport/xport.R");
pub(super) const FDASAVE_HELPER: &str = include_str!("xport/fdasave.R");
pub(super) const FDAUSE_HELPER: &str = include_str!("xport/fdause.R");

impl Session {
    /// `fdasave file [if] [in] [, rename replace vallabfile(xpf|sascode|both|none)]`.
    pub(super) fn fdasave(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        if let Some(o) = opts.other(&[("replace", 7), ("rename", 6), ("vallabfile", 6)]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        let kind = match opts.arg("vallabfile", 6).as_deref().map(str::trim) {
            None | Some("xpf") => "xpf",
            Some("sascode") => "sascode",
            Some("both") => "both",
            Some("none") => "none",
            Some(_) => {
                stop(out, "option vallabfile() invalid (Stata: r(198))");
                return Ok(());
            }
        };
        let files = crate::dofile::macros::tokens(&c.parts.main);
        let Some(first) = files.first() else {
            stop(out, "varlist required (Stata: r(100))");
            return Ok(());
        };
        if files.len() > 1 {
            return Err(no(
                "fdasave with more than one word before the options is not translated yet",
            ));
        }
        let path = file_name(first, "xpt");
        if self.data.is_none() && self.data_lost.is_none() {
            // no data in memory is no variables
            stop(out, "no variables defined (Stata: r(111))");
            return Ok(());
        }
        let d = self.data()?;
        let mut frame = d.clone();
        if let Some(cond) = self.condition(c, out)? {
            out.package("dplyr");
            frame = format!("dplyr::filter({d}, {cond})");
        }
        out.package("haven");
        self.helpers.insert("stata_xpt");
        self.helpers.insert("stata_fdasave");
        let flag = |b: bool| if b { "TRUE" } else { "FALSE" };
        out.line(format!(
            "stata_fdasave({frame}, {p}, {p}, rename = {}, replace = {}, vallabfile = {})",
            flag(opts.has("rename", 6)),
            flag(opts.has("replace", 7)),
            r_text(kind),
            p = r_text(&path),
        ));
        out.caveat("a string is laid out by its declared width in the file; RCraft takes the width the variable was given by input str# or generate str#, else the longest value");
        Ok(())
    }

    /// `fdause file [, clear novallabels]`.
    pub(super) fn fdause(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        if let Some(o) = opts.other(&[("clear", 5), ("novallabels", 6)]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        if !opts.has("clear", 5) {
            if self.data.is_some() {
                return Err(no(
                    "fdause with data in memory and no clear is not translated (Stata may refuse: no; dataset in memory has changed since last saved, r(4))",
                ));
            }
            if let Some(why) = &self.data_lost {
                return Err(no(format!(
                    "fdause without clear: whether data is in memory is not known here ({why})"
                )));
            }
        }
        let files = crate::dofile::macros::tokens(&c.parts.main);
        let Some(first) = files.first() else {
            stop(out, "invalid file specification (Stata: r(198))");
            return Ok(());
        };
        if let Some(extra) = files.get(1) {
            stop(
                out,
                &format!("invalid '{}' (Stata: r(198))", extra.trim_matches('"')),
            );
            return Ok(());
        }
        let path = file_name(first, "xpt");
        let name = frame_name(&path);
        out.package("haven");
        self.helpers.insert("stata_xpt");
        self.helpers.insert("stata_fdause");
        out.line(format!(
            "{name} <- stata_fdause({p}, {p}, novallabels = {})",
            if opts.has("novallabels", 6) {
                "TRUE"
            } else {
                "FALSE"
            },
            p = r_text(&path),
        ));
        self.data = Some(name);
        self.data_lost = None;
        self.file = None;
        self.kinds.clear();
        self.storage.clear();
        self.fv_base.clear();
        self.forget_ts();
        self.label_sets.clear();
        Ok(())
    }
}

pub(super) const LABEL_DROPPED_HELPER: &str = include_str!("xport/label_dropped.R");

impl Session {
    /// After `label drop`: the variables that used the labels keep the name
    /// and lose the entries, as Stata's do (`fdasave` then says the label is
    /// not found).
    pub(super) fn label_dropped(&mut self, names: &[String], out: &mut Out) {
        if let Some(d) = self.data.clone() {
            self.helpers.insert("stata_label_dropped");
            out.package("haven");
            out.line(format!(
                "{d} <- stata_label_dropped({d}, c({}))",
                names.join(", ")
            ));
        }
    }
}
