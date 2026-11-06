//! `postfile`, `post` and `postclose`: rows of results posted one at a time
//! and written to a .dta file at the end. The rules are Stata 19's, as
//! StatCraft decoded them (`plan/stata/postfile.md` there).

use super::*;
use crate::lang::syntax::abbrev;

/// The rows posted are kept in a list and written with haven at
/// `postclose`; values take the declared variable's type as they are posted
/// (a float is rounded to float, a string cut to its width).
pub(super) const POST_HELPER: &str = r##"if (!exists("stata_posts")) stata_posts <- list()
stata_postfile <- function(h, names, types, file, replace = FALSE, show = TRUE) {
  if (!is.null(stata_posts[[h]])) stop("postfile ", h, " already exists (Stata: r(110))", call. = FALSE)
  if (anyDuplicated(names)) stop("postfile: variable ", names[duplicated(names)][1], " already exists (Stata: r(110))", call. = FALSE)
  there <- file.exists(file)
  if (there && !replace) stop("file ", file, " already exists (Stata: r(602))", call. = FALSE)
  if (!there && replace && show) cat("(file ", file, " not found)\n", sep = "")
  stata_posts[[h]] <<- list(file = file, names = names, types = types, rows = list())
  invisible(NULL)
}
stata_post <- function(h, ...) {
  p <- stata_posts[[h]]
  if (is.null(p)) stop("post ", h, " not found (Stata: r(111))", call. = FALSE)
  v <- tryCatch(list(...), error = function(e) {
    stata_posts[[h]] <<- NULL
    stop(e)
  })
  k <- length(p$names)
  if (length(v) != k) {
    stata_posts[[h]] <<- NULL
    stop("post: ", k, " expressions expected and ", length(v), " found (Stata: r(198))", call. = FALSE)
  }
  for (i in seq_len(k)) {
    str <- startsWith(p$types[i], "str")
    if (is.character(v[[i]]) != str) {
      stata_posts[[h]] <<- NULL
      stop("type mismatch: post expression ", i, ", variable ", p$names[i], " (Stata: r(109))", call. = FALSE)
    }
    if (str && p$types[i] != "strL") v[[i]] <- substr(v[[i]], 1, as.integer(substring(p$types[i], 4)))
    if (p$types[i] == "float") v[[i]] <- stata_float(v[[i]])
  }
  p$rows[[length(p$rows) + 1]] <- v
  stata_posts[[h]] <<- p
  invisible(NULL)
}
stata_postclose <- function(h) {
  p <- stata_posts[[h]]
  if (is.null(p)) stop("post ", h, " not found (Stata: r(111))", call. = FALSE)
  cols <- lapply(seq_along(p$names), function(i) {
    if (startsWith(p$types[i], "str")) return(vapply(p$rows, function(r) as.character(r[[i]]), ""))
    x <- vapply(p$rows, function(r) as.numeric(r[[i]]), 0)
    if (p$types[i] %in% c("byte", "int", "long")) as.integer(x) else x
  })
  d <- structure(stats::setNames(cols, p$names), row.names = .set_row_names(length(p$rows)), class = "data.frame")
  haven::write_dta(d, p$file)
  stata_posts[[h]] <<- NULL
  invisible(NULL)
}"##;

/// Whether a word of `postfile`'s variable list is a storage type.
fn storage_type(w: &str) -> bool {
    matches!(w, "byte" | "int" | "long" | "float" | "double" | "strL")
        || w.strip_prefix("str")
            .is_some_and(|n| !n.is_empty() && n.chars().all(|ch| ch.is_ascii_digit()))
}

/// The expressions of `post h (exp) (exp) ...`, each without its
/// parentheses; `None` when something is not in parentheses.
fn bound_expressions(mut tail: &str) -> Result<Option<Vec<&str>>, Untranslated> {
    let mut exps = Vec::new();
    loop {
        tail = tail.trim_start();
        if tail.is_empty() {
            return Ok(Some(exps));
        }
        if !tail.starts_with('(') {
            return Ok(None);
        }
        let mut depth = 0usize;
        let mut quoted = false;
        let mut end = None;
        for (i, ch) in tail.char_indices() {
            match ch {
                '"' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let end = end.ok_or_else(|| no("post: a ( without its )"))?;
        exps.push(tail.get(1..end).unwrap_or("").trim());
        tail = tail.get(end + 1..).unwrap_or("");
    }
}

impl Session {
    /// `postfile h newvarlist using file [, replace]`, `post h (exp) ...`
    /// and `postclose h`.
    pub(super) fn post_cmd(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let rest = super::returned::after_name(c);
        let h = match c.name.as_str() {
            "postfile" => c.parts.main.split_whitespace().next().unwrap_or(""),
            _ => rest.split_whitespace().next().unwrap_or(""),
        };
        if self.post_lost.contains(h) {
            return Err(no(format!(
                "{} {h}: a post {h} line before it was not translated, and an error there closes the postfile in Stata, so whether {h} is open is not known",
                c.name
            )));
        }
        match c.name.as_str() {
            "postfile" => {
                let done = self.postfile(c, opts, out);
                // declined: Stata may have opened it
                if done.is_err() {
                    self.post_lost.insert(h.to_string());
                }
                done
            }
            "postutil" => {
                let sub = rest.trim();
                if abbrev(sub, "clear", 5) {
                    // each postfile is written and closed, as postclose does
                    self.helpers.insert("stata_post");
                    out.package("haven");
                    out.line("for (stata_h in names(stata_posts)) stata_postclose(stata_h)");
                    let open: Vec<String> =
                        std::mem::take(&mut self.post_handles).into_iter().collect();
                    self.post_closed.extend(open);
                    let lost: Vec<String> =
                        std::mem::take(&mut self.post_lost).into_iter().collect();
                    self.post_closed.extend(lost);
                    Ok(())
                } else if abbrev(sub, "dir", 3)
                    && self.post_handles.is_empty()
                    && self.post_lost.is_empty()
                {
                    // nothing open: Stata lists nothing
                    Ok(())
                } else {
                    Err(no(format!("postutil {sub} is not translated yet (postutil clear is, and dir with no postfile open)")))
                }
            }
            "post" => {
                let text = rest.trim();
                let (h, tail) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
                if !self.post_handles.contains(h) && !self.post_closed.contains(h) {
                    return Err(no(format!(
                        "post {h}: no postfile {h} before this line in the do-file was translated"
                    )));
                }
                let lose = |s: &mut Self, e: Untranslated| {
                    s.post_handles.remove(h);
                    s.post_lost.insert(h.to_string());
                    e
                };
                let exps = match bound_expressions(tail) {
                    Ok(Some(exps)) => exps,
                    Ok(None) => {
                        // the error closes the postfile, as every error of post does
                        self.helpers.insert("stata_post");
                        out.line(format!("stata_posts[[{}]] <- NULL", r_text(h)));
                        out.line(format!(
                            "stop({}, call. = FALSE)",
                            r_text("post command requires expressions be bound in parentheses (Stata: r(198))")
                        ));
                        return Ok(());
                    }
                    Err(e) => return Err(lose(self, e)),
                };
                let mut vals = Vec::new();
                for e in exps {
                    match self.display_value(e, out) {
                        Ok((code, _)) => vals.push(code),
                        Err(e) => return Err(lose(self, e)),
                    }
                }
                let args: String = vals.iter().map(|v| format!(", {v}")).collect();
                self.helpers.insert("stata_post");
                out.line(format!("stata_post({}{args})", r_text(h)));
                Ok(())
            }
            _ => {
                let h = rest.trim();
                if !self.post_handles.remove(h) && !self.post_closed.contains(h) {
                    return Err(no(format!(
                        "postclose {h}: no postfile {h} before this line in the do-file was translated"
                    )));
                }
                self.helpers.insert("stata_post");
                out.package("haven");
                out.line(format!("stata_postclose({})", r_text(h)));
                self.post_closed.insert(h.to_string());
                Ok(())
            }
        }
    }

    fn postfile(&mut self, c: &Command, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        // every(#) only sets how often Stata writes to the file; the file is the same
        if let Some(o) = opts.other(&[("replace", 7), ("every", 5)]) {
            return Err(no(format!(
                "postfile with the option {o} is not translated yet"
            )));
        }
        let file = c
            .using
            .clone()
            .ok_or_else(|| no("postfile needs using and a file (Stata: r(100))"))?;
        // `double(a b c)`: one type for each name in the parentheses
        let mut spread = String::new();
        let mut rest = c.parts.main.as_str();
        while let Some(open) = rest.find('(') {
            let close = rest[open..]
                .find(')')
                .map(|k| open + k)
                .ok_or_else(|| no("postfile: a ( without its )"))?;
            let before = &rest[..open];
            let ty_at = before.rfind(char::is_whitespace).map_or(0, |k| k + 1);
            let ty = &before[ty_at..];
            if !storage_type(ty) {
                return Err(no(format!(
                    "postfile with {ty}( in the variables is not translated yet"
                )));
            }
            spread.push_str(&before[..ty_at]);
            for n in rest[open + 1..close].split_whitespace() {
                spread.push_str(&format!(" {ty} {n}"));
            }
            spread.push(' ');
            rest = &rest[close + 1..];
        }
        spread.push_str(rest);
        let mut words = spread.split_whitespace();
        let h = words.next().ok_or_else(|| no("postfile needs a name"))?;
        if !is_name(h) {
            return Err(no(format!("postfile {h}: an invalid name (Stata: r(198))")));
        }
        let (mut names, mut types) = (Vec::new(), Vec::new());
        let mut pending: Option<&str> = None;
        for w in words {
            if storage_type(w) && pending.is_none() {
                pending = Some(w);
                continue;
            }
            if w.len() > 32 {
                return Err(no(format!(
                    "postfile with {w} in the variables is not translated yet"
                )));
            }
            if !is_name(w) {
                // Stata stops, and the postfile is not opened
                out.line(format!(
                    "stop({}, call. = FALSE)",
                    r_text(&format!("{w} invalid name (Stata: r(198))"))
                ));
                return Ok(());
            }
            names.push(r_text(w));
            types.push(r_text(pending.take().unwrap_or("float")));
        }
        if names.is_empty() {
            return Err(no("postfile needs the new variables (Stata: r(100))"));
        }
        let path = file_name(&file, "dta");
        self.helpers.insert("stata_float");
        self.helpers.insert("stata_post");
        out.package("haven");
        out.line(format!(
            "stata_postfile({}, c({}), c({}), {}, replace = {}, show = {})",
            r_text(h),
            names.join(", "),
            types.join(", "),
            r_text(&path),
            if opts.has("replace", 7) {
                "TRUE"
            } else {
                "FALSE"
            },
            if self.quiet(c) { "FALSE" } else { "TRUE" },
        ));
        self.post_handles.insert(h.to_string());
        self.post_closed.remove(h);
        Ok(())
    }
}
