//! `outreg` and `frmttable` (John Luke Gallup's outreg package, SSC), the
//! part a replication package's balance tables use, as observed in Stata 19
//! on made-up data (`plan/stata/outreg.md`); the package's programs were not
//! read.
//!
//! - After a model, `outreg, keep(x) rtitle(...) stats(b) noautosumm
//!   starlevels(10 5 1) starloc(1) store(name)`: one row per kept
//!   coefficient, %.3f with thousands commas, the stars after it (p below
//!   each level in percent; t with the model's residual df, else normal).
//! - `frmttable, statmat(M) store(name) sfmt(g,f,...)`: the matrix's cells,
//!   `f` with sdec() decimals (2), `g` a whole number, a missing cell empty;
//!   no row titles.
//! - `replay(name)` takes a stored table (none yet: empty), `append(m)` adds
//!   m's rows below, `merge(m)` m's columns to the right (rows by position
//!   when the table has no row titles); `ctitles()` sets the title rows
//!   (`\` between rows, the row-title column first), `note()` the note;
//!   `store()` keeps the result.
//! - `using file, tex fragment plain nocenter [replace]`: a LaTeX tabular,
//!   `l` for the row titles (when there are any) and `c` per column, the
//!   first title row and the first body row after `\hline `, cells joined by
//!   ` & `, rows ending `\\`, `multicol(r,c,n;...)` as `\multicolumn{n}{c}`,
//!   then `\hline\end{tabular}\\` and `\hfil\\` for an empty note; CR LF.

use super::*;

pub(super) const OUTREG_HELPER: &str = r##"stata_or <- new.env()
stata_or_model <- function(keep, rtitle, levels, sdec = 3) {
  df <- get0("stata_df", envir = globalenv(), ifnotfound = Inf)
  if (is.null(df) || length(df) != 1 || is.na(df)) df <- Inf
  cells <- vapply(keep, function(k) {
    b <- stata_bget(k)
    s <- stata_bget(k, se = TRUE)
    t <- b / s
    p <- if (is.finite(df)) 2 * stats::pt(-abs(t), df) else 2 * stats::pnorm(-abs(t))
    paste0(formatC(b, format = "f", digits = sdec, big.mark = ","), strrep("*", sum(p < levels / 100)))
  }, "")
  list(rt = if (length(rtitle)) rtitle else keep, body = matrix(cells, ncol = 1), ct = NULL, note = NA_character_)
}
stata_or_frm <- function(M, sfmt, sdec = 2) {
  M <- as.matrix(M)
  sfmt <- rep_len(sfmt, ncol(M))
  body <- matrix("", nrow(M), ncol(M))
  for (j in seq_len(ncol(M))) for (i in seq_len(nrow(M))) {
    x <- M[i, j]
    body[i, j] <- if (is.na(x)) "" else if (sfmt[j] == "f") sprintf(paste0("%.", sdec, "f"), x) else {
      if (x != round(x)) stop("frmttable: sfmt(g) of a number with decimals is not translated yet", call. = FALSE)
      format(x, scientific = FALSE)
    }
  }
  list(rt = rep("", nrow(M)), body = body, ct = NULL, note = NA_character_)
}
stata_or_get <- function(name) get0(name, envir = stata_or, inherits = FALSE)
stata_or_put <- function(name, tab) assign(name, tab, envir = stata_or)
# a table about to be stored again is gone first, so that a step that stops
# in R leaves no older table under its name to be written in its place
stata_or_drop <- function(name) if (exists(name, envir = stata_or, inherits = FALSE)) rm(list = name, envir = stata_or)
stata_or_append <- function(base, add) {
  if (is.null(base)) return(add)
  if (is.null(add)) stop("outreg: the table to append is not stored", call. = FALSE)
  if (ncol(base$body) != ncol(add$body)) stop("outreg, append(): tables of different widths are not translated yet", call. = FALSE)
  base$body <- rbind(base$body, add$body)
  base$rt <- c(base$rt, add$rt)
  base
}
stata_or_merge <- function(base, add) {
  if (is.null(add)) stop("outreg: the table to merge is not stored", call. = FALSE)
  if (is.null(base)) stop("outreg: the table replay() names is not stored", call. = FALSE)
  if (nrow(base$body) != nrow(add$body))
    stop("outreg, merge(): tables with different rows are not translated yet", call. = FALSE)
  if (any(nzchar(base$rt)) && any(nzchar(add$rt)) && !identical(base$rt, add$rt))
    stop("outreg, merge(): rows matched by title are not translated yet", call. = FALSE)
  rt <- if (any(nzchar(base$rt))) base$rt else add$rt
  nb <- ncol(base$body)
  na <- ncol(add$body)
  ct <- NULL
  if (!is.null(base$ct) || !is.null(add$ct)) {
    hr <- max(NROW(base$ct), NROW(add$ct))
    ct <- matrix("", hr, 1 + nb + na)
    if (!is.null(base$ct)) ct[seq_len(nrow(base$ct)), 1:(1 + nb)] <- base$ct
    if (!is.null(add$ct)) ct[seq_len(nrow(add$ct)), c(1, (2 + nb):(1 + nb + na))] <- cbind(ifelse(nzchar(ct[seq_len(nrow(add$ct)), 1]), ct[seq_len(nrow(add$ct)), 1], add$ct[, 1]), add$ct[, -1, drop = FALSE])
  }
  list(rt = rt, body = cbind(base$body, add$body), ct = ct, note = base$note)
}
stata_or_ctitles <- function(tab, rows) {
  w <- 1 + ncol(tab$body)
  ct <- t(vapply(rows, function(r) {
    if (length(r) > w) stop("outreg, ctitles(): more titles than columns", call. = FALSE)
    c(r, rep("", w - length(r)))
  }, character(w)))
  tab$ct <- ct
  tab
}
stata_or_show <- function(tab) {
  if (is.null(tab)) stop("outreg: the table replay() names is not stored", call. = FALSE)
  titles <- any(nzchar(tab$rt))
  rows <- if (is.null(tab$ct)) list() else lapply(seq_len(nrow(tab$ct)), function(r) if (titles) tab$ct[r, ] else tab$ct[r, -1])
  body <- lapply(seq_len(nrow(tab$body)), function(i) c(if (titles) tab$rt[i], tab$body[i, ]))
  all <- c(rows, body)
  w <- vapply(seq_along(all[[1]]), function(j) max(nchar(vapply(all, `[`, "", j))), 0)
  pad <- function(s, n, left) if (left) paste0(s, strrep(" ", max(0, n - nchar(s)))) else paste0(strrep(" ", max(0, n - nchar(s))), s)
  line <- function(r) cat(" ", paste(mapply(pad, r, w, c(titles, rep(FALSE, length(r) - 1))), collapse = "  "), "\n", sep = "")
  rule <- strrep("-", sum(w) + 2 * length(w))
  cat("\n", rule, "\n", sep = "")
  for (r in rows) line(r)
  if (length(rows)) cat(rule, "\n", sep = "")
  for (r in body) line(r)
  cat(rule, "\n", sep = "")
  invisible(NULL)
}
stata_or_tex <- function(tab, file, replace = FALSE, multicol = list()) {
  if (is.null(tab)) stop("outreg: the table replay() names is not stored", call. = FALSE)
  if (file.exists(file) && !replace) stop("file ", file, " already exists (Stata: r(602))", call. = FALSE)
  if (is.na(tab$note) || nzchar(tab$note)) stop("outreg: a table with outreg's own note, or a note of text, is not translated yet (note(\"\") is)", call. = FALSE)
  titles <- any(nzchar(tab$rt))
  out <- paste0("\\begin{tabular}{", if (titles) "l", strrep("c", ncol(tab$body)), "}")
  first <- TRUE
  if (!is.null(tab$ct)) for (r in seq_len(nrow(tab$ct))) {
    cells <- tab$ct[r, ]
    keep <- rep(TRUE, length(cells))
    for (m in multicol) if (m[1] == r) {
      cells[m[2]] <- paste0("\\multicolumn{", m[3], "}{c}{", cells[m[2]], "}")
      if (m[3] > 1) keep[(m[2] + 1):(m[2] + m[3] - 1)] <- FALSE
    }
    if (!titles) keep[1] <- FALSE
    out <- c(out, paste0(if (first) "\\hline " else "", paste(cells[keep], collapse = " & "), "\\\\"))
    first <- FALSE
  }
  for (i in seq_len(nrow(tab$body))) {
    out <- c(out, paste0(if (i == 1) "\\hline " else "", paste(c(if (titles) tab$rt[i], tab$body[i, ]), collapse = " & "), "\\\\"))
  }
  out <- c(out, "\\hline\\end{tabular}\\\\", "\\hfil\\\\")
  con <- file(file, open = "wb")
  on.exit(close(con))
  writeLines(out, con, sep = "\r\n", useBytes = TRUE)
  invisible(NULL)
}"##;

/// The cells of a `ctitles()` or `multicol()` argument: rows split at `\`
/// (or `;`), cells at commas, outside quotes; quotes taken off.
fn split_rows(text: &str, row_sep: char) -> Vec<Vec<String>> {
    let mut rows = vec![Vec::new()];
    let mut cell = String::new();
    let mut quoted = false;
    for ch in text.chars() {
        match ch {
            '"' => quoted = !quoted,
            c if c == row_sep && !quoted => {
                if let Some(r) = rows.last_mut() {
                    r.push(cell.trim().to_string());
                }
                cell.clear();
                rows.push(Vec::new());
            }
            ',' if !quoted => {
                if let Some(r) = rows.last_mut() {
                    r.push(cell.trim().to_string());
                }
                cell.clear();
            }
            c => cell.push(c),
        }
    }
    if let Some(r) = rows.last_mut() {
        r.push(cell.trim().to_string());
    }
    rows
}

impl Session {
    pub(super) fn outreg(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let known = [
            ("keep", 4),
            ("rtitles", 2),
            ("stats", 4),
            ("noautosumm", 6),
            ("store", 5),
            ("replay", 6),
            ("append", 6),
            ("merge", 5),
            ("ctitles", 2),
            ("note", 4),
            ("starlevels", 5),
            ("starloc", 7),
            ("tex", 3),
            ("nocenter", 8),
            ("fragment", 8),
            ("plain", 5),
            ("replace", 7),
            ("multicol", 8),
        ];
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "outreg with the option {o} is not translated yet"
            )));
        }
        if !c.parts.main.trim().is_empty() || c.parts.if_exp.is_some() || c.parts.in_range.is_some()
        {
            return Err(no("outreg with a varlist, if or in is not translated yet"));
        }
        let name = |o: &str, min: usize| -> Result<Option<String>, Untranslated> {
            match opts.arg(o, min) {
                None => Ok(None),
                Some(a) => {
                    let a = a.trim().to_string();
                    if is_name(&a) {
                        Ok(Some(a))
                    } else {
                        Err(no(format!("outreg, {o}({a}): a table's name is needed")))
                    }
                }
            }
        };
        let replay = name("replay", 6)?;
        let append = name("append", 6)?;
        let merge = name("merge", 5)?;
        let store = name("store", 5)?;
        for n in [&replay, &append, &merge].into_iter().flatten() {
            if let Some(line) = self.outreg_lost.get(n) {
                return Err(no(format!(
                    "outreg: the table {n} is not known here: a line that may store it was not translated ({line})"
                )));
            }
        }
        if append.is_some() && merge.is_some() {
            return Err(no(
                "outreg with both append() and merge() is not translated yet",
            ));
        }
        self.helpers.insert("stata_outreg");
        // the table: a stored one, or the model's coefficients
        let mut steps = Vec::new();
        // the table to be stored is gone until it is stored again (once the
        // replay has read it): a step that stops in R leaves none behind
        let drop = store
            .as_ref()
            .map(|s| format!("stata_or_drop({})", r_text(s)));
        if let Some(r) = &replay {
            for o in ["keep", "rtitles", "stats", "starlevels", "starloc"] {
                if opts.has(o, 4) {
                    return Err(no(format!(
                        "outreg, replay() with {o}() is not translated yet"
                    )));
                }
            }
            steps.push(format!("stata_t <- stata_or_get({})", r_text(r)));
            steps.extend(drop.clone());
        } else {
            self.estimates_for("outreg")?;
            steps.extend(drop.clone());
            if opts.arg("stats", 4).as_deref().map(str::trim) != Some("b")
                || !opts.has("noautosumm", 6)
            {
                return Err(no(
                    "outreg of a model is translated with stats(b) noautosumm (coefficients alone); the rest of its layout is not yet",
                ));
            }
            if opts.arg("starloc", 7).as_deref().map(str::trim) != Some("1") {
                return Err(no(
                    "outreg, stats(b) without starloc(1) is not translated yet",
                ));
            }
            let levels: Vec<f64> = match opts.arg("starlevels", 5) {
                Some(l) => l
                    .split_whitespace()
                    .map(|w| w.parse::<f64>())
                    .collect::<Result<_, _>>()
                    .map_err(|_| no("outreg, starlevels(): numbers are needed"))?,
                None => return Err(no("outreg without starlevels() is not translated yet")),
            };
            let keep: Vec<String> = opts
                .arg("keep", 4)
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_string)
                .collect();
            if keep.is_empty() {
                return Err(no("outreg of a model without keep() is not translated yet"));
            }
            let rtitles: Vec<String> = match opts.arg("rtitles", 2) {
                Some(t) => split_rows(&t, '\\').into_iter().flatten().collect(),
                None => Vec::new(),
            };
            if !rtitles.is_empty() && rtitles.len() != keep.len() {
                return Err(no(
                    "outreg, rtitle(): one title per kept coefficient is translated",
                ));
            }
            self.helpers.insert("stata_returned");
            self.helpers.insert("stata_names");
            steps.push(format!(
                "stata_t <- stata_or_model(c({}), c({}), c({}))",
                keep.iter()
                    .map(|k| r_text(k))
                    .collect::<Vec<_>>()
                    .join(", "),
                rtitles
                    .iter()
                    .map(|k| r_text(k))
                    .collect::<Vec<_>>()
                    .join(", "),
                levels
                    .iter()
                    .map(|l| r_number(*l))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(a) = &append {
            steps.push(format!(
                "stata_t <- stata_or_append(stata_t, stata_or_get({}))",
                r_text(a)
            ));
        }
        if let Some(m) = &merge {
            steps.push(format!(
                "stata_t <- stata_or_merge(stata_t, stata_or_get({}))",
                r_text(m)
            ));
        }
        if let Some(ct) = opts.arg("ctitles", 2) {
            let rows = split_rows(&ct, '\\');
            steps.push(format!(
                "stata_t <- stata_or_ctitles(stata_t, list({}))",
                rows.iter()
                    .map(|r| format!(
                        "c({})",
                        r.iter().map(|x| r_text(x)).collect::<Vec<_>>().join(", ")
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if let Some(n) = opts.arg("note", 4) {
            let n = n.trim().trim_matches('"').to_string();
            steps.push(format!("stata_t$note <- {}", r_text(&n)));
        }
        if let Some(s) = &store {
            steps.push(format!("stata_or_put({}, stata_t)", r_text(s)));
        }
        match &c.using {
            Some(file) => {
                for (o, min) in [("tex", 3), ("fragment", 8), ("plain", 5), ("nocenter", 8)] {
                    if !opts.has(o, min) {
                        return Err(no(format!(
                            "outreg using a file is translated as LaTeX with tex fragment plain nocenter; without {o} not yet"
                        )));
                    }
                }
                let mut path = file.trim().trim_matches('"').to_string();
                if !path.to_lowercase().ends_with(".tex") {
                    path.push_str(".tex");
                }
                let multicol: Vec<String> = match opts.arg("multicol", 8) {
                    Some(m) => {
                        let mut v = Vec::new();
                        for r in split_rows(&m, ';') {
                            let nums: Vec<u32> = r
                                .iter()
                                .map(|x| x.parse::<u32>())
                                .collect::<Result<_, _>>()
                                .map_err(|_| {
                                    no("outreg, multicol(): three numbers a cell are needed")
                                })?;
                            let [r, col, n] = nums.as_slice() else {
                                return Err(no(
                                    "outreg, multicol(): three numbers a cell are needed",
                                ));
                            };
                            v.push(format!("c({r}, {col}, {n})"));
                        }
                        v
                    }
                    None => Vec::new(),
                };
                steps.push(format!(
                    "stata_or_tex(stata_t, {}, replace = {}, multicol = list({}))",
                    r_text(&path),
                    super::stats::r_logical(opts.has("replace", 7)),
                    multicol.join(", ")
                ));
            }
            None => {
                if !self.quiet(c) {
                    steps.push("stata_or_show(stata_t)".into());
                }
            }
        }
        steps.push("rm(stata_t)".into());
        for s in steps {
            out.line(s);
        }
        if let Some(s) = &store {
            self.outreg_lost.remove(s);
        }
        out.caveat("the table on screen in R's layout; a .tex file as outreg writes it");
        Ok(())
    }

    pub(super) fn frmttable(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("statmat", 5), ("store", 5), ("sfmt", 4), ("sdec", 4)]) {
            return Err(no(format!(
                "frmttable with the option {o} is not translated yet"
            )));
        }
        if c.using.is_some() || !c.parts.main.trim().is_empty() {
            return Err(no("frmttable using a file is not translated yet"));
        }
        let Some(m) = opts.arg("statmat", 5).map(|m| m.trim().to_string()) else {
            return Err(no("frmttable without statmat() is not translated yet"));
        };
        let Some(store) = opts.arg("store", 5).map(|s| s.trim().to_string()) else {
            return Err(no("frmttable without store() is not translated yet"));
        };
        let sfmt: Vec<String> = opts
            .arg("sfmt", 4)
            .unwrap_or_else(|| "g".into())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        if let Some(f) = sfmt.iter().find(|f| !matches!(f.as_str(), "g" | "f")) {
            return Err(no(format!(
                "frmttable, sfmt({f}) is not translated yet (g and f are)"
            )));
        }
        let sdec = match opts.arg("sdec", 4) {
            Some(s) => s
                .trim()
                .parse::<u32>()
                .map_err(|_| no("frmttable, sdec(): one whole number is translated"))?,
            None => 2,
        };
        let mat = self.user_matrix(&m)?;
        self.helpers.insert("stata_outreg");
        self.outreg_lost.remove(&store);
        out.line(format!("stata_or_drop({})", r_text(&store)));
        out.line(format!(
            "stata_or_put({}, stata_or_frm({mat}, c({}), sdec = {sdec}))",
            r_text(&store),
            sfmt.iter()
                .map(|f| r_text(f))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        Ok(())
    }
}

impl Session {
    /// `mata: mata clear`: Mata's memory emptied, which in Stata takes the
    /// tables outreg stored with it (outreg keeps them in Mata); other Mata
    /// is not translated.
    pub(super) fn mata_line(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let main = c.parts.main.trim().trim_start_matches(':').trim();
        if main != "mata clear" || !c.options.is_empty() {
            return Err(no("mata is not translated yet (mata: mata clear is)"));
        }
        self.outreg_lost.clear();
        out.line("if (exists(\"stata_or\")) rm(list = ls(stata_or), envir = stata_or)");
        out.caveat("Mata's memory holds nothing in R but outreg's tables, which are emptied");
        Ok(())
    }
}
