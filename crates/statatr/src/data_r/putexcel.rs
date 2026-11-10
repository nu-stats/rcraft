//! `putexcel set file.xlsx [, sheet(name) modify | replace]`, `putexcel cell
//! = value ...` and `putexcel clear`, as Stata 19 writes the workbook
//! (observed on made-up workbooks, `plan/stata/putexcel.md`), with the R
//! package openxlsx2:
//!
//! - Text is a text cell (in the workbook's table of strings), a number a
//!   number written to 17 digits (`%.17g`, as Stata), `.` an empty cell.
//!   Several `cell = value` in one line are written in order.
//! - `modify` keeps the workbook's other sheets and cells and makes the file
//!   if it is not there; `replace` starts it again at the first write (with
//!   Stata's note); neither, on a file that exists, is r(602). Without
//!   `sheet()` the sheet is `Sheet1`, added after the others if missing.
//! - `file … saved` after each write, as Stata; a write before any `set` is
//!   r(198), as is a column past XFD.
//! - Not yet (refused): cell ranges, `matrix()`, `etable`, `formula()`,
//!   pictures, formatting options (bold, nformat(), ...), `sheet(, replace)`,
//!   .xls, `putexcel save`/`close`/`describe`, and a value other than a
//!   string, a number or `( expression )`.

use super::*;

pub(super) const PUTEXCEL_HELPER: &str = r##"stata_px_set <- function(file, sheet = "Sheet1", mode = "") {
  if (!nzchar(mode) && file.exists(file))
    stop("file ", file, " already exists; you must specify either option modify or option replace (Stata: r(602))", call. = FALSE)
  if (mode == "replace") cat("note: file will be replaced when the first putexcel command is issued.\n")
  list(file = file, sheet = sheet, fresh = mode == "replace")
}
stata_px_put <- function(p, cells) {
  if (is.null(p)) stop("no Excel file specified (Stata: r(198))", call. = FALSE)
  wb <- if (!p$fresh && file.exists(p$file)) openxlsx2::wb_load(p$file) else openxlsx2::wb_workbook()
  if (!p$sheet %in% wb$get_sheet_names()) wb$add_worksheet(p$sheet)
  i <- match(p$sheet, wb$get_sheet_names())
  for (d in names(cells)) {
    v <- cells[[d]]
    # a missing value empties the cell, as Stata leaves it
    if (length(v) == 1 && is.na(v)) {
      wb$clean_sheet(sheet = p$sheet, dims = d, numbers = TRUE, characters = TRUE, styles = FALSE, merged_cells = FALSE)
      next
    }
    wb$add_data(sheet = p$sheet, x = v, dims = d, col_names = FALSE, na.strings = NULL, inline_strings = FALSE)
    # a number as Stata writes it, to 17 digits (R's own text has 15)
    if (is.numeric(v)) {
      cc <- wb$worksheets[[i]]$sheet_data$cc
      cc$v[cc$r == d] <- sprintf("%.17g", v)
      wb$worksheets[[i]]$sheet_data$cc <- cc
    }
  }
  wb$save(p$file)
  cat("file ", p$file, " saved\n", sep = "")
  p$fresh <- FALSE
  p
}"##;

/// A cell's name upper-cased and checked: `Ok(None)` for a column past XFD
/// (Stata's r(198) at run time), `Err` for something that is not one cell.
fn cell(name: &str) -> Result<Option<String>, Untranslated> {
    let up = name.to_ascii_uppercase();
    let letters: String = up.chars().take_while(char::is_ascii_alphabetic).collect();
    let digits = up.get(letters.len()..).unwrap_or("");
    if letters.is_empty() || digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(no(format!(
            "putexcel {name}: only single cells (A1, C7) are translated yet, not ranges or names"
        )));
    }
    let column = letters
        .chars()
        .fold(0u64, |n, c| n * 26 + u64::from(c as u8 - b'A' + 1));
    let row: u64 = digits.parse().unwrap_or(0);
    if row == 0 || row > 1_048_576 {
        return Err(no(format!("putexcel {name}: a row Excel does not have")));
    }
    Ok((letters.len() <= 3 && column <= 16_384).then_some(up))
}

/// A number as Stata writes it in a do-file, for R; `.` is missing.
fn number(text: &str) -> Option<String> {
    if text == "." {
        return Some("NA_real_".into());
    }
    let ok = text
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
        && text.chars().any(|c| c.is_ascii_digit())
        && text.parse::<f64>().is_ok();
    // R reads 1e5, .5, -.5 and 5. as Stata does
    ok.then(|| text.to_string())
}

impl Session {
    pub(super) fn putexcel(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let main = c.parts.main.trim();
        let (first, rest) = match main.split_once(char::is_whitespace) {
            Some((a, b)) => (a, b.trim()),
            None => (main, ""),
        };
        match first {
            "set" => return self.putexcel_set(rest, opts, out),
            "clear" if rest.is_empty() && opts.other(&[]).is_none() => {
                out.line("stata_putexcel <- NULL");
                return Ok(());
            }
            "save" | "close" | "describe" | "clear" => {
                return Err(no(format!("putexcel {first} is not translated yet")))
            }
            _ => {}
        }
        if let Some(o) = opts.other(&[]) {
            return Err(no(format!(
                "putexcel with the option {o} (a cell's format) is not translated yet"
            )));
        }
        let mut cells: Vec<String> = Vec::new();
        let mut bad_column: Option<String> = None;
        let chars: Vec<char> = main.chars().collect();
        let mut i = 0;
        let skip = |i: &mut usize| {
            while chars.get(*i).is_some_and(|c| c.is_whitespace()) {
                *i += 1;
            }
        };
        loop {
            skip(&mut i);
            if i >= chars.len() {
                break;
            }
            let start = i;
            while chars
                .get(i)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == ':' || *c == '$')
            {
                i += 1;
            }
            let name: String = chars.get(start..i).unwrap_or_default().iter().collect();
            skip(&mut i);
            if name.is_empty() || chars.get(i) != Some(&'=') {
                return Err(no(
                    "putexcel is translated for set, clear and cell = value only, so far",
                ));
            }
            i += 1;
            skip(&mut i);
            let value = match chars.get(i) {
                // "text" or `"text"'
                Some('"') | Some('`') => {
                    let compound = chars.get(i) == Some(&'`');
                    let open = if compound { 2 } else { 1 };
                    let mut j = i + open;
                    let close: &[char] = if compound { &['"', '\''] } else { &['"'] };
                    while j < chars.len()
                        && chars.get(j..j + close.len()).is_none_or(|w| w != close)
                    {
                        j += 1;
                    }
                    if j >= chars.len() {
                        return Err(no("putexcel: a string without its closing quote"));
                    }
                    let s: String = chars.get(i + open..j).unwrap_or_default().iter().collect();
                    i = j + close.len();
                    r_text(&s)
                }
                // ( expression ): Stata's value, a string or a number
                Some('(') => {
                    let mut depth = 0;
                    let mut j = i;
                    let mut quoted = false;
                    while let Some(&ch) = chars.get(j) {
                        match ch {
                            '"' => quoted = !quoted,
                            '(' if !quoted => depth += 1,
                            ')' if !quoted => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    if depth != 0 {
                        return Err(no("putexcel: a ( without its )"));
                    }
                    let inner: String = chars.get(i + 1..j).unwrap_or_default().iter().collect();
                    i = j + 1;
                    let (code, _) = self.value(&inner, out)?;
                    code
                }
                _ => {
                    let start = i;
                    while chars.get(i).is_some_and(|c| !c.is_whitespace()) {
                        i += 1;
                    }
                    let word: String = chars.get(start..i).unwrap_or_default().iter().collect();
                    number(&word).ok_or_else(|| {
                        no(format!(
                            "putexcel {name} = {word}: only a string, a number or ( expression ) \
                             is translated yet (not matrix(), etable, formula() or pictures)"
                        ))
                    })?
                }
            };
            match cell(&name)? {
                Some(up) => cells.push(format!("{up} = {value}")),
                None => {
                    bad_column.get_or_insert(name);
                }
            }
        }
        if let Some(name) = bad_column {
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text(&format!(
                    "{name}: Excel column out of range (Stata: r(198))"
                ))
            ));
            return Ok(());
        }
        if cells.is_empty() {
            return Err(no("putexcel with nothing to write"));
        }
        self.helpers.insert("stata_putexcel");
        out.package("openxlsx2");
        out.line(format!(
            "stata_putexcel <- stata_px_put(get0(\"stata_putexcel\"), list({}))",
            cells.join(", ")
        ));
        Ok(())
    }

    fn putexcel_set(&mut self, rest: &str, opts: &Opts, out: &mut Out) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("sheet", 5), ("modify", 6), ("replace", 7)]) {
            return Err(no(format!(
                "putexcel set with the option {o} is not translated yet"
            )));
        }
        let files = crate::dofile::macros::tokens(rest);
        let [file] = files.as_slice() else {
            return Err(no("putexcel set needs one file name"));
        };
        let file = file.trim().trim_matches('"').to_string();
        if !file.to_ascii_lowercase().ends_with(".xlsx") {
            return Err(no(format!(
                "putexcel set {file}: only .xlsx workbooks named with their extension are translated yet"
            )));
        }
        let sheet = match opts.arg("sheet", 5) {
            Some(s) if s.contains(',') => {
                return Err(no(
                    "putexcel set, sheet(name, replace) is not translated yet",
                ))
            }
            Some(s) => s.trim().trim_matches('"').to_string(),
            None => "Sheet1".to_string(),
        };
        let mode = match (opts.has("modify", 6), opts.has("replace", 7)) {
            (true, true) => {
                out.line("stop(\"options modify and replace may not be combined (Stata: r(198))\", call. = FALSE)");
                return Ok(());
            }
            (true, false) => "modify",
            (false, true) => "replace",
            (false, false) => "",
        };
        self.helpers.insert("stata_putexcel");
        out.package("openxlsx2");
        out.line(format!(
            "stata_putexcel <- stata_px_set({}, sheet = {}, mode = {})",
            r_text(&file),
            r_text(&sheet),
            r_text(mode)
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_and_numbers_are_read_as_stata_writes_them() {
        assert_eq!(cell("c7").ok().flatten().as_deref(), Some("C7"));
        assert_eq!(cell("XFD1").ok().flatten().as_deref(), Some("XFD1"));
        assert!(matches!(cell("XFE1"), Ok(None)));
        assert!(matches!(cell("ZZZZ1"), Ok(None)));
        assert!(cell("A1:B2").is_err() && cell("A0").is_err() && cell("A").is_err());
        assert_eq!(number(".").as_deref(), Some("NA_real_"));
        assert_eq!(number("-.5").as_deref(), Some("-.5"));
        assert_eq!(number("1e-20").as_deref(), Some("1e-20"));
        assert!(number("inf").is_none() && number("matrix(b)").is_none() && number("e").is_none());
    }
}
