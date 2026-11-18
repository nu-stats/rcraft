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
