//! More of `frame`, as Stata 19 does it on made-up data (P2C.12ao,
//! `plan/stata/frames-more.md`): `frame dir`, `frame copy`, `frame rename`,
//! `frame put`, `cwf`, `frames reset`, and a command run in another frame
//! (`frame name: command`, `frame name { ... }`).
//!
//! `frame create`, `change`, `drop` and `pwf` are in `data_r.rs`.

use super::*;

/// `frame dir`: a line for each frame in the order of their names, a star
/// before one that holds unsaved data, then `rows x columns`, then the file
/// its data came from or went to (`; f1.dta`); a note when any has a star.
pub(super) const FRAME_DIR_HELPER: &str = r#"stata_frame_dir <- function(name, star, file, n, k) {
  o <- order(name, method = "radix")
  name <- name[o]
  star <- star[o]
  file <- file[o]
  n <- n[o]
  k <- k[o]
  cat(paste0(star, " ", formatC(name, width = -max(nchar(name))), "  ", format(n, scientific = FALSE, trim = TRUE), " x ", format(k, scientific = FALSE, trim = TRUE), file, "\n"), sep = "")
  if (any(star == "*")) cat("\nNote: Frames marked with * contain unsaved data.\n")
  invisible(NULL)
}"#;

/// A file's name without its folders.
fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn stop_text(msg: &str) -> String {
    format!("stop({}, call. = FALSE)", r_text(msg))
}

impl Session {
    /// The current frame's name.
    fn current_frame(&self) -> String {
        self.frame.clone().unwrap_or_else(|| "default".into())
    }

    /// The R that reads a frame's data frame: the one in memory for the
    /// current frame, else the one kept in `stata_frames`.
    fn frame_data_expr(&self, name: &str) -> Option<String> {
        if name == self.current_frame() {
            self.data.clone()
        } else {
            self.frames
                .get(name)
                .and_then(|p| p.data.as_ref())
                .map(|_| format!("stata_frames[[{}]]", r_text(name)))
        }
    }

    /// The translator's view of a frame (the current one's is made now).
    fn frame_state(&self, name: &str) -> Option<Preserved> {
        if name == self.current_frame() {
            Some(self.data_state())
        } else {
            self.frames.get(name).cloned()
        }
    }

    /// `frame dir`.
    pub(super) fn frame_dir(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let mut rows: Vec<(String, Preserved, bool)> = Vec::new();
        let current = self.current_frame();
        rows.push((current.clone(), self.data_state(), true));
        for (n, p) in &self.frames {
            rows.push((n.clone(), p.clone(), false));
        }
        let (mut names, mut stars, mut files, mut ns, mut ks) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (n, p, is_current) in rows {
            let star = match (&p.data, p.saved_now) {
                (None, _) => " ",
                (Some(_), Some(true)) => " ",
                (Some(_), Some(false)) => "*",
                (Some(_), None) => {
                    return Err(no(format!(
                        "frame dir marks the frames that hold unsaved data, and whether frame {n} does is not known here: a line before it changed that frame's data in a way RCraft does not follow"
                    )))
                }
            };
            names.push(r_text(&n));
            stars.push(r_text(star));
            files.push(match (&p.data, &p.file) {
                (Some(_), Some(f)) => r_text(&format!("; {}", base_name(f))),
                _ => r_text(""),
            });
            let (rows_n, cols_n) = match &p.data {
                None => ("0".to_string(), "0".to_string()),
                Some(d) => {
                    let obj = if is_current {
                        d.clone()
                    } else {
                        format!("stata_frames[[{}]]", r_text(&n))
                    };
                    (format!("nrow({obj})"), format!("ncol({obj})"))
                }
            };
            ns.push(rows_n);
            ks.push(cols_n);
        }
        self.helpers.insert("stata_frame_dir");
        if self.quiet(c) {
            out.line("invisible(NULL)");
        } else {
            out.line(format!(
                "stata_frame_dir(c({}), c({}), c({}), c({}), c({}))",
                names.join(", "),
                stars.join(", "),
                files.join(", "),
                ns.join(", "),
                ks.join(", ")
            ));
        }
        Ok(())
    }

    /// `frame copy from to [, replace]`: the frame's data as it is, with the
    /// file it came from and whether it was saved (Stata 19 keeps both).
    pub(super) fn frame_copy(
        &mut self,
        opts: &Opts,
        words: &[&str],
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("replace", 7)]) {
            return Err(no(format!("frame copy with the option {o} is not translated yet")));
        }
        let [from, to] = words else {
            return Err(no("frame copy takes the frame to copy and the name of the copy"));
        };
        let (from, to) = (from.to_string(), to.to_string());
        for n in [&from, &to] {
            if !is_name(n) {
                return Err(no(format!("frame copy: {n} is not a frame name")));
            }
        }
        let replace = opts.has("replace", 7);
        let current = self.current_frame();
        let exists = |s: &Self, n: &str| n == current || s.frames.contains_key(n);
        if !exists(self, &from) {
            out.line(stop_text(&format!("frame {from} not found (Stata: r(111))")));
            return Ok(());
        }
        if exists(self, &to) && !replace {
            out.line(stop_text(&format!("frame {to} already defined (Stata: r(110))")));
            return Ok(());
        }
        if from == to {
            return Err(no("frame copy of a frame onto itself is not translated"));
        }
        if self.preserved.is_some() || !self.frame_preserved.is_empty() {
            return Err(no(
                "frame copy while a preserve is pending is not translated yet",
            ));
        }
        let Some(state) = self.frame_state(&from) else {
            return Err(no(format!("frame {from} is not known here")));
        };
        let source = self.frame_data_expr(&from);
        if to == current {
            // the data in memory become the copy
            let old = self.data.clone();
            match (&source, &state.data) {
                (Some(src), Some(new)) => {
                    let mut lines = Vec::new();
                    if let Some(o) = &old {
                        if o != new {
                            lines.push(format!("rm({o})"));
                        }
                    }
                    lines.push(format!("{new} <- {src}"));
                    out.line(lines.join("\n"));
                }
                _ => match &old {
                    Some(o) => out.line(format!("rm({o})")),
                    None => out.line("invisible(NULL)"),
                },
            }
            self.set_data_state(state);
            self.esample = false;
            self.changed_since_fit = true;
        } else {
            match &source {
                Some(src) => {
                    out.line(format!(
                        "if (!exists(\"stata_frames\")) stata_frames <- list()\nstata_frames[[{}]] <- {src}",
                        r_text(&to)
                    ));
                }
                None => {
                    out.line(format!(
                        "if (exists(\"stata_frames\")) stata_frames[[{}]] <- NULL",
                        r_text(&to)
                    ));
                }
            }
            self.frames.insert(to, state);
        }
        Ok(())
    }

    /// `frame rename old new`.
    pub(super) fn frame_rename(
        &mut self,
        words: &[&str],
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let [old, new] = words else {
            return Err(no("frame rename takes the old name and the new one"));
        };
        let (old, new) = (old.to_string(), new.to_string());
        for n in [&old, &new] {
            if !is_name(n) {
                return Err(no(format!("frame rename: {n} is not a frame name")));
            }
        }
        let current = self.current_frame();
        let exists = |s: &Self, n: &str| n == current || s.frames.contains_key(n);
        if !exists(self, &old) {
            out.line(stop_text(&format!("frame {old} not found (Stata: r(111))")));
            return Ok(());
        }
        if exists(self, &new) {
            out.line(stop_text(&format!("frame {new} already defined (Stata: r(110))")));
            return Ok(());
        }
        if old == "default" {
            return Err(no("frame rename of the default frame is not translated yet"));
        }
        if old == current {
            self.frame = Some(new);
            out.line("invisible(NULL)");
            return Ok(());
        }
        let mut code = Vec::new();
        if let Some(p) = self.frames.remove(&old) {
            if p.data.is_some() {
                code.push(format!(
                    "stata_frames[[{}]] <- stata_frames[[{}]]\nstata_frames[[{}]] <- NULL",
                    r_text(&new),
                    r_text(&old),
                    r_text(&old)
                ));
            }
            self.frames.insert(new.clone(), p);
        }
        if let Some(p) = self.frame_preserved.remove(&old) {
            code.push(format!(
                "stata_frames_preserved[[{}]] <- stata_frames_preserved[[{}]]\nstata_frames_preserved[[{}]] <- NULL",
                r_text(&new),
                r_text(&old),
                r_text(&old)
            ));
            self.frame_preserved.insert(new, p);
        }
        out.line(if code.is_empty() {
            "invisible(NULL)".to_string()
        } else {
            code.join("\n")
        });
        Ok(())
    }

    /// `frame put [varlist] [if] [in], into(name)`: the selected variables of
    /// the selected rows, in a new frame.
    pub(super) fn frame_put(
        &mut self,
        c: &Command,
        opts: &Opts,
        rest: &str,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("into", 4)]) {
            return Err(no(format!("frame put with the option {o} is not translated yet")));
        }
        let Some(into) = opts.arg("into", 4).map(|s| s.trim().to_string()) else {
            return Err(no("frame put needs into(newname)"));
        };
        if !is_name(&into) {
            return Err(no(format!("frame put: {into} is not a frame name")));
        }
        let d = self.data()?;
        let current = self.current_frame();
        if into == current || self.frames.contains_key(&into) {
            out.line(stop_text(&format!("frame {into} already defined (Stata: r(110))")));
            return Ok(());
        }
        let vars: Vec<&str> = rest.split_whitespace().collect();
        let sel = if vars.is_empty() {
            None
        } else {
            for v in &vars {
                if v.contains(['*', '?', '~', '-']) || self.kind_of(v).is_none() {
                    return Err(no(format!(
                        "frame put: {v} is not a variable name known here (names written out are translated)"
                    )));
                }
            }
            Some(select(rest)?)
        };
        let cond = self.condition(c, out)?;
        let mut steps = vec![d.clone()];
        if let Some(k) = &cond {
            steps.push(format!("dplyr::filter({k})"));
        }
        if let Some(s) = &sel {
            steps.push(format!("dplyr::select({s})"));
        }
        out.package("dplyr");
        out.line(format!(
            "if (!exists(\"stata_frames\")) stata_frames <- list()\nstata_frames[[{}]] <- {}",
            r_text(&into),
            steps.join(" |> ")
        ));
        let mut state = self.data_state();
        state.data = Some("data".into());
        state.saved_now = Some(false);
        state.file = None;
        state.panel = None;
        state.panel_time = None;
        state.ts_delta = None;
        state.stset = None;
        if !vars.is_empty() {
            state.kinds.retain(|k, _| vars.contains(&k.as_str()));
            state.storage.retain(|k, _| vars.contains(&k.as_str()));
        }
        self.frames.insert(into, state);
        Ok(())
    }

    /// `frames reset` and `clear all` with frames: every frame but `default`
    /// goes, and `default` is empty.
    pub(super) fn frames_reset(&mut self, out: &mut Out) -> Result<(), Untranslated> {
        if self.preserved.is_some() || !self.frame_preserved.is_empty() {
            return Err(no(
                "frames reset while a preserve is pending is not translated yet",
            ));
        }
        let mut code = vec!["if (exists(\"stata_frames\")) rm(stata_frames)".to_string()];
        if let Some(d) = self.data.take() {
            code.push(format!("rm({d})"));
        }
        out.line(code.join("\n"));
        self.frames.clear();
        self.frame = None;
        self.chars_lost = None;
        self.data_lost = None;
        self.kinds.clear();
        self.storage.clear();
        self.fv_base.clear();
        self.forget_ts();
        self.label_sets.clear();
        self.file = None;
        self.saved_now = Some(true);
        Ok(())
    }

    /// A command that runs in another frame (`frame name: command`, or a
    /// line in a `frame name { }` block): the frame is changed to, the
    /// command runs, and the frame in use is changed back.
    pub(super) fn in_frame(&mut self, c: &Command, name: &str) -> Result<RCode, Untranslated> {
        // what a command in another frame could do to the frames themselves
        if matches!(
            c.name.as_str(),
            "frame" | "frames" | "cwf" | "pwf" | "clear" | "preserve" | "restore" | "capture"
        ) {
            self.rows_unknown();
            return Err(no(format!(
                "{} in a frame prefix is not translated yet",
                c.name
            )));
        }
        let mut inner = c.clone();
        inner
            .prefixes
            .retain(|p| !matches!(p, Prefix::Frame(f) if f == name));
        let current = self.current_frame();
        let switch = |to: &str| -> Command {
            let mut s = c.clone();
            s.prefixes = Vec::new();
            s.name = "frame".into();
            s.written = "frame".into();
            s.text = format!("frame change {to}");
            s.parts = Default::default();
            s.parts.main = format!("change {to}");
            s.options = Vec::new();
            s.macros = Vec::new();
            s.using = None;
            s
        };
        // the frame is not there: Stata stops (r(111)) where the command would run
        let missing = name != current && !self.frames.contains_key(name);
        if missing {
            let mut fail = switch(name);
            fail.prefixes = inner.prefixes.clone();
            return self.command_inner(&fail);
        }
        if name == current {
            return self.command_inner(&inner);
        }
        let into = self.command_inner(&switch(name))?;
        let ran = self.command_inner(&inner);
        // back to the frame in use, whether or not the command was translated
        let back = self.command_inner(&switch(&current));
        let (ran, back) = (ran?, back?);
        let mut code = Vec::new();
        for part in [&into.code, &ran.code, &back.code] {
            if !part.is_empty() {
                code.push(part.clone());
            }
        }
        let mut caveats = into.caveats;
        for cv in ran.caveats.into_iter().chain(back.caveats) {
            if !caveats.contains(&cv) {
                caveats.push(cv);
            }
        }
        let mut packages = into.packages;
        for p in ran.packages.into_iter().chain(back.packages) {
            if !packages.contains(&p) {
                packages.push(p);
            }
        }
        Ok(RCode {
            code: code.join("\n"),
            caveats,
            packages,
        })
    }
}

impl Session {
    /// `frames reset`: every frame but `default` is dropped and `default` is
    /// emptied, from whichever frame it is run in (Stata 19).
    pub(super) fn frames_cmd(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!("frames with the option {o} is not translated yet")));
        }
        match c.parts.main.trim() {
            "reset" => self.frames_reset(out),
            other => Err(no(format!("frames {other} is not translated yet"))),
        }
    }

    /// `c(frame)`: the name of the frame in use.
    pub(super) fn c_frame(&self) -> (String, Kind) {
        (r_text(&self.current_frame()), Kind::Text)
    }
}

impl Session {
    /// `frame create name varlist`: a frame with these variables and no rows
    /// (Stata 19 counts it as holding unsaved data).
    pub(super) fn frame_create_vars(
        &mut self,
        words: &[&str],
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let Some((name, vars)) = words.split_first() else {
            return Err(no("frame create needs a frame name"));
        };
        if !is_name(name) {
            return Err(no(format!("frame create: {name} is not a frame name")));
        }
        if *name == self.current_frame() || self.frames.contains_key(*name) {
            out.line(stop_text(&format!("frame {name} already defined (Stata: r(110))")));
            return Ok(());
        }
        let mut kinds = BTreeMap::new();
        let mut storage = BTreeMap::new();
        let mut cols = Vec::new();
        let mut after = Vec::new();
        let mut ty: Option<&str> = None;
        for w in vars {
            let is_type = matches!(*w, "byte" | "int" | "long" | "float" | "double")
                || w.strip_prefix("str")
                    .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
            if is_type {
                if ty.is_some() {
                    return Err(no("frame create: two types in a row"));
                }
                ty = Some(w);
                continue;
            }
            if !is_name(w) || cols.iter().any(|(n, _): &(String, bool)| n == w) {
                return Err(no(format!("frame create: {w} is not a new variable name")));
            }
            let t = ty.take().unwrap_or("float");
            let text = t.starts_with("str");
            cols.push((w.to_string(), text));
            kinds.insert(w.to_string(), if text { Kind::Text } else { Kind::Number });
            storage.insert(
                w.to_string(),
                match t {
                    "byte" => Storage::Byte,
                    "int" => Storage::Int,
                    "long" => Storage::Long,
                    "double" => Storage::Double,
                    t if text => {
                        if let Some(width) = t.strip_prefix("str").and_then(|n| n.parse::<usize>().ok()) {
                            after.push(format!(
                                "attr(d[[{}]], \"format.stata\") <- \"%{width}s\"",
                                r_text(w)
                            ));
                        }
                        Storage::Str
                    }
                    _ => Storage::Float,
                },
            );
        }
        if ty.is_some() {
            return Err(no("frame create: a type with no variable after it"));
        }
        let frame_cols: Vec<String> = cols
            .iter()
            .map(|(n, text)| {
                format!(
                    "{} = {}",
                    r_name(n),
                    if *text { "character(0)" } else { "numeric(0)" }
                )
            })
            .collect();
        out.line(format!(
            "if (!exists(\"stata_frames\")) stata_frames <- list()\nstata_frames[[{}]] <- local({{\nd <- data.frame({}, stringsAsFactors = FALSE)\n{}\nd\n}})",
            r_text(name),
            frame_cols.join(", "),
            after.join("\n")
        ));
        self.frames.insert(
            name.to_string(),
            Preserved {
                data: Some("data".into()),
                kinds,
                storage,
                saved_now: Some(false),
                ..Preserved::default()
            },
        );
        Ok(())
    }
}
