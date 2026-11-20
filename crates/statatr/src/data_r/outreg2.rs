//! P2C.9 continued, `outreg2` (the SSC package), as the replication
//! packages run it: `outreg2 using f.xls, tstat bdec(#) tdec(#)
//! parenthesis(tstat) [adjr2 r2 keep() e() replace]`. Matched to the files
//! it writes on Stata 19 with made-up data (`plan/stata/outreg2.md`), never
//! its code.
//!
//! - It writes f.txt (cells by tabs) and, for .xls, Excel's XML spreadsheet
//!   with the same cells. Without `replace`, the model is a new column of
//!   the table already in f.txt.
//! - A coefficient has bdec decimals and thousands commas, with *, ** and
//!   *** at p < 0.1, 0.05 and 0.01; its t statistic tdec decimals, in
//!   parentheses. R-squared has 3 decimals; an e() statistic 3 significant
//!   digits (a count none).
//! - A variable a later model adds comes before Constant; the statistics
//!   are Observations, R-squared and Adjusted R-squared, then the e() ones in
//!   order of their names.

use super::*;

/// Writes the table (`stata_e` is the model).
pub(super) const OUTREG2_HELPER: &str = r##"stata_outreg2 <- function(file, bdec = NULL, tdec = NULL, sdec = NULL, keep = NULL, sortvar = NULL, r2 = FALSE, adjr2 = FALSE,
                          extras = character(), replace = FALSE, se = FALSE, tex = FALSE,
                          lab = FALSE, ctitle = NULL, addtext = character(), addstat = list(),
                          data = NULL, word = FALSE, texdoc = FALSE, nocons = FALSE, noobs = FALSE,
                          nor2 = FALSE, noaster = FALSE, alpha = c("0.01", "0.05", "0.1"),
                          symbol = c("***", "**", "*"), stats = NULL) {
  e <- stata_e
  nm <- colnames(e$b)
  # one equation (logit, probit, poisson): the names without it
  if (length(nm) && all(grepl(":", nm, fixed = TRUE)) && length(unique(sub(":.*$", "", nm))) == 1) nm <- sub("^[^:]*:", "", nm)
  b <- as.numeric(e$b)
  sev <- sqrt(pmax(diag(as.matrix(e$V)), 0))
  df <- if (!is.null(e$df_r) && is.finite(e$df_r)) e$df_r else Inf
  vars <- nm[nm != "_cons"]
  if (!is.null(keep)) vars <- vars[vars %in% keep]
  if ("_cons" %in% nm && !nocons) vars <- c(vars, "_cons")
  # outreg2's own number format (no bdec(), sdec() or tdec()), by the size of the number
  # (Stata 19: 4 significant digits from 1 up, 3 below, e-notation from 1e6 and below 1e-4)
  auto <- function(x) {
    if (is.na(x)) return(".")
    ax <- abs(x)
    if (ax < 5e-11) return(sprintf("%.0f", x))
    if (ax >= 1e6) return(sprintf("%.3e", x))
    if (ax < 1e-4) return(sprintf("%.2e", x))
    d <- if (ax >= 1000) 0 else if (ax >= 1) 3 - floor(log10(ax)) else 2 - floor(log10(ax))
    formatC(x, format = "f", digits = d, big.mark = ",")
  }
  num <- function(x, d) if (is.null(d)) auto(x) else formatC(x, format = "f", digits = d, big.mark = ",")
  sig_at <- as.numeric(alpha)
  stars <- function(p) {
    if (noaster) return("")
    for (i in seq_along(sig_at)) if (p < sig_at[i]) return(symbol[i])
    ""
  }
  # the lines under each coefficient: the standard error (the default) or the t statistic, or
  # what stats() names after coef: se, tstat, pval, ci
  kinds <- if (!is.null(stats)) stats[-1] else if (se) "se" else "tstat"
  paren <- if (length(kinds) == 3) c(TRUE, FALSE, TRUE) else rep(TRUE, length(kinds))
  sdec_ <- if (!is.null(sdec)) sdec else if (!is.null(bdec)) 3 else NULL
  coef <- list()
  for (v in vars) {
    i <- match(v, nm)
    if (is.na(b[i])) next
    # no standard error (e(V) zero there): the coefficient with (.), no stars (Stata 19)
    # an omitted term (o.x): "-" and an empty line under it
    if (grepl("^([0-9]+)?o[.]", v)) {
      coef[[v]] <- c("-", "")
      next
    }
    if (!(sev[i] > 0)) {
      coef[[if (v == "_cons") "Constant" else v]] <- c(num(b[i], bdec), rep("(.)", length(kinds)))
      next
    }
    t <- b[i] / sev[i]
    p <- 2 * (if (is.finite(df)) stats::pt(-abs(t), df) else stats::pnorm(-abs(t)))
    # with se, the standard error has 3 decimals whatever tdec() says (Stata 19)
    cell <- function(kind) switch(kind,
      se = num(sev[i], sdec_),
      tstat = num(t, tdec),
      pval = num(p, NULL),
      ci = {
        q <- if (is.finite(df)) stats::qt(0.975, df) else stats::qnorm(0.975)
        paste0(num(b[i] - q * sev[i], NULL), " - ", num(b[i] + q * sev[i], NULL))
      })
    lines <- vapply(seq_along(kinds), function(j) {
      v1 <- cell(kinds[j])
      if (paren[j]) paste0("(", v1, ")") else v1
    }, "")
    coef[[if (v == "_cons") "Constant" else v]] <- c(paste0(num(b[i], bdec), stars(p)), lines)
  }
  sig3 <- function(x) {
    if (x == round(x)) return(format(x, big.mark = ",", scientific = FALSE))
    # below 1e-4, 3 significant digits in e-notation, as C's %g (-2.58e-05)
    if (abs(x) < 1e-4) return(sprintf("%.2e", x))
    formatC(x, format = "f", digits = max(0, 2 - floor(log10(abs(x)))))
  }
  # the column's statistics: Observations, R-squared from the model, xtreg's
  # Number of panels, then e() ones, addtext() and addstat() (Stata 19's order)
  fixed <- c("Observations", "R-squared", "Adjusted R-squared")
  st <- list(Observations = format(e$N, big.mark = ",", scientific = FALSE))
  r2v <- e[["r2"]]
  if (is.null(r2v) && identical(e$cmd, "xtreg") && identical(e$model, "fe")) r2v <- e$r2_w
  if ((r2 || !adjr2) && !is.null(r2v) && is.finite(r2v)) st[["R-squared"]] <- formatC(r2v, format = "f", digits = 3)
  if (adjr2 && !is.null(e$r2_a)) st[["Adjusted R-squared"]] <- formatC(e$r2_a, format = "f", digits = 3)
  if (identical(e$cmd, "xtreg") && !is.null(e$N_g)) st[[paste("Number of", e$ivar)]] <- format(e$N_g, scientific = FALSE)
  if (noobs) st[["Observations"]] <- NULL
  if (nor2) st[c("R-squared", "Adjusted R-squared")] <- NULL
  natv <- names(st)
  rest <- character()
  for (x in sort(extras, method = "radix")) if (!is.null(e[[x]]) && !is.na(e[[x]])) { st[[x]] <- sig3(e[[x]]); rest <- c(rest, x) }
  if (length(addtext)) for (j in seq(1, length(addtext), by = 2)) { st[[addtext[j]]] <- addtext[j + 1]; rest <- c(rest, addtext[j]) }
  for (s in names(addstat)) {
    v <- addstat[[s]]
    st[[s]] <- if (is.numeric(v) && length(v) == 1 && !is.na(v)) sig3(v) else "."
    if (!s %in% natv) rest <- c(rest, s)
  }
  rest <- unique(rest)
  base <- sub("[.](xls|tex|rtf|doc)$", "", file, ignore.case = TRUE)
  txt <- if (identical(base, file)) paste0(file, ".txt") else paste0(base, ".txt")
  vlab <- function(v) {
    if (v == "Constant") return("Constant")
    if (grepl("#", v, fixed = TRUE)) return("")
    l <- if (!is.null(data) && v %in% names(data)) attr(data[[v]], "label", exact = TRUE) else NULL
    if (is.null(l) || !nzchar(l)) v else l
  }
  # the table so far: its columns' cells by variable and by statistic
  old_vars <- list()
  old_stats <- list()
  heads <- character()
  deps <- character()
  had_lab <- FALSE
  foot <- NULL
  if (!replace && file.exists(txt)) {
    rows <- lapply(sub("\r$", "", readLines(txt, warn = FALSE)), function(l) utils::head(strsplit(paste0(l, "\tX"), "\t", fixed = TRUE)[[1]], -1))
    had_lab <- length(rows) > 1 && length(rows[[2]]) > 1 && rows[[2]][2] == "LABELS"
    o <- if (had_lab) 1 else 0
    k <- length(rows[[1]]) - 1 - o
    cell <- function(r, j) { v <- rows[[r]][j + 1 + o]; if (is.na(v)) "" else v }
    heads <- vapply(seq_len(k), function(j) cell(1, j), "")
    deps <- vapply(seq_len(k), function(j) cell(2, j), "")
    r <- 4
    while (r < length(rows) && nzchar(rows[[r]][1])) {
      old_vars[[rows[[r]][1]]] <- rbind(vapply(seq_len(k), function(j) cell(r, j), ""), vapply(seq_len(k), function(j) cell(r + 1, j), ""))
      r <- r + 2
    }
    r <- r + 1
    while (r <= length(rows) - 2) {
      old_stats[[rows[[r]][1]]] <- vapply(seq_len(k), function(j) cell(r, j), "")
      r <- r + 1
    }
    foot <- rows[[length(rows) - 1]][1]
  }
  has_lab <- lab || had_lab
  robust <- identical(e$vcetype, "Robust")
  if (is.null(foot)) foot <- if (!is.null(stats)) {
    if (length(kinds) == 0 || identical(kinds, "se")) "Standard errors in parentheses"
    else if (identical(kinds, "tstat")) "t-statistics in parentheses"
    else paste0(paste(kinds[paren], collapse = " "), " in parentheses")
  } else if (!se) paste0(if (robust) "Robust " else "", "t-statistics in parentheses") else if (robust) "Robust standard errors in parentheses" else "Standard errors in parentheses"
  star_note <- if (noaster) NULL else paste0(symbol, " p<", alpha, collapse = ", ")
  k <- length(heads)
  # the rows: the table's own, a variable the model adds going before the next one the
  # two share (in the model's order), else last; an omitted term (o.x) last of all
  old <- setdiff(names(old_vars), "Constant")
  new <- setdiff(names(coef), "Constant")
  omitted <- grepl("^([0-9]+)?o[.]", new)
  var_order <- old
  shared <- new[!omitted & new %in% old]
  for (j in which(!omitted)) {
    v <- new[j]
    if (v %in% var_order) next
    after <- new[seq_along(new) > j & !omitted]
    nxt <- after[after %in% shared][1]
    var_order <- if (is.na(nxt)) c(var_order, v) else append(var_order, v, after = match(nxt, var_order) - 1)
  }
  var_order <- c(var_order, setdiff(new[omitted], var_order))
  if (!is.null(sortvar)) var_order <- c(intersect(sortvar, var_order), setdiff(var_order, sortvar))
  if ("Constant" %in% c(names(old_vars), names(coef))) var_order <- c(var_order, "Constant")
  # the statistics' rows. One column: the model's own order (xtreg fe: the
  # Number of panels before R-squared), then e(), addtext() and addstat(). A
  # column added to a table: Observations and the R-squareds first, then the
  # table's rows as they were, then this column's others (Stata 19)
  on <- as.character(names(old_stats))
  own <- c(setdiff(names(st), rest), rest)
  if (identical(e$cmd, "xtreg") && identical(e$model, "fe")) {
    nr <- startsWith(own, "Number of ")
    own <- c("Observations", own[nr], setdiff(own[!nr], "Observations"))
  }
  # e() statistics are sorted by name across the columns (Stata 19); a table's
  # row is one when its name is an e() name (lowercase, or with an underscore)
  is_e <- function(x) grepl("^[A-Za-z][A-Za-z0-9_]*$", x) & (grepl("_", x, fixed = TRUE) | x == tolower(x))
  stat_order <- if (!length(on)) own else {
    all <- unique(c(on, own))
    number <- all[startsWith(all, "Number of ")]
    enames <- sort(unique(c(on[is_e(on) & !on %in% c(fixed, number)], intersect(extras, names(st)))), method = "radix")
    c(intersect(fixed, all), number, enames, setdiff(all, c(fixed, number, enames)))
  }
  dep <- if (!is.null(ctitle)) ctitle else if (lab) vlab(e$depvar) else e$depvar
  lc <- function(x) if (has_lab) x else character()
  blank <- rep("", k + 2 + has_lab)
  out <- list(c("", lc(""), heads, sprintf("(%d)", k + 1)), c("VARIABLES", lc("LABELS"), deps, dep), blank)
  # stats() leaves a row of the dependent variable's name above the variables, as tall as a variable
  m <- 1 + length(kinds)
  if (!is.null(stats) && !identical(stats, c("coef", "se"))) {
    out <- c(out, list(c(e$depvar, rep("", k + 1 + has_lab))))
    for (j in seq_len(m - 1)) out <- c(out, list(c("", lc(""), rep("", k), if (identical(kinds[j], "ci")) "( - )" else "")))
  }
  for (v in var_order) {
    o <- if (is.null(old_vars[[v]])) matrix("", m, k) else old_vars[[v]]
    n <- if (is.null(coef[[v]])) rep("", m) else coef[[v]]
    n <- c(n, rep("", m - length(n)))
    for (j in seq_len(m)) out <- c(out, list(c(if (j == 1) v else "", lc(if (j == 1) vlab(v) else ""), o[j, ], n[j])))
  }
  out <- c(out, list(blank))
  for (s in stat_order) {
    o <- if (is.null(old_stats[[s]])) rep("", k) else old_stats[[s]]
    out <- c(out, list(c(s, lc(""), o, if (is.null(st[[s]])) "" else st[[s]])))
  }
  nn <- if (is.null(star_note)) 1 else 2
  out <- c(out, list(c(foot, rep("", k + 1 + has_lab))))
  if (!is.null(star_note)) out <- c(out, list(c(star_note, rep("", k + 1 + has_lab))))
  eol <- if (.Platform$OS.type == "windows") "\r\n" else "\n"
  con <- file(txt, "wb")
  writeChar(paste0(vapply(out, paste, "", collapse = "\t"), eol, collapse = ""), con, eos = NULL)
  close(con)
  if (tex) {
    esc <- function(s) gsub("#", "\\#", gsub("%", "\\%", gsub("_", "\\_", s, fixed = TRUE), fixed = TRUE), fixed = TRUE)
    nc <- k + 1
    drop_lab <- function(r) if (has_lab) r[-2] else r
    body <- character()
    nv <- length(var_order)
    for (r in seq_along(out)) {
      row <- out[[r]]
      if (r > length(out) - 2) break
      cells <- drop_lab(row)
      # the variable's label where there is one (lab), its name otherwise
      if (has_lab && r > 3 && r <= 3 + 2 * nv && nzchar(row[1])) cells[1] <- if (nzchar(row[2])) row[2] else row[1]
      cells[1] <- esc(cells[1])
      line <- paste(cells, collapse = " & ")
      if (r == 2) line <- paste(line, "\\\\ \\hline") else if (r == length(out) - 2) line <- paste0(" ", line, " \\\\ \\hline") else line <- paste(line, "\\\\")
      body <- c(body, line)
    }
    texl <- c(paste0("\\begin{tabular}{l", strrep("c", nc), "} \\hline"), body,
              sprintf("\\multicolumn{%d}{c}{ %s} \\\\", nc + 1, foot),
              sprintf("\\multicolumn{%d}{c}{ *** p$<$0.01, ** p$<$0.05, * p$<$0.1} \\\\", nc + 1),
              "\\end{tabular}")
    if (texdoc) texl <- c("\\documentclass[]{article}", "\\setlength{\\pdfpagewidth}{8.5in} \\setlength{\\pdfpageheight}{11in}",
                          "\\begin{document}", texl, "\\end{document}")
    con <- file(file, "wb")
    writeChar(paste0(texl, eol, collapse = ""), con, eos = NULL)
    close(con)
  }
  if (word) {
    # Rich Text: a table row per line of the .txt, the first row and the VARIABLES row ruled
    # above and below, the last row below; cell widths 144 twips per character (plus one)
    rows <- out[seq_len(length(out) - nn)]
    nr <- length(rows)
    wid <- vapply(seq_len(k + 2), function(j) {
      from <- if (j == 1) 3 else 1
      144 * (1 + max(nchar(vapply(rows[from:nr], function(r) r[j], ""))))
    }, 0)
    cx <- cumsum(wid)
    head <- "\\trowd\\trgaph75\\trleft-75"
    cellx <- function(border) paste0(vapply(seq_along(cx), function(j) paste0(border, "\\cellx", cx[j]), ""), collapse = "")
    row <- function(r) paste0("\\ql ", r[1], "\\cell", paste0("\\qc ", r[-1], "\\cell", collapse = ""), "\\row")
    rtf <- c("{\\rtf1\\fs24",
             paste0(head, "\\intbl\\trqc", cellx("\\clbrdrt\\brdrs")), row(rows[[1]]),
             paste0(head, "\\trqc", cellx("")),
             paste0(head, "\\trqc", cellx("\\clbrdrb\\brdrs")), row(rows[[2]]),
             paste0(head, "\\trqc", cellx("")))
    for (r in 3:(nr - 1)) rtf <- c(rtf, row(rows[[r]]))
    rtf <- c(rtf, paste0(head, "\\trqc\\trbrdrb\\brdrs", cellx("\\clbrdrb\\brdrs")), row(rows[[nr]]), "\\pard\\qc",
             paste0(vapply(out[(nr + 1):length(out)], function(r) r[1], ""), "\\par"))
    con <- file(file, "wb")
    writeChar(paste0(paste0(rtf, eol, collapse = ""), "}"), con, eos = NULL)
    close(con)
  }
  if (grepl("[.]xls$", file, ignore.case = TRUE)) {
    esc <- function(s) gsub(">", "&gt;", gsub("<", "&lt;", gsub("&", "&amp;", s, fixed = TRUE), fixed = TRUE), fixed = TRUE)
    n <- length(out)
    cells <- function(r, a, b) paste0(vapply(seq_along(out[[r]]), function(j) sprintf("<Cell ss:StyleID=\"%s\"><Data ss:Type=\"String\">%s</Data></Cell>", if (j == 1) a else b, esc(out[[r]][j])), ""), collapse = "")
    body <- vapply(seq_len(n), function(r) {
      if (r > n - 2) return(sprintf("<Row><Cell ss:StyleID=\"s24\"><Data ss:Type=\"String\">%s</Data></Cell></Row>", esc(out[[r]][1])))
      style <- if (r %in% c(1, 3)) c("s31", "s21") else if (r == n - 2) c("s33", "s23") else c("s32", "s22")
      paste0("<Row>", cells(r, style[1], style[2]), "</Row>")
    }, "")
    w1 <- 6 * max(nchar(vapply(out[seq_len(n - 2)], function(r) r[1], "")))
    xml <- paste0(r"---(<?xml version="1.0" encoding="US-ASCII" standalone="yes"?><?mso-application progid="Excel.Sheet"?><Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:x="urn:schemas-microsoft-com:office:excel" xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet" xmlns:html="http://www.w3.org/TR/REC-html40"><DocumentProperties xmlns="urn:schemas-microsoft-com:office:office"><Author></Author><LastAuthor></LastAuthor><Created></Created><LastSaved></LastSaved><Company></Company><Version></Version></DocumentProperties><ExcelWorkbook  xmlns="urn:schemas-microsoft-com:office:excel"><ProtectStructure>False</ProtectStructure><ProtectWindows>False</ProtectWindows></ExcelWorkbook><Styles><Style ss:ID="Default" ss:Name="Normal"><Alignment ss:Vertical="Bottom"/><Borders/><Font ss:FontName="Calibri" ss:Size="10"/><Interior/><NumberFormat/><Protection/></Style><Style ss:ID="s1"><Font ss:FontName="Calibri" ss:Bold="1" ss:Size="12"/><NumberFormat/></Style><Style ss:ID="s21"><Alignment ss:Horizontal="Center" ss:Vertical="Bottom"/><NumberFormat/><Font ss:FontName="Calibri" ss:Size="10"/><Borders><Border ss:Position="Top" ss:LineStyle="Continuous" ss:Weight="1"/></Borders></Style><Style ss:ID="s22"><Alignment ss:Horizontal="Center" ss:Vertical="Bottom"/><Font ss:FontName="Calibri" ss:Size="10"/><NumberFormat/></Style><Style ss:ID="s23"><Alignment ss:Horizontal="Center" ss:Vertical="Bottom"/><NumberFormat/><Font ss:FontName="Calibri" ss:Size="10"/><Borders><Border ss:Position="Bottom" ss:LineStyle="Continuous" ss:Weight="1"/></Borders></Style><Style ss:ID="s24"><Font ss:FontName="Calibri" ss:Size="10"/><NumberFormat/></Style><Style ss:ID="s31"><Font ss:FontName="Calibri" ss:Size="10"/><Borders><Border ss:Position="Top" ss:LineStyle="Continuous" ss:Weight="1"/></Borders></Style><Style ss:ID="s32"><Font ss:FontName="Calibri" ss:Size="10"/><Borders/></Style><Style ss:ID="s33"><Font ss:FontName="Calibri" ss:Size="10"/><Borders><Border ss:Position="Bottom" ss:LineStyle="Continuous" ss:Weight="1"/></Borders></Style></Styles><Names></Names>)---",
      "<Worksheet ss:Name=\"Sheet1\"><Table ss:ExpandedColumnCount=\"", k + 2, "\" ss:ExpandedRowCount=\"", n + 1,
      "\" x:FullColumns=\"1\" x:FullRows=\"1\"><Column ss:AutoFitWidth=\"0\" ss:Width=\"", w1, "\"/>",
      strrep("<Column ss:AutoFitWidth=\"0\" ss:Width=\"58\"/>", k + 1), "<Row></Row>", paste0(body, collapse = ""),
      "</Table><WorksheetOptions xmlns=\"urn:schemas-microsoft-com:office:excel\"><ProtectedObjects>False</ProtectedObjects><ProtectedScenarios>False</ProtectedScenarios></WorksheetOptions></Worksheet></Workbook>")
    con <- file(file, "wb")
    writeChar(xml, con, eos = NULL)
    close(con)
  }
  # Stata 19 names the file it wrote when it is not the plain table, and always ends with this line
  cat(if (tex || word || grepl("[.]xls$", file, ignore.case = TRUE)) paste0(basename(file), "\n"), "dir : seeout\n", sep = "")
  invisible(NULL)
}"##;

impl Session {
    /// `outreg2 using file, (tstat | se) bdec(#) tdec(#) [parenthesis(tstat)
    /// adjr2 r2 keep(vars) e(names) replace append tex(fragment) label
    /// ctitle(text) addtext(name, text, ...) addstat(name, exp, ...)]` after
    /// a model.
    pub(super) fn outreg2(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("tstat", 5),
            ("se", 2),
            ("bdec", 4),
            ("tdec", 4),
            ("sdec", 4),
            ("parenthesis", 5),
            ("adjr2", 5),
            ("r2", 2),
            ("keep", 4),
            ("sortvar", 4),
            ("e", 1),
            ("replace", 7),
            ("append", 6),
            ("tex", 3),
            ("word", 4),
            ("label", 3),
            ("ctitle", 2),
            ("addtext", 4),
            ("addstat", 4),
            ("onecol", 6),
            ("nocons", 6),
            ("noobs", 5),
            ("nor2", 4),
            ("noaster", 7),
            ("alpha", 5),
            ("symbol", 6),
            ("stats", 5),
        ]) {
            return Err(no(format!(
                "outreg2 with the option {o} is not translated yet"
            )));
        }
        if !c.parts.main.trim().is_empty() {
            return Err(no(
                "outreg2 with a list of stored models is not translated yet",
            ));
        }
        let se = opts.has("se", 2);
        let tstat = opts.has("tstat", 5);
        if se && tstat {
            return Err(no("outreg2 with both se and tstat is not translated yet"));
        }
        let replace = opts.has("replace", 7);
        // stats(coef ...): the lines under each coefficient (Stata 19 stops with r(198) when
        // tstat or se is given too)
        let stats: Option<Vec<String>> = match opts.arg("stats", 5) {
            None => None,
            Some(s) => {
                let words: Vec<String> = s.split_whitespace().map(str::to_lowercase).collect();
                let ok = matches!(
                    words
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .as_slice(),
                    ["coef"]
                        | ["coef", "se"]
                        | ["coef", "tstat"]
                        | ["coef", "pval"]
                        | ["coef", "ci"]
                        | ["coef", "se", "tstat", "pval"]
                );
                if !ok || se || tstat {
                    return Err(no(format!(
                        "outreg2, stats({s}) is not translated yet (coef, coef se, coef with tstat, pval or ci, and coef se tstat pval are)"
                    )));
                }
                Some(words)
            }
        };
        let new_layout = opts.has("noobs", 5)
            || opts.has("nor2", 4)
            || opts.has("noaster", 7)
            || opts.has("alpha", 5)
            || opts.has("symbol", 6)
            || stats.is_some();
        if (new_layout || opts.has("nocons", 6)) && !replace {
            return Err(no(
                "outreg2 with nocons, noobs, nor2, noaster, alpha(), symbol() or stats() and no replace is not translated yet",
            ));
        }
        if let Some(p) = opts.arg("parenthesis", 5) {
            let want = if se { "se" } else { "tstat" };
            if p.trim() != want || (!se && !tstat) {
                return Err(no(format!(
                    "outreg2, parenthesis({p}) is not translated yet"
                )));
            }
        }
        let digits = |name: &str| -> Result<Option<String>, Untranslated> {
            let Some(v) = opts.arg(name, 4) else {
                return Ok(None);
            };
            match v.trim().parse::<u32>() {
                Ok(d) if d <= 10 => Ok(Some(d.to_string())),
                _ => Err(no(format!(
                    "{name}({v}): only a number of decimals is translated"
                ))),
            }
        };
        let bdec = digits("bdec")?;
        let tdec = digits("tdec")?;
        let sdec = digits("sdec")?;
        if stats.is_some() && (bdec.is_some() || tdec.is_some() || sdec.is_some()) {
            return Err(no(
                "outreg2 with stats() and bdec(), sdec() or tdec() is not translated yet",
            ));
        }
        // the symbols and the levels they stand for (Stata 19: the first symbol is for the
        // smallest level; the levels are shown as typed)
        let alpha: Option<Vec<String>> = opts
            .arg("alpha", 5)
            .map(|a| a.split(',').map(|x| x.trim().to_string()).collect());
        let symbol: Option<Vec<String>> = opts
            .arg("symbol", 6)
            .map(|a| a.split(',').map(|x| x.trim().to_string()).collect());
        if alpha.is_some() != symbol.is_some() {
            return Err(no(
                "outreg2 with alpha() and no symbol(), or the reverse, is not translated yet",
            ));
        }
        if let (Some(a), Some(s)) = (&alpha, &symbol) {
            let levels: Vec<Option<f64>> = a
                .iter()
                .map(|x| {
                    x.strip_prefix("0.")
                        .filter(|r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()))
                        .and_then(|_| x.parse::<f64>().ok())
                })
                .collect();
            let ascending = levels.windows(2).all(|w| match (w.first(), w.get(1)) {
                (Some(Some(p)), Some(Some(q))) => p < q,
                _ => false,
            });
            if a.len() != s.len()
                || a.is_empty()
                || a.len() > 3
                || levels.iter().any(Option::is_none)
                || !ascending
                || s.iter().any(|x| x.is_empty() || x.contains('"'))
            {
                return Err(no(
                    "outreg2 with alpha() and symbol() other than up to three levels like 0.01, 0.05, in ascending order, with as many symbols, is not translated yet",
                ));
            }
        }
        let file = c
            .using
            .clone()
            .ok_or_else(|| no("outreg2 needs using file"))?;
        let mut path = file.trim().trim_matches('"').replace('\\', "/");
        let ext = std::path::Path::new(&path)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());
        let word = opts.has("word", 4);
        let tex_arg = opts.arg("tex", 3);
        if word && tex_arg.is_some() {
            return Err(no("outreg2 with both word and tex is not translated yet"));
        }
        // a .txt name is the table's file as it stands
        let ext = if ext.as_deref() == Some("txt") && !word && tex_arg.is_none() {
            path.truncate(path.len().saturating_sub(4));
            None
        } else {
            ext
        };
        if word && !matches!(ext.as_deref(), None | Some("rtf") | Some("doc")) {
            return Err(no(
                "outreg2, word with a file name that has an extension other than .rtf or .doc is not translated yet",
            ));
        }
        if word && opts.has("label", 3) {
            return Err(no("outreg2, word with label is not translated yet"));
        }
        let (tex, texdoc) = match tex_arg.as_deref().map(str::trim) {
            Some("fragment" | "frag") if matches!(ext.as_deref(), Some("tex")) => (true, false),
            // tex alone: a whole document, in a file that gets .tex when it has no extension
            Some("") if matches!(ext.as_deref(), None | Some("tex")) => (true, true),
            Some(t) => {
                return Err(no(format!(
                    "outreg2, tex({t}) is not translated yet (tex(fragment) with a .tex file, and tex alone, are)"
                )))
            }
            None => (false, false),
        };
        if (tex || word) && ext.is_none() {
            path.push_str(if tex { ".tex" } else { ".rtf" });
        }
        let ext = if word {
            Some(ext.unwrap_or_else(|| "rtf".to_string()))
        } else if tex {
            Some("tex".to_string())
        } else {
            ext
        };
        // the new layout options were observed on a table in a .txt or .rtf only (the .xls and
        // .tex rows depend on the number of notes under the table)
        if new_layout && !(word || (!tex && ext.is_none())) {
            return Err(no(
                "outreg2 with noobs, nor2, noaster, alpha(), symbol() or stats() and an .xls or .tex file is not translated yet (a .txt or word is)",
            ));
        }
        if !tex && !word && !matches!(ext.as_deref(), None | Some("xls")) {
            return Err(no(format!(
                "outreg2 using a .{} file is not translated yet (.xls, .txt, .rtf and .tex are)",
                ext.unwrap_or_default()
            )));
        }
        let lab = opts.has("label", 3);
        if lab && ext.as_deref() == Some("xls") {
            return Err(no("outreg2, label with an .xls file is not translated yet"));
        }
        let Some(est) = self.estimates.as_ref() else {
            return Err(no("outreg2: no model is fitted before it in this do-file"));
        };
        // onecol: one column for all equations; after a model of one equation
        // Stata 19 writes the same files with it or without (plan/stata/outreg2.md)
        if opts.has("onecol", 6)
            && !matches!(
                est.fit,
                model::Fit::Regress | model::Fit::Xtreg | model::Fit::Reghdfe
            )
        {
            return Err(no(format!(
                "outreg2 with the option onecol after {} is not translated yet",
                est.fit.name()
            )));
        }
        // keep(): a##b is a, b and a#b; factor variables only when keep()
        // leaves their columns out
        let keep: Option<Vec<String>> = opts.arg("keep", 4).map(|k| {
            let mut names = Vec::new();
            for w in k.split_whitespace() {
                if let Some((a, b)) = w.split_once("##") {
                    for part in [a, b] {
                        names.push(part.trim_start_matches("c.").to_string());
                    }
                    names.push(format!("{a}#{b}"));
                } else {
                    names.push(w.to_string());
                }
            }
            names
        });
        if !est.factors.is_empty() {
            let factor_kept = |w: &str| {
                w.contains("i.")
                    || w.contains("b.")
                    || w.split('#')
                        .any(|p| est.factors.contains_key(p.trim_start_matches("i.")))
            };
            match &keep {
                Some(k) if !k.iter().any(|w| factor_kept(w)) => {}
                _ => {
                    return Err(no(
                        "outreg2 after a model with factor variables is not translated yet, unless keep() leaves them out",
                    ))
                }
            }
        }
        let mut args = r_text(&path);
        for (name, v) in [("bdec", &bdec), ("tdec", &tdec), ("sdec", &sdec)] {
            if let Some(v) = v {
                args.push_str(&format!(", {name} = {v}"));
            }
        }
        let flag = |name: &str, min: usize, arg: &str| {
            opts.has(name, min).then(|| format!(", {arg} = TRUE"))
        };
        for extra in [
            flag("nocons", 6, "nocons"),
            flag("noobs", 5, "noobs"),
            flag("nor2", 4, "nor2"),
            flag("noaster", 7, "noaster"),
        ]
        .into_iter()
        .flatten()
        {
            args.push_str(&extra);
        }
        if let (Some(a), Some(s)) = (&alpha, &symbol) {
            let q = |v: &Vec<String>| v.iter().map(|x| r_text(x)).collect::<Vec<_>>().join(", ");
            args.push_str(&format!(", alpha = c({}), symbol = c({})", q(a), q(s)));
        }
        if let Some(s) = &stats {
            let q: Vec<String> = s.iter().map(|x| r_text(x)).collect();
            args.push_str(&format!(", stats = c({})", q.join(", ")));
        }
        if let Some(k) = &keep {
            let names: Vec<String> = k.iter().map(|s| r_text(s)).collect();
            args.push_str(&format!(", keep = c({})", names.join(", ")));
        }
        // sortvar(): those variables first, in that order, then the rest
        if let Some(s) = opts.arg("sortvar", 4) {
            let names: Vec<String> = s.split_whitespace().map(r_text).collect();
            if !names.is_empty() {
                args.push_str(&format!(", sortvar = c({})", names.join(", ")));
            }
        }
        if opts.has("r2", 2) {
            args.push_str(", r2 = TRUE");
        }
        if opts.has("adjr2", 5) {
            args.push_str(", adjr2 = TRUE");
        }
        if let Some(x) = opts.arg("e", 1) {
            let names: Vec<String> = x.split_whitespace().map(r_text).collect();
            args.push_str(&format!(", extras = c({})", names.join(", ")));
        }
        if opts.has("replace", 7) {
            args.push_str(", replace = TRUE");
        }
        if se || (!tstat && stats.is_none()) {
            args.push_str(", se = TRUE");
        }
        if tex {
            args.push_str(if texdoc {
                ", tex = TRUE, texdoc = TRUE"
            } else {
                ", tex = TRUE"
            });
        }
        if word {
            args.push_str(", word = TRUE");
        }
        if lab {
            args.push_str(", lab = TRUE");
        }
        if let Some(t) = opts.arg("ctitle", 2) {
            args.push_str(&format!(", ctitle = {}", r_text(t.trim())));
        }
        let pairs = |text: &str, what: &str| -> Result<Vec<String>, Untranslated> {
            let items = comma_items(text);
            if items.len() % 2 != 0 || items.iter().any(String::is_empty) {
                return Err(no(format!(
                    "outreg2, {what}() needs pairs of a name and a value"
                )));
            }
            Ok(items)
        };
        if let Some(t) = opts.arg("addtext", 4) {
            let items = pairs(&t, "addtext")?;
            let texts: Vec<String> = items.iter().map(|s| r_text(s)).collect();
            args.push_str(&format!(", addtext = c({})", texts.join(", ")));
        }
        if let Some(t) = opts.arg("addstat", 4) {
            let items = pairs(&t, "addstat")?;
            let mut names = Vec::new();
            let mut codes = Vec::new();
            for p in items.chunks(2) {
                let [name, exp] = p else { continue };
                let (code, _) = self.display_value(exp, out)?;
                names.push(r_text(name));
                codes.push(code);
            }
            args.push_str(&format!(
                ", addstat = stats::setNames(list({}), c({}))",
                codes.join(", "),
                names.join(", ")
            ));
        }
        if lab {
            let d = self.data()?;
            args.push_str(&format!(", data = {d}"));
        }
        self.helpers.insert("stata_outreg2");
        out.line(format!("stata_outreg2({args})"));
        out.caveat(
            "outreg2 as the SSC package writes its .txt, .xls and .tex, matched to the files it writes \
             (not its code)",
        );
        Ok(())
    }
}

/// The items of `a, b, c`, split at the commas outside parentheses and
/// quotes, each trimmed.
fn comma_items(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut cur = String::new();
    for ch in text.chars() {
        match ch {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth = depth.saturating_sub(1),
            ',' if !quoted && depth == 0 => {
                items.push(cur.trim().to_string());
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    items.push(cur.trim().to_string());
    items
}
