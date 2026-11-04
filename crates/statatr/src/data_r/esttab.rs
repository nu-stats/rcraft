//! `esttab` (Ben Jann's estout package, SSC): tables of stored models, as
//! the package prints and writes them, decoded from Stata 19's output alone
//! (`plan/stata/esttab.md`), never its code.
//!
//! The rows are the models' coefficients by Stata's names in e(b), in the
//! order they first appear with `_cons` last (base levels as 0 over `(.)`
//! unless `nobaselevels`); under each, its t statistic (or `se`, `p`, none
//! with `not`) in parentheses; then `N` and the statistics asked for. The
//! numbers use esttab's automatic format (a3) unless a format is given.
//! The screen table and `.txt` are laid out in columns of fixed width;
//! `.csv` cells are `="..."`; `.tex` is a tabular with `\sym{}` stars,
//! `booktabs` rules or plain ones. Files end their lines with CR LF.

use super::*;

/// The table, its cells formatted, and its renderers.
pub(super) const ESTTAB_HELPER: &str = r##"stata_esttab_fmt <- function(x, fmt) {
  # esttab's a# (automatic), a fixed number of decimals (2) or a Stata %w.df / %w.de
  vapply(x, function(v) {
    if (is.na(v)) return(".")
    if (grepl("^a[0-9]+$", fmt)) {
      k <- as.numeric(sub("a", "", fmt))
      if (v == round(v) && abs(v) < 1e15) return(formatC(v, format = "f", digits = 0))
      a <- abs(v)
      # Stata's log10 is ln(x) / ln(10), a little below at the powers of 10
      fl <- floor(log(a) / log(10))
      if (fl < -6) return(sprintf(paste0("%.", k - 1, "e"), v))
      d <- if (a < 1) k - 1 - fl else max(1, k - fl)
      return(formatC(v, format = "f", digits = d))
    }
    if (grepl("^[0-9]+$", fmt)) return(formatC(v, format = "f", digits = as.numeric(fmt)))
    m <- regmatches(fmt, regexec("^%-?0?[0-9]*\\.([0-9]+)([fe])c?$", fmt))[[1]]
    if (length(m)) return(formatC(v, format = m[3], digits = as.numeric(m[2])))
    stop("esttab: the format ", fmt, " is not translated yet")
  }, "")
}
# padding to a width (R's formatC counts a backslash twice): w < 0 pads on the right
stata_pad <- function(s, w) {
  n <- pmax(0, abs(w) - nchar(s))
  if (w < 0) paste0(s, strrep(" ", n)) else paste0(strrep(" ", n), s)
}
stata_esttab <- function(models, d, aux = "t", bfmt = "a3", auxfmt = NULL, star = TRUE,
                         stars = c("*" = 0.05, "**" = 0.01, "***" = 0.001), star_text = NULL,
                         label = FALSE, nobase = FALSE, keep = NULL, drop = NULL, order = NULL,
                         stats = character(), statfmt = list(), noobs = FALSE, obslast = FALSE,
                         numbers = TRUE, mtitles = NULL, nomtitles = FALSE, title = NULL, notes = TRUE,
                         addnotes = character(), varwidth = NULL, modelwidth = 12, format = "screen",
                         booktabs = FALSE, file = NULL, replace = FALSE) {
  wild <- function(p) paste0("^", gsub("\\?", ".", gsub("\\*", ".*", gsub("([.+^$(){}|\\[\\]])", "\\\\\\1", p))), "$")
  hits <- function(n, pats) Reduce(`|`, lapply(pats, function(p) grepl(wild(p), n)), rep(FALSE, length(n)))
  getm <- function(name) {
    e <- if (name == ".") {
      if (!exists("stata_e")) stop("esttab: no estimation results in memory")
      stata_e
    } else {
      s <- if (exists("stata_stored")) stata_stored[[name]] else NULL
      if (is.null(s)) stop("esttab: estimation result ", name, " not found")
      s$stata_e
    }
    raw <- colnames(e$b)
    if (any(grepl(":", raw))) stop("esttab of a model with several equations is not translated yet")
    if (any(grepl("#", raw))) stop("esttab of interactions is not translated yet")
    base <- grepl("^[0-9]+b?n?b\\.", raw) | grepl("^o\\.", raw) | grepl("^[0-9]+o\\.", raw)
    nm <- sub("^o\\.", "", sub("^([0-9]+)[bno]+\\.", "\\1.", raw))
    b <- as.numeric(e$b[1, ])
    se <- sqrt(pmax(diag(as.matrix(e$V)), 0))
    df <- if (!is.null(e$df_r) && is.finite(e$df_r)) e$df_r else Inf
    t <- ifelse(base, NA, b / se)
    p <- if (is.finite(df)) 2 * stats::pt(-abs(t), df) else 2 * stats::pnorm(-abs(t))
    b[base] <- 0
    se[base] <- NA
    k <- if (!is.null(e$rank)) e$rank else length(b) - sum(base)
    st <- list(N = e$N, r2 = e$r2, ar2 = e$r2_a, pr2 = e$r2_p,
               aic = if (!is.null(e$ll)) -2 * e$ll + 2 * k else NULL,
               bic = if (!is.null(e$ll)) -2 * e$ll + k * log(e$N) else NULL)
    for (s in setdiff(stats, names(st))) st[[s]] <- e[[s]]
    list(t = data.frame(name = nm, base = base, b = b, se = se, t = t, p = p, stringsAsFactors = FALSE)[!(base & nobase), , drop = FALSE],
         stats = st, depvar = if (is.null(e$depvar)) "" else e$depvar)
  }
  ms <- lapply(models, getm)
  all <- unique(unlist(lapply(ms, function(m) m$t$name)))
  all <- c(setdiff(all, "_cons"), intersect(all, "_cons"))
  if (length(keep)) all <- all[hits(all, keep)]
  if (length(drop)) all <- all[!hits(all, drop)]
  if (length(order)) {
    first <- character()
    for (o in order) first <- c(first, setdiff(all[hits(all, o)], first))
    all <- c(first, setdiff(all, first))
  }
  lab <- function(n) {
    if (!label) return(n)
    if (n == "_cons") return("Constant")
    m <- regmatches(n, regexec("^([0-9]+)\\.(.+)$", n))[[1]]
    if (length(m)) {
      v <- m[3]
      l <- if (v %in% names(d)) attr(d[[v]], "labels", exact = TRUE) else NULL
      k <- if (!is.null(l)) names(l)[match(as.numeric(m[2]), as.numeric(l))] else NA
      return(if (length(k) && !is.na(k)) k else paste0(v, "=", m[2]))
    }
    if (n %in% names(d)) stata_varlab(d, n) else n
  }
  afmt <- if (is.null(auxfmt)) switch(aux, t = "2", se = "a3", p = "3", "a3") else auxfmt
  cells <- lapply(ms, function(m) {
    r <- lapply(all, function(n) {
      i <- match(n, m$t$name)
      if (is.na(i)) return(list(main = "", st = "", aux = ""))
      x <- m$t[i, ]
      s <- ""
      if (star && !x$base && !is.na(x$p)) for (k in names(stars)) if (x$p < stars[[k]]) s <- k
      a <- switch(aux, t = x$t, se = x$se, p = x$p, NA)
      list(main = stata_esttab_fmt(x$b, bfmt), st = s,
           aux = if (aux == "none") "" else paste0("(", if (x$base) "." else stata_esttab_fmt(a, afmt), ")"))
    })
    r
  })
  srows <- c(if (!noobs && !obslast) "N", stats, if (!noobs && obslast) "N")
  slabel <- function(s) {
    if (label) switch(s, N = "Observations", r2 = "R-squared", ar2 = "Adjusted R-squared", pr2 = "Pseudo R-squared", aic = "AIC", bic = "BIC", s)
    else switch(s, N = "N", r2 = "R-sq", ar2 = "adj. R-sq", pr2 = "pseudo R-sq", aic = "AIC", bic = "BIC", s)
  }
  scell <- function(m, s) {
    v <- m$stats[[s]]
    if (is.null(v) || is.na(v)) return("")
    # the statistics take b()'s format unless given their own; N is a count
    f <- if (!is.null(statfmt[[s]])) statfmt[[s]] else if (s == "N") "a3" else bfmt
    stata_esttab_fmt(v, f)
  }
  heads <- list()
  if (numbers) heads[[length(heads) + 1]] <- paste0("(", seq_along(ms), ")")
  if (!nomtitles) {
    heads[[length(heads) + 1]] <- if (!is.null(mtitles)) rep_len(mtitles, length(ms)) else
      vapply(ms, function(m) if (label && m$depvar %in% names(d)) stata_varlab(d, m$depvar) else m$depvar, "")
  }
  note1 <- switch(aux, t = "t statistics in parentheses", se = "Standard errors in parentheses",
                  p = "p-values in parentheses", NULL)
  note2 <- if (star) {
    if (!is.null(star_text)) star_text else paste(paste0(names(stars), " p<", format(unname(stars), scientific = FALSE, drop0trailing = TRUE)), collapse = ", ")
  }
  nts <- c(if (notes) c(note1, note2), addnotes)
  vw <- if (!is.null(varwidth)) varwidth else if (label) 20 else 12
  sw <- if (star) 3 else 0
  nm_cell <- function(s, w) {
    if (nchar(s) <= w) return(stata_pad(s, -w))
    paste0(substr(s, 1, w - 2), "~", substr(s, nchar(s), nchar(s)))
  }
  out <- character()
  if (format %in% c("screen", "txt")) {
    width <- vw + length(ms) * (1 + modelwidth + sw)
    rule <- strrep("-", width)
    cell <- function(x, s = "") paste0(" ", stata_pad(x, modelwidth), stata_pad(s, -sw))
    row <- function(n, xs, ss = rep("", length(xs))) paste0(nm_cell(n, vw), paste0(mapply(cell, xs, ss), collapse = ""))
    if (format == "screen") out <- ""
    if (!is.null(title)) out <- c(out, title)
    out <- c(out, rule)
    for (h in heads) out <- c(out, row("", h))
    out <- c(out, rule)
    for (j in seq_along(all)) {
      # one line a coefficient with not: no blank lines between them
      if (j > 1 && aux != "none") out <- c(out, "")
      out <- c(out, row(lab(all[j]), vapply(cells, function(m) m[[j]]$main, ""), vapply(cells, function(m) m[[j]]$st, "")))
      if (aux != "none") out <- c(out, row("", vapply(cells, function(m) m[[j]]$aux, "")))
    }
    out <- c(out, rule)
    if (length(srows)) {
      for (s in srows) out <- c(out, row(slabel(s), vapply(ms, scell, "", s = s)))
      out <- c(out, rule)
    }
    out <- c(out, nts)
  } else if (format == "csv") {
    q <- function(x) paste0("=\"", x, "\"")
    line <- function(xs) paste(q(xs), collapse = ",")
    out <- ""
    for (h in heads) out <- c(out, line(c("", h)))
    for (j in seq_along(all)) {
      out <- c(out, "")
      out <- c(out, line(c(lab(all[j]), vapply(cells, function(m) paste0(m[[j]]$main, m[[j]]$st), ""))))
      if (aux != "none") out <- c(out, line(c("", vapply(cells, function(m) m[[j]]$aux, ""))))
    }
    if (length(srows)) {
      out <- c(out, "")
      for (s in srows) out <- c(out, line(c(slabel(s), vapply(ms, scell, "", s = s))))
    }
    if (length(nts)) out <- c(out, "", vapply(nts, q, ""))
  } else {
    esc <- function(s) gsub("_", "\\\\_", s)
    texs <- function(s) if (nzchar(s)) stata_pad(paste0("\\sym{", s, "}"), -9) else strrep(" ", 9)
    sl <- function(s) if (s == "N" && !label) "\\(N\\)" else esc(slabel(s))
    nmc <- function(s) stata_pad(s, -vw)
    tcell <- function(x, s = "") paste0("&", stata_pad(x, 12), if (star) texs(s) else "")
    n <- length(ms)
    top <- if (booktabs) "\\toprule" else "\\hline\\hline"
    mid <- if (booktabs) "\\midrule" else "\\hline"
    bot <- if (booktabs) "\\bottomrule" else "\\hline\\hline"
    gap <- if (booktabs) "\\addlinespace" else "[1em]"
    # a title makes it a table with a caption
    open <- if (is.null(title)) "{" else "\\begin{table}[htbp]\\centering"
    out <- c(open, if (star) "\\def\\sym#1{\\ifmmode^{#1}\\else\\(^{#1}\\)\\fi}",
             if (!is.null(title)) paste0("\\caption{", title, "}"), sprintf("\\begin{tabular}{l*{%d}{c}}", n), top)
    for (h in heads) out <- c(out, paste0(nmc(""), paste0("&\\multicolumn{1}{c}{", esc(h), "}", collapse = ""), "\\\\"))
    out <- c(out, mid)
    for (j in seq_along(all)) {
      if (j > 1) out <- c(out, gap)
      out <- c(out, paste0(nmc(esc(lab(all[j]))), paste0(mapply(tcell, vapply(cells, function(m) m[[j]]$main, ""), vapply(cells, function(m) m[[j]]$st, "")), collapse = ""), "\\\\"))
      if (aux != "none") out <- c(out, paste0(nmc(""), paste0(vapply(vapply(cells, function(m) m[[j]]$aux, ""), tcell, ""), collapse = ""), "\\\\"))
    }
    out <- c(out, mid)
    if (length(srows)) {
      for (s in srows) out <- c(out, paste0(nmc(sl(s)), paste0(vapply(vapply(ms, scell, "", s = s), tcell, ""), collapse = ""), "\\\\"))
      out <- c(out, bot)
    }
    texnote <- function(s) {
      s <- sub("^t statistics", "\\\\textit{t} statistics", s)
      if (star && identical(s, note2) && is.null(star_text)) {
        s <- paste(paste0("\\sym{", names(stars), "} \\(p<", format(unname(stars), scientific = FALSE, drop0trailing = TRUE), "\\)"), collapse = ", ")
      }
      sprintf("\\multicolumn{%d}{l}{\\footnotesize %s}\\\\", n + 1, s)
    }
    out <- c(out, vapply(nts, texnote, ""), "\\end{tabular}", if (is.null(title)) "}" else "\\end{table}")
  }
  if (is.null(file)) {
    cat(out, sep = "\n")
    cat("\n")
  } else {
    if (file.exists(file) && !replace) stop("file ", file, " already exists")
    if (!file.exists(file) && replace) cat("(file ", file, " not found)\n", sep = "")
    con <- file(file, "wb")
    writeBin(charToRaw(paste0(paste(out, collapse = "\r\n"), "\r\n")), con)
    close(con)
    cat("(output written to ", file, ")\n", sep = "")
  }
  invisible(out)
}"##;

impl Session {
    /// `esttab [names] [using file] [, options]`.
    pub(super) fn esttab(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let known = [
            ("se", 2),
            ("t", 1),
            ("p", 1),
            ("not", 3),
            ("b", 1),
            ("star", 4),
            ("nostar", 6),
            ("label", 3),
            ("title", 5),
            ("mtitles", 7),
            ("nomtitles", 9),
            ("nonumbers", 9),
            ("nonotes", 7),
            ("addnotes", 8),
            ("scalars", 7),
            ("r2", 2),
            ("ar2", 3),
            ("pr2", 3),
            ("aic", 3),
            ("bic", 3),
            ("obslast", 7),
            ("noobs", 5),
            ("keep", 4),
            ("drop", 4),
            ("order", 5),
            ("nobaselevels", 12),
            ("noconstant", 10),
            ("modelwidth", 10),
            ("varwidth", 8),
            ("booktabs", 8),
            ("tex", 3),
            ("csv", 3),
            ("replace", 7),
        ];
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "esttab with the option {o} is not translated yet"
            )));
        }
        // the models: names, else those eststo stored, else the active ones
        let mut names: Vec<String> = c
            .parts
            .main
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if names.is_empty() {
            names = self
                .eststo_names
                .iter()
                .filter(|n| self.stored_fits.contains_key(*n))
                .cloned()
                .collect();
        }
        if names.is_empty() {
            if matches!(self.e_ret, super::returned::Returned::Nothing) {
                return Err(no(
                    "esttab: no model is fitted or stored before it in this do-file",
                ));
            }
            names.push(".".into());
        }
        for n in &names {
            if n != "." && !self.stored_fits.contains_key(n) {
                return Err(no(format!(
                    "esttab: {n} is not stored before it in this do-file (eststo or estimates store)"
                )));
            }
        }
        let fmt_arg = |name: &str, m: usize| -> Option<String> {
            opts.arg(name, m).map(|a| {
                let a = a.trim().to_string();
                a.split(',').next().unwrap_or("").trim().to_string()
            })
        };
        let (aux, auxfmt) = if opts.has("not", 3) {
            ("none", None)
        } else if opts.has("se", 2) {
            ("se", fmt_arg("se", 2))
        } else if opts.has("p", 1) {
            ("p", fmt_arg("p", 1))
        } else {
            ("t", fmt_arg("t", 1))
        };
        let file = match &c.using {
            Some(f) => {
                let f = f.trim().trim_matches('"').replace('\\', "/");
                let ext = f.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
                let named = if f.contains('.') {
                    f.clone()
                } else {
                    format!("{f}.txt")
                };
                let kind = if opts.has("tex", 3) || ext == "tex" {
                    "tex"
                } else if opts.has("csv", 3) || ext == "csv" {
                    "csv"
                } else if ext == "txt" || !f.contains('.') {
                    "txt"
                } else {
                    return Err(no(format!(
                        "esttab using a .{ext} file is not translated yet (.txt, .csv and .tex are)"
                    )));
                };
                Some((named, kind))
            }
            None if opts.has("tex", 3) || opts.has("csv", 3) => {
                return Err(no(
                    "esttab with tex or csv on the screen is not translated yet",
                ))
            }
            None => None,
        };
        if file.is_some() && opts.has("not", 3) {
            return Err(no("esttab using a file with not is not translated yet"));
        }
        let words = |name: &str, m: usize| -> Vec<String> {
            opts.arg(name, m)
                .map(|a| quoted_words(&a))
                .unwrap_or_default()
        };
        let mut stats: Vec<String> = Vec::new();
        let mut statfmt: Vec<String> = Vec::new();
        for (o, m) in [("r2", 2), ("ar2", 3), ("pr2", 3), ("aic", 3), ("bic", 3)] {
            if opts.has(o, m) {
                stats.push(r_text(o));
                if let Some(f) = fmt_arg(o, m).filter(|f| !f.is_empty()) {
                    statfmt.push(format!("{o} = {}", r_text(&f)));
                }
            }
        }
        for s in words("scalars", 7) {
            if !is_name(&s) {
                return Err(no(format!(
                    "esttab scalars({s}): names are translated, not labels or formats"
                )));
            }
            stats.push(r_text(&s));
        }
        let (star, stars, star_text) = if opts.has("nostar", 6) {
            (false, "NULL".to_string(), "NULL".to_string())
        } else if let Some(s) = opts.arg("star", 4) {
            let w: Vec<&str> = s.split_whitespace().collect();
            if w.len() % 2 != 0 || w.is_empty() {
                return Err(no(format!(
                    "star({s}): symbol and level pairs are translated"
                )));
            }
            let mut pairs = Vec::new();
            let mut text = Vec::new();
            for p in w.chunks(2) {
                let [sym, lev] = p else {
                    return Err(no(format!(
                        "star({s}): symbol and level pairs are translated"
                    )));
                };
                let (sym, lev) = (*sym, *lev);
                let v: f64 = lev
                    .parse()
                    .map_err(|_| no(format!("star({s}): {lev} is not a level")))?;
                pairs.push(format!("{} = {}", r_text(sym), r_number(v)));
                text.push(format!("{sym} p<{lev}"));
            }
            (
                true,
                format!("c({})", pairs.join(", ")),
                r_text(&text.join(", ")),
            )
        } else {
            (
                true,
                "c(\"*\" = 0.05, \"**\" = 0.01, \"***\" = 0.001)".to_string(),
                "NULL".to_string(),
            )
        };
        let mut drop = words("drop", 4);
        if opts.has("noconstant", 10) {
            drop.push("_cons".into());
        }
        let list = |v: Vec<String>| -> String {
            format!(
                "c({})",
                v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
            )
        };
        let num = |name: &str, m: usize| -> Result<String, Untranslated> {
            match opts.arg(name, m) {
                None => Ok("NULL".into()),
                Some(a) => a
                    .trim()
                    .parse::<f64>()
                    .map(r_number)
                    .map_err(|_| no(format!("{name}({a}) needs a number"))),
            }
        };
        let mtitles = words("mtitles", 7);
        let title = opts
            .arg("title", 5)
            .map(|t| t.trim().trim_matches('"').to_string());
        let d = self.data()?;
        self.helpers.insert("stata_esttab");
        self.helpers.insert("stata_graph");
        let call = format!(
            "stata_esttab(c({}), {d}, aux = {}, bfmt = {}, auxfmt = {}, star = {}, stars = {stars}, star_text = {star_text}, \
             label = {}, nobase = {}, keep = {}, drop = {}, order = {}, stats = c({}), statfmt = list({}), noobs = {}, \
             obslast = {}, numbers = {}, mtitles = {}, nomtitles = {}, title = {}, notes = {}, addnotes = {}, \
             varwidth = {}, modelwidth = {}, format = {}, booktabs = {}, file = {}, replace = {})",
            names.iter().map(|n| r_text(n)).collect::<Vec<_>>().join(", "),
            r_text(aux),
            r_text(&fmt_arg("b", 1).filter(|f| !f.is_empty()).unwrap_or_else(|| "a3".into())),
            auxfmt.filter(|f| !f.is_empty()).map(|f| r_text(&f)).unwrap_or_else(|| "NULL".into()),
            super::stats::r_logical(star),
            super::stats::r_logical(opts.has("label", 3)),
            super::stats::r_logical(opts.has("nobaselevels", 12)),
            list(words("keep", 4)),
            list(drop),
            list(words("order", 5)),
            stats.join(", "),
            statfmt.join(", "),
            super::stats::r_logical(opts.has("noobs", 5)),
            super::stats::r_logical(opts.has("obslast", 7)),
            super::stats::r_logical(!opts.has("nonumbers", 9)),
            if mtitles.is_empty() { "NULL".to_string() } else { list(mtitles) },
            super::stats::r_logical(opts.has("nomtitles", 9)),
            title.map(|t| r_text(&t)).unwrap_or_else(|| "NULL".into()),
            super::stats::r_logical(!opts.has("nonotes", 7)),
            list(words("addnotes", 8)),
            num("varwidth", 8)?,
            match opts.arg("modelwidth", 10) {
                None => "12".to_string(),
                Some(_) => num("modelwidth", 10)?,
            },
            r_text(file.as_ref().map(|(_, k)| *k).unwrap_or("screen")),
            super::stats::r_logical(opts.has("booktabs", 8)),
            file.as_ref().map(|(f, _)| r_text(f)).unwrap_or_else(|| "NULL".into()),
            super::stats::r_logical(opts.has("replace", 7)),
        );
        if file.is_none() && self.quiet(c) {
            return Ok(());
        }
        out.line(if file.is_some() {
            super::returned::quietly(c, call)
        } else {
            call
        });
        out.caveat("esttab as the estout package prints it, matched to its output (not its code)");
        Ok(())
    }
}

/// Words split at spaces outside quotes, the quotes taken off.
fn quoted_words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = false;
    for ch in text.chars() {
        match ch {
            '"' => quote = !quote,
            c if c.is_whitespace() && !quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
