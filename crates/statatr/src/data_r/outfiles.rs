//! `mkdir`, `rmdir`, and the commands that write the data out as text
//! (`outsheet`, `outfile`), as Stata 19 does them (observed on made-up
//! data, `plan/stata/mkdir.md`, `outsheet.md`, `outfile.md`).

use super::*;

impl Session {
    /// `mkdir folder [, public]`: one folder, whose parent must be there.
    /// Anything that stops it (it exists, a file has the name, no parent)
    /// is "could not create directory", r(693); no name is "invalid
    /// syntax" (r(198)) and a second name "varlist not allowed" (r(101)).
    /// `rmdir folder` removes an empty one, with "could not remove
    /// directory" (r(693)) otherwise.
    pub(super) fn mkdir(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let make = c.name == "mkdir";
        let known: &[(&str, usize)] = if make { &[("public", 6)] } else { &[] };
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "{} with the option {o} is not translated yet",
                c.name
            )));
        }
        let names = crate::dofile::macros::tokens(&c.parts.main);
        match names.as_slice() {
            [] => out.line("stop(\"invalid syntax (Stata: r(198))\", call. = FALSE)"),
            [one] => {
                let path = file_name(one, "");
                let p = r_text(&path);
                if make {
                    out.line(format!(
                        "if (!suppressWarnings(dir.create({p}, showWarnings = FALSE))) stop({}, call. = FALSE)",
                        r_text(&format!("could not create directory {path} (Stata: r(693))"))
                    ));
                } else {
                    out.line(format!(
                        "if (!dir.exists({p}) || length(list.files({p}, all.files = TRUE, no.. = TRUE)) > 0 || unlink({p}, recursive = TRUE) != 0) stop({}, call. = FALSE)",
                        r_text(&format!("could not remove directory {path} (Stata: r(693))"))
                    ));
                }
            }
            _ => out.line("stop(\"varlist not allowed (Stata: r(101))\", call. = FALSE)"),
        }
        Ok(())
    }
}

impl Session {
    /// `outsheet [varlist] using file [if] [in] [, comma delimiter(char)
    /// nonames nolabel noquote replace]`: the text file `export delimited`
    /// writes, tab-separated by default, numbers in their display formats,
    /// strings and value labels in quotes (a quote inside is not doubled),
    /// extension `.out`. Nothing is printed after the file is written, and
    /// `(file x not found)` only with `replace` and no file there.
    pub(super) fn outsheet(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        if let Some(o) = opts.other(&[
            ("comma", 5),
            ("delimiter", 5),
            ("nonames", 3),
            ("nolabel", 3),
            ("noquote", 3),
            ("replace", 7),
        ]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
        // `outsheet x if x > 1 using f`: the reader keeps `using f` inside the condition
        let mut moved = c.clone();
        if moved.using.is_none() {
            if let Some(e) = moved.parts.if_exp.clone() {
                if let Some((cond, file)) = e.split_once(" using ") {
                    moved.parts.if_exp = Some(cond.trim().to_string());
                    moved.using = Some(file.trim().to_string());
                }
            }
        }
        let c = &moved;
        let Some(file) = c.using.clone() else {
            stop(out, "using required (Stata: r(100))");
            return Ok(());
        };
        let comma = opts.has("comma", 5);
        let typed = opts
            .raw("delimiter", 5)
            .or_else(|| opts.arg("delimiter", 5));
        let delim = match typed {
            None if comma => ",".to_string(),
            None => "\t".to_string(),
            Some(_) if comma => {
                stop(
                    out,
                    "both comma and delimiter() not allowed (Stata: r(198))",
                );
                return Ok(());
            }
            Some(d) => {
                let d = crate::dofile::macros::unquote(&d);
                if d.chars().count() != 1 {
                    stop(
                        out,
                        "option delimiter() incorrectly specified (Stata: r(198))",
                    );
                    return Ok(());
                }
                d
            }
        };
        let path = file_name(&file, "out");
        let vars: Vec<String> = c.parts.main.split_whitespace().map(String::from).collect();
        let d = self.data()?;
        let mut frame = d.clone();
        if let Some(cond) = self.condition(c, out)? {
            out.package("dplyr");
            frame = format!("dplyr::filter({d}, {cond})");
        }
        let names = if vars.is_empty() {
            "NULL".to_string()
        } else if vars.iter().all(|v| is_name(v)) {
            for v in &vars {
                out.line(format!(
                    "if (!{} %in% names({d})) stop({}, call. = FALSE)",
                    r_text(v),
                    r_text(&format!("variable {v} not found (Stata: r(111))"))
                ));
            }
            format!(
                "c({})",
                vars.iter()
                    .map(|v| r_text(v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            return Err(no(
                "outsheet with variable patterns or ranges is not translated yet",
            ));
        };
        out.package("haven");
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_float");
        self.helpers.insert("stata_export_delim");
        let flag = |b: bool| if b { "TRUE" } else { "FALSE" };
        out.line(format!(
            "stata_export_delim({frame}, {p}, {p}, delim = {}, varnames = {}, nolabel = {}, datafmt = TRUE, replace = {}, vars = {names}, sheet = TRUE, noquote = {})",
            r_text(&delim),
            flag(!opts.has("nonames", 3)),
            flag(opts.has("nolabel", 3)),
            flag(opts.has("replace", 7)),
            flag(opts.has("noquote", 3)),
            p = r_text(&path),
        ));
        Ok(())
    }
}
