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
pub(super) const OUTREG2_HELPER: &str = r##"stata_outreg2 <- function(file, bdec, tdec, keep = NULL, r2 = FALSE, adjr2 = FALSE,
                          extras = character(), replace = FALSE, se = FALSE, tex = FALSE,
                          lab = FALSE, ctitle = NULL, addtext = character(), addstat = list(),
                          data = NULL) {
  e <- stata_e
  nm <- colnames(e$b)
  b <- as.numeric(e$b)
  sev <- sqrt(pmax(diag(as.matrix(e$V)), 0))
  df <- if (!is.null(e$df_r) && is.finite(e$df_r)) e$df_r else Inf
  vars <- nm[nm != "_cons"]
  if (!is.null(keep)) vars <- vars[vars %in% keep]
  if ("_cons" %in% nm) vars <- c(vars, "_cons")
  num <- function(x, d) formatC(x, format = "f", digits = d, big.mark = ",")
  stars <- function(p) if (p < 0.01) "***" else if (p < 0.05) "**" else if (p < 0.1) "*" else ""
  coef <- list()
  for (v in vars) {
    i <- match(v, nm)
    if (is.na(b[i]) || !(sev[i] > 0)) next
    t <- b[i] / sev[i]
    p <- 2 * (if (is.finite(df)) stats::pt(-abs(t), df) else stats::pnorm(-abs(t)))
    # with se, the standard error has 3 decimals whatever tdec() says (Stata 19)
    shown <- if (se) sev[i] else t
    coef[[if (v == "_cons") "Constant" else v]] <- c(paste0(num(b[i], bdec), stars(p)), paste0("(", num(shown, if (se) 3 else tdec), ")"))
  }
  sig3 <- function(x) {
    if (x == round(x)) return(format(x, big.mark = ",", scientific = FALSE))
    formatC(x, format = "f", digits = max(0, 2 - floor(log10(abs(x)))))
  }
  # the column's statistics: Observations, R-squared from the model, xtreg's
  # Number of panels, then e() ones, addtext() and addstat() (Stata 19's order)
  fixed <- c("Observations", "R-squared", "Adjusted R-squared")
  st <- list(Observations = format(e$N, big.mark = ",", scientific = FALSE))
  r2v <- e$r2
  if (is.null(r2v) && identical(e$cmd, "xtreg") && identical(e$model, "fe")) r2v <- e$r2_w
  if ((r2 || !adjr2) && !is.null(r2v)) st[["R-squared"]] <- formatC(r2v, format = "f", digits = 3)
  if (adjr2 && !is.null(e$r2_a)) st[["Adjusted R-squared"]] <- formatC(e$r2_a, format = "f", digits = 3)
  if (identical(e$cmd, "xtreg") && !is.null(e$N_g)) st[[paste("Number of", e$ivar)]] <- format(e$N_g, scientific = FALSE)
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
  base <- sub("[.](xls|tex)$", "", file, ignore.case = TRUE)
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
  if (is.null(foot)) foot <- if (!se) "t-statistics in parentheses" else if (robust) "Robust standard errors in parentheses" else "Standard errors in parentheses"
  k <- length(heads)
  var_order <- c(setdiff(names(old_vars), "Constant"), setdiff(setdiff(names(coef), "Constant"), names(old_vars)))
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
  for (v in var_order) {
    o <- if (is.null(old_vars[[v]])) matrix("", 2, k) else old_vars[[v]]
    n <- if (is.null(coef[[v]])) c("", "") else coef[[v]]
    out <- c(out, list(c(v, lc(vlab(v)), o[1, ], n[1]), c("", lc(""), o[2, ], n[2])))
  }
  out <- c(out, list(blank))
  for (s in stat_order) {
    o <- if (is.null(old_stats[[s]])) rep("", k) else old_stats[[s]]
    out <- c(out, list(c(s, lc(""), o, if (is.null(st[[s]])) "" else st[[s]])))
  }
  out <- c(out, list(c(foot, rep("", k + 1 + has_lab)), c("*** p<0.01, ** p<0.05, * p<0.1", rep("", k + 1 + has_lab))))
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
    con <- file(file, "wb")
    writeChar(paste0(texl, eol, collapse = ""), con, eos = NULL)
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
  cat(if (tex || grepl("[.]xls$", file, ignore.case = TRUE)) basename(file) else basename(txt), "\ndir : seeout\n", sep = "")
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
            ("parenthesis", 5),
            ("adjr2", 5),
            ("r2", 2),
            ("keep", 4),
            ("e", 1),
            ("replace", 7),
            ("append", 6),
            ("tex", 3),
            ("label", 3),
            ("ctitle", 2),
            ("addtext", 4),
            ("addstat", 4),
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
        if opts.has("tstat", 5) == se {
            return Err(no(
                "outreg2 without tstat or se (its own default layout) is not translated yet",
            ));
        }
        if let Some(p) = opts.arg("parenthesis", 5) {
            let want = if se { "se" } else { "tstat" };
            if p.trim() != want {
                return Err(no(format!(
                    "outreg2, parenthesis({p}) is not translated yet"
                )));
            }
        }
        let digits = |name: &str| -> Result<String, Untranslated> {
            let v = opts.arg(name, 4).ok_or_else(|| {
                no(format!(
                    "outreg2 without {name}() (its own number format) is not translated yet"
                ))
            })?;
            match v.trim().parse::<u32>() {
                Ok(d) if d <= 10 => Ok(d.to_string()),
                _ => Err(no(format!(
                    "{name}({v}): only a number of decimals is translated"
                ))),
            }
        };
        let bdec = digits("bdec")?;
        let tdec = digits("tdec")?;
        let file = c
            .using
            .clone()
            .ok_or_else(|| no("outreg2 needs using file"))?;
        let path = file.trim().trim_matches('"').replace('\\', "/");
        let ext = std::path::Path::new(&path)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());
        let tex = match opts.arg("tex", 3) {
            Some(t) if t.trim() == "fragment" && ext.as_deref() == Some("tex") => true,
            Some(t) => {
                return Err(no(format!(
                    "outreg2, tex({t}) is not translated yet (tex(fragment) with a .tex file is)"
                )))
            }
            None => false,
        };
        if !tex && !matches!(ext.as_deref(), None | Some("xls")) {
            return Err(no(format!(
                "outreg2 using a .{} file is not translated yet (.xls, .txt and .tex with tex(fragment) are)",
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
        let mut args = format!("{}, bdec = {bdec}, tdec = {tdec}", r_text(&path));
        if let Some(k) = &keep {
            let names: Vec<String> = k.iter().map(|s| r_text(s)).collect();
            args.push_str(&format!(", keep = c({})", names.join(", ")));
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
        if se {
            args.push_str(", se = TRUE");
        }
        if tex {
            args.push_str(", tex = TRUE");
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
             (not its code), for tstat or se with bdec() and tdec()",
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
