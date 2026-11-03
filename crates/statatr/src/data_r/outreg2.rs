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
pub(super) const OUTREG2_HELPER: &str = r#"stata_outreg2 <- function(file, bdec, tdec, keep = NULL, r2 = FALSE, adjr2 = FALSE,
                          extras = character(), replace = FALSE) {
  e <- stata_e
  nm <- colnames(e$b)
  b <- as.numeric(e$b)
  se <- sqrt(pmax(diag(as.matrix(e$V)), 0))
  df <- if (!is.null(e$df_r) && is.finite(e$df_r)) e$df_r else Inf
  vars <- nm[nm != "_cons"]
  if (!is.null(keep)) vars <- vars[vars %in% keep]
  if ("_cons" %in% nm) vars <- c(vars, "_cons")
  num <- function(x, d) formatC(x, format = "f", digits = d, big.mark = ",")
  stars <- function(p) if (p < 0.01) "***" else if (p < 0.05) "**" else if (p < 0.1) "*" else ""
  coef <- list()
  for (v in vars) {
    i <- match(v, nm)
    if (is.na(b[i]) || !(se[i] > 0)) next
    t <- b[i] / se[i]
    p <- 2 * (if (is.finite(df)) stats::pt(-abs(t), df) else stats::pnorm(-abs(t)))
    coef[[if (v == "_cons") "Constant" else v]] <- c(paste0(num(b[i], bdec), stars(p)), paste0("(", num(t, tdec), ")"))
  }
  sig3 <- function(x) {
    if (x == round(x)) return(format(x, big.mark = ",", scientific = FALSE))
    formatC(x, format = "f", digits = max(0, 2 - floor(log10(abs(x)))))
  }
  st <- list(Observations = format(e$N, big.mark = ",", scientific = FALSE))
  if ((r2 || !adjr2) && !is.null(e$r2)) st[["R-squared"]] <- formatC(e$r2, format = "f", digits = 3)
  if (adjr2 && !is.null(e$r2_a)) st[["Adjusted R-squared"]] <- formatC(e$r2_a, format = "f", digits = 3)
  for (x in extras) if (!is.null(e[[x]]) && !is.na(e[[x]])) st[[x]] <- sig3(e[[x]])
  txt <- if (grepl("[.]xls$", file, ignore.case = TRUE)) sub("[.]xls$", ".txt", file, ignore.case = TRUE) else paste0(file, ".txt")
  # the table so far: its columns' cells by variable and by statistic
  old_vars <- list()
  old_stats <- list()
  heads <- character()
  deps <- character()
  if (!replace && file.exists(txt)) {
    rows <- lapply(sub("\r$", "", readLines(txt, warn = FALSE)), function(l) utils::head(strsplit(paste0(l, "\tX"), "\t", fixed = TRUE)[[1]], -1))
    k <- length(rows[[1]]) - 1
    cell <- function(r, j) { v <- rows[[r]][j + 1]; if (is.na(v)) "" else v }
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
  }
  k <- length(heads)
  var_order <- c(setdiff(names(old_vars), "Constant"), setdiff(setdiff(names(coef), "Constant"), names(old_vars)))
  if ("Constant" %in% c(names(old_vars), names(coef))) var_order <- c(var_order, "Constant")
  fixed <- c("Observations", "R-squared", "Adjusted R-squared")
  all_stats <- unique(c(names(old_stats), names(st)))
  stat_order <- c(intersect(fixed, all_stats), sort(setdiff(all_stats, fixed), method = "radix"))
  blank <- rep("", k + 2)
  out <- list(c("", heads, sprintf("(%d)", k + 1)), c("VARIABLES", deps, e$depvar), blank)
  for (v in var_order) {
    o <- if (is.null(old_vars[[v]])) matrix("", 2, k) else old_vars[[v]]
    n <- if (is.null(coef[[v]])) c("", "") else coef[[v]]
    out <- c(out, list(c(v, o[1, ], n[1]), c("", o[2, ], n[2])))
  }
  out <- c(out, list(blank))
  for (s in stat_order) {
    o <- if (is.null(old_stats[[s]])) rep("", k) else old_stats[[s]]
    out <- c(out, list(c(s, o, if (is.null(st[[s]])) "" else st[[s]])))
  }
  out <- c(out, list(c("t-statistics in parentheses", rep("", k + 1)), c("*** p<0.01, ** p<0.05, * p<0.1", rep("", k + 1))))
  eol <- if (.Platform$OS.type == "windows") "\r\n" else "\n"
  con <- file(txt, "wb")
  writeChar(paste0(vapply(out, paste, "", collapse = "\t"), eol, collapse = ""), con, eos = NULL)
  close(con)
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
  cat(if (grepl("[.]xls$", file, ignore.case = TRUE)) basename(file) else basename(txt), "\ndir : seeout\n", sep = "")
  invisible(NULL)
}"#;

impl Session {
    /// `outreg2 using file, tstat bdec(#) tdec(#) [parenthesis(tstat) adjr2
    /// r2 keep(vars) e(names) replace append]` after a model.
    pub(super) fn outreg2(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("tstat", 5),
            ("bdec", 4),
            ("tdec", 4),
            ("parenthesis", 5),
            ("adjr2", 5),
            ("r2", 2),
            ("keep", 4),
            ("e", 1),
            ("replace", 7),
            ("append", 6),
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
        if !opts.has("tstat", 5) {
            return Err(no(
                "outreg2 without tstat (standard errors and its own number format) is not translated yet",
            ));
        }
        if let Some(p) = opts.arg("parenthesis", 5) {
            if p.trim() != "tstat" {
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
        if !matches!(ext.as_deref(), None | Some("xls")) {
            return Err(no(format!(
                "outreg2 using a .{} file is not translated yet (.xls and .txt are)",
                ext.unwrap_or_default()
            )));
        }
        let Some(est) = self.estimates.as_ref() else {
            return Err(no("outreg2: no model is fitted before it in this do-file"));
        };
        if !est.factors.is_empty() {
            return Err(no(
                "outreg2 after a model with factor variables is not translated yet",
            ));
        }
        let mut args = format!("{}, bdec = {bdec}, tdec = {tdec}", r_text(&path));
        if let Some(k) = opts.arg("keep", 4) {
            let names: Vec<String> = k.split_whitespace().map(r_text).collect();
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
        self.helpers.insert("stata_outreg2");
        out.line(format!("stata_outreg2({args})"));
        out.caveat(
            "outreg2 as the SSC package writes its .txt and .xls, matched to the files it writes \
             (not its code), for tstat with bdec() and tdec()",
        );
        Ok(())
    }
}
