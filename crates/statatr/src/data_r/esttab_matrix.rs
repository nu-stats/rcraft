//! `esttab matrix(M[, fmt()]) [using file] [, options]`: a matrix the do-file made, laid out
//! as Stata 19 lays it out (`plan/stata/esttab-matrix.md`): its row and column names, a
//! row of the model label (the matrix's name unless `mlabels()` gives one, `none` leaving
//! it out), each cell `%9.0g` or `fmt()`'s decimals (one for all, or one for each column),
//! a missing value `.`. On the screen and in a .txt file between rules, in a .tex the
//! tabular's head and foot unless `fragment`, `posthead()`, `prefoot()` and `postfoot()`
//! in their places; a .csv as `=""` cells.

use super::*;

impl Session {
    /// `esttab matrix(...)`: see the module.
    pub(super) fn esttab_matrix(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let known = [
            ("mlabels", 7),
            ("title", 5),
            ("replace", 7),
            ("nonotes", 7),
            ("posthead", 8),
            ("prefoot", 7),
            ("postfoot", 8),
            ("fragment", 8),
            ("tex", 3),
            ("csv", 3),
        ];
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "esttab matrix() with the option {o} is not translated yet"
            )));
        }
        let main = c.parts.main.trim();
        let inner = main
            .strip_prefix("matrix(")
            .and_then(|m| m.strip_suffix(')'))
            .ok_or_else(|| no("esttab matrix(): one matrix(name[, fmt()]) is translated"))?;
        let (name, mopts) = super::coefplot::split_top_comma(inner);
        let name = name.trim();
        if !is_name(name) {
            return Err(no(format!(
                "esttab matrix({name}): a matrix's name is translated"
            )));
        }
        let mo = crate::lang::syntax::options(mopts);
        let mo = Opts(&mo);
        if let Some(o) = mo.other(&[("fmt", 3)]) {
            return Err(no(format!(
                "esttab matrix({name}, {o}) is not translated yet (fmt() is)"
            )));
        }
        let fmts = match mo.arg("fmt", 3) {
            None => "NULL".to_string(),
            Some(f) => {
                let ds: Vec<&str> = f.split_whitespace().collect();
                if ds.is_empty() || !ds.iter().all(|d| d.parse::<u8>().is_ok_and(|n| n <= 20)) {
                    return Err(no(format!(
                        "esttab matrix({name}, fmt({f})): numbers of decimals are translated (not %fmts yet)"
                    )));
                }
                format!("c({})", ds.join(", "))
            }
        };
        let m = self.user_matrix(name)?;
        let file = match &c.using {
            Some(f) => {
                let f = f.trim().trim_matches('"').replace('\\', "/");
                let ext = f.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
                let kind = if opts.has("tex", 3) || ext == "tex" {
                    "tex"
                } else if opts.has("csv", 3) || ext == "csv" {
                    "csv"
                } else if ext == "txt" {
                    "screen"
                } else {
                    return Err(no(format!(
                        "esttab matrix() using a .{ext} file is not translated yet (.txt, .csv and .tex are)"
                    )));
                };
                Some((f, kind))
            }
            None if opts.has("tex", 3) || opts.has("csv", 3) => {
                return Err(no(
                    "esttab matrix() with tex or csv on the screen is not translated yet",
                ))
            }
            None => None,
        };
        let kind = file.as_ref().map(|(_, k)| *k).unwrap_or("screen");
        let fragment = opts.has("fragment", 8);
        let heads = ["posthead", "prefoot", "postfoot"];
        if kind != "tex" {
            if fragment {
                return Err(no("esttab, fragment outside .tex is not translated yet"));
            }
            if let Some(h) = heads.iter().find(|h| opts.has(h, h.len().min(8))) {
                return Err(no(format!(
                    "esttab matrix(), {h}() outside .tex is not translated yet"
                )));
            }
        }
        if !fragment && heads.iter().any(|h| opts.has(h, h.len().min(8))) {
            return Err(no(
                "esttab matrix(), posthead() prefoot() postfoot() without fragment is not translated yet",
            ));
        }
        if kind != "screen" && opts.has("title", 5) {
            return Err(no(
                "esttab matrix(), title() in a .tex or .csv is not translated yet",
            ));
        }
        let lines = |name: &str, m: usize| -> Result<String, Untranslated> {
            match opts.raw(name, m) {
                None => Ok("NULL".into()),
                Some(a) => {
                    if a.contains("`\"") || a.contains('@') {
                        return Err(no(format!(
                            "esttab matrix(), {name}() with compound quotes or @ is not translated yet"
                        )));
                    }
                    let ls = super::esttab::quoted_lines(&a).ok_or_else(|| {
                        no(format!(
                            "esttab matrix(), {name}() of text outside quotes is not translated yet"
                        ))
                    })?;
                    Ok(format!(
                        "c({})",
                        ls.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
                    ))
                }
            }
        };
        let posthead = lines("posthead", 8)?;
        let prefoot = lines("prefoot", 7)?;
        let postfoot = lines("postfoot", 8)?;
        // mlabels(none), mlabels("text") (one), else the matrix's name
        let mlabel = match opts.raw("mlabels", 7) {
            None => "NULL".to_string(),
            Some(a) if a.trim() == "none" => "NA".to_string(),
            Some(a) => {
                let ws = super::coefplot::quoted_words_kept(a.trim());
                match ws.as_slice() {
                    [w] => r_text(
                        w.strip_prefix('"')
                            .and_then(|w| w.strip_suffix('"'))
                            .unwrap_or(w),
                    ),
                    _ => return Err(no(
                        "esttab matrix(), mlabels() of other than one label is not translated yet",
                    )),
                }
            }
        };
        let title = match opts.raw("title", 5) {
            None => "NULL".to_string(),
            Some(t) => {
                let t = t.trim();
                let t = t
                    .strip_prefix('"')
                    .and_then(|t| t.strip_suffix('"'))
                    .unwrap_or(t);
                if t.contains('"') {
                    return Err(no(
                        "esttab matrix(), title() of several texts is not translated yet",
                    ));
                }
                r_text(t)
            }
        };
        self.helpers.insert("stata_esttab");
        self.helpers.insert("stata_fmt");
        let call = format!(
            "stata_esttab_matrix({m}, {}, fmts = {fmts}, mlabel = {mlabel}, title = {title}, format = {}, file = {}, replace = {}, posthead = {posthead}, prefoot = {prefoot}, postfoot = {postfoot}, fragment = {})",
            r_text(name),
            r_text(kind),
            file.as_ref().map(|(f, _)| r_text(f)).unwrap_or_else(|| "NULL".into()),
            super::stats::r_logical(opts.has("replace", 7)),
            super::stats::r_logical(fragment)
        );
        if file.is_none() && self.quiet(c) {
            return Ok(());
        }
        out.line(if file.is_some() {
            super::returned::quietly(c, call)
        } else {
            call
        });
        out.caveat(
            "esttab matrix() as the estout package prints it, matched to its output (not its code)",
        );
        Ok(())
    }
}
