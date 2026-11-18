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

/// `outfile`'s writer. The free layout (`plan/stata/outfile.md`): each
/// number right-aligned in its display format's width, each string
/// quoted and left-aligned in at least 11, a labelled value quoted and
/// left-aligned in the format's width or its longest label, two blanks
/// after each; without `wide` a field that would bring the line to 80
/// columns starts a new line and lines lose their trailing blanks.
pub(super) const OUTFILE_HELPER: &str = r##"stata_outfile <- function(d, file, shown, vars, comma = FALSE, noquote = FALSE, nolabel = FALSE,
                          wide = FALSE, runtogether = FALSE, replace = FALSE, defaults = character(0)) {
  if (file.exists(file) && !replace) stop("file ", shown, " already exists (Stata: r(602))", call. = FALSE)
  if (runtogether && !all(vapply(vars, function(v) is.character(d[[v]]), NA)))
    stop("type mismatch (Stata: r(109))", call. = FALSE)
  existed <- file.exists(file)
  quote <- !noquote && !runtogether
  eol <- if (.Platform$OS.type == "windows") "\r\n" else "\n"
  nobs <- nrow(d)
  fields <- lapply(vars, function(v) {
    x <- d[[v]]
    if (is.character(x)) {
      fmt <- attr(x, "format.stata", exact = TRUE)
      w <- suppressWarnings(as.integer(sub("^%-?([0-9]+)s$", "\\1", fmt)))
      if (is.null(fmt) || is.na(w)) w <- max(c(nchar(x, "bytes"), 1L), na.rm = TRUE)
      text <- ifelse(is.na(x), "", x)
      if (quote) text <- paste0("\"", text, "\"")
      return(list(text = text, width = if (quote) max(w + 2L, 11L) else max(w, 9L), left = TRUE))
    }
    fmt <- attr(x, "format.stata", exact = TRUE)
    if (is.null(fmt) && v %in% names(defaults)) fmt <- defaults[[v]]
    if (is.null(fmt)) stop("RCraft: the display format of ", v, " is not known here, so outfile cannot lay it out like Stata (not translated yet)", call. = FALSE)
    wm <- regmatches(fmt, regexec("^%-?0?([0-9]+)", fmt))[[1]]
    if (length(wm) == 0) stop("RCraft: outfile with the display format ", fmt, " of ", v, " is not translated yet", call. = FALSE)
    fw <- as.integer(wm[2])
    labs <- if (nolabel) NULL else attr(x, "labels", exact = TRUE)
    y <- as.numeric(unclass(x))
    ok <- !is.na(y)
    text <- rep(".", length(y))
    tag <- rep(NA_character_, length(y))
    if (any(!ok)) tag[!ok] <- haven::na_tag(y[!ok])
    text[!ok & !is.na(tag)] <- paste0(".", tag[!ok & !is.na(tag)])
    if (any(ok)) text[ok] <- vapply(y[ok], function(u) stata_fmt(u, sub("c$", "", fmt), trim = TRUE), "")
    left <- rep(FALSE, length(y))
    width <- fw
    if (length(labs)) {
      width <- max(fw, nchar(names(labs), "chars"))
      hit <- ok & !is.na(match(y, as.numeric(labs)))
      lab <- names(labs)[match(y[hit], as.numeric(labs))]
      text[hit] <- if (quote) paste0("\"", lab, "\"") else lab
      left[hit] <- TRUE
    }
    if (comma) text[!ok] <- ""
    list(text = text, width = width, left = left)
  })
  lines <- character()
  for (i in seq_len(nobs)) {
    if (comma || runtogether) {
      lines <- c(lines, paste(vapply(fields, function(f) f$text[i], ""), collapse = if (runtogether) "" else ","))
      next
    }
    cur <- ""
    for (f in fields) {
      s <- f$text[i]
      left <- if (length(f$left) > 1) f$left[i] else f$left
      pad <- strrep(" ", max(0, f$width - nchar(s, "chars")))
      field <- paste0(if (left) s else NULL, pad, if (!left) s else NULL, "  ")
      if (!wide && nchar(cur) > 0 && nchar(cur) + nchar(field) >= 80) {
        lines <- c(lines, sub(" +$", "", cur))
        cur <- ""
      }
      cur <- paste0(cur, field)
    }
    lines <- c(lines, if (wide) cur else sub(" +$", "", cur))
  }
  if (!existed && replace) cat("(file ", shown, " not found)\n", sep = "")
  writeBin(charToRaw(enc2utf8(paste0(lines, eol, collapse = ""))), file)
  invisible(NULL)
}"##;

impl Session {
    /// `outfile [varlist] using file [if] [in] [, comma noquote nolabel
    /// runtogether wide replace]`: the free layout described above (and
    /// `plan/stata/outfile.md`). `dictionary` and the other forms are not
    /// translated.
    pub(super) fn outfile(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let stop = |out: &mut Out, msg: &str| {
            out.line(format!("stop({}, call. = FALSE)", r_text(msg)));
        };
        if opts.has("dictionary", 4) {
            return Err(no(
                "outfile with the option dictionary is not translated yet",
            ));
        }
        if let Some(o) = opts.other(&[
            ("comma", 5),
            ("noquote", 4),
            ("nolabel", 5),
            ("runtogether", 4),
            ("wide", 4),
            ("replace", 7),
        ]) {
            stop(out, &format!("option {o} not allowed (Stata: r(198))"));
            return Ok(());
        }
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
        let path = file_name(&file, "raw");
        let vars: Vec<String> = c.parts.main.split_whitespace().map(String::from).collect();
        let d = self.data()?;
        let mut frame = d.clone();
        if let Some(cond) = self.condition(c, out)? {
            out.package("dplyr");
            frame = format!("dplyr::filter({d}, {cond})");
        }
        let names = if vars.is_empty() {
            format!("names({d})")
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
                "outfile with variable patterns or ranges is not translated yet",
            ));
        };

        out.package("haven");
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_outfile");
        let defaults: Vec<String> = self
            .storage
            .iter()
            .filter_map(|(v, t)| Some(format!("{} = {}", r_text(v), r_text(t.default_format()?))))
            .collect();
        let defaults = if defaults.is_empty() {
            String::new()
        } else {
            format!(", defaults = c({})", defaults.join(", "))
        };
        let flag = |b: bool| if b { "TRUE" } else { "FALSE" };
        out.line(format!(
            "stata_outfile({frame}, {p}, {p}, {names}, comma = {}, noquote = {}, nolabel = {}, wide = {}, runtogether = {}, replace = {}{defaults})",
            flag(opts.has("comma", 5)),
            flag(opts.has("noquote", 4)),
            flag(opts.has("nolabel", 5)),
            flag(opts.has("wide", 4)),
            flag(opts.has("runtogether", 4)),
            flag(opts.has("replace", 7)),
            p = r_text(&path),
        ));
        out.caveat("outfile lays strings out by their declared width in Stata; RCraft takes the display format's width when the variable has one, else the longest value");
        Ok(())
    }
}
