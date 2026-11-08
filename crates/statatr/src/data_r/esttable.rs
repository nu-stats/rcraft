//! `estimates table`: the stored models' coefficients side by side, as
//! Stata 19 shows them (observed on made-up data): base levels left out,
//! stars at .05, .01 and .001 with `star`, standard errors below with `se`,
//! and the e() scalars of `stats()` at the foot. It leaves r(names) (and
//! r(coef) and r(stats), which are not kept here).

use super::returned::Returned;
use super::stats::r_logical;
use super::*;

pub(super) const EST_TABLE_HELPER: &str = r##"stata_est_table <- function(names, stats = character(), star = FALSE, bfmt = "%9.0g", se = FALSE, sefmt = "%9.0g") {
  g <- globalenv()
  st <- get0("stata_stored", envir = g, ifnotfound = list())
  keys <- c("stata_estimates", "stata_b", "stata_vcov", "stata_df", "stata_e")
  one <- function(s) {
    had <- vapply(keys, function(k) exists(k, envir = g, inherits = FALSE), NA)
    old <- lapply(keys, function(k) if (had[[k]]) get(k, envir = g) else NULL)
    names(old) <- keys
    on.exit({
      for (k in keys) {
        if (had[[k]]) assign(k, old[[k]], envir = g)
        else if (exists(k, envir = g, inherits = FALSE)) rm(list = k, envir = g)
      }
    })
    for (k in intersect(keys, names(s))) assign(k, s[[k]], envir = g)
    e <- get("stata_e", envir = g)
    eb <- stata_eb(get("stata_b", envir = g), get("stata_vcov", envir = g), e$cmd, e$depvar)
    b <- eb$b[1, ]
    nm <- colnames(eb$b)
    eqs <- unique(sub(":.*$", "", nm[grepl(":", nm)]))
    if (length(eqs) == 1 && all(grepl(":", nm))) nm <- sub("^[^:]*:", "", nm)
    s_e <- sqrt(pmax(diag(as.matrix(eb$V)), 0))
    df <- get0("stata_df", envir = g, ifnotfound = Inf)
    if (is.null(df) || length(df) != 1 || is.na(df)) df <- Inf
    list(b = stats::setNames(b, nm), se = stats::setNames(s_e, nm), df = df, e = e)
  }
  ms <- lapply(names, function(n) {
    if (n == ".") return(one(mget(intersect(keys, ls(g)), envir = g)))
    s <- st[[n]]
    if (is.null(s)) stop("estimation result ", n, " not found (Stata: r(111))", call. = FALSE)
    one(s)
  })
  base <- function(n) grepl("(^|#)[0-9]+b\\.|(^|#)o\\.", n)
  rows <- unique(unlist(lapply(ms, function(m) names(m$b)[!base(names(m$b))])))
  w <- if (star) 16 else 12
  heads <- ifelse(names == ".", "Active", names)
  rule <- function(ch) cat(strrep("-", 13), ch, strrep("-", w * length(ms) + 2), "\n", sep = "")
  cat("\n")
  rule("-")
  cat(formatC("Variable", width = 12), " |", paste(formatC(heads, width = -(w - 2)), collapse = "  "), "\n", sep = "")
  rule("+")
  cell <- function(m, r, what) {
    if (!r %in% names(m$b)) return(strrep(" ", w))
    if (what == "b") {
      v <- trimws(stata_fmt(m$b[[r]], bfmt))
      s <- ""
      if (star && m$se[[r]] > 0) {
        z <- m$b[[r]] / m$se[[r]]
        p <- if (is.finite(m$df)) 2 * stats::pt(-abs(z), m$df) else 2 * stats::pnorm(-abs(z))
        s <- if (p < .001) "***" else if (p < .01) "**" else if (p < .05) "*" else ""
      }
      paste0("  ", formatC(v, width = 9), formatC(s, width = -(w - 11)))
    } else {
      paste0("  ", formatC(trimws(stata_fmt(m$se[[r]], sefmt)), width = 9), strrep(" ", w - 11))
    }
  }
  last <- ""
  for (i in seq_along(rows)) {
    r <- rows[i]
    fv <- regmatches(r, regexec("^([0-9]+)\\.([A-Za-z_][A-Za-z0-9_]*)$", r))[[1]]
    group <- if (length(fv)) fv[3] else ""
    if (group != last && i > 1 && (nzchar(group) || nzchar(last))) cat(formatC("", width = 12), " |\n", sep = "")
    if (nzchar(group) && group != last) cat(formatC(group, width = 12), " |\n", sep = "")
    label <- if (nzchar(group)) paste0(fv[2], " ") else r
    cat(formatC(label, width = 12), " |", paste(vapply(ms, cell, "", r = r, what = "b"), collapse = ""), "\n", sep = "")
    if (se) cat(formatC("", width = 12), " |", paste(vapply(ms, cell, "", r = r, what = "se"), collapse = ""), "\n", sep = "")
    last <- group
  }
  if (length(stats)) {
    rule("+")
    for (s in stats) {
      v <- vapply(ms, function(m) {
        x <- m$e[[s]]
        if (is.null(x)) strrep(" ", w) else paste0("  ", formatC(trimws(stata_fmt(x, "%9.0g")), width = 9), strrep(" ", w - 11))
      }, "")
      cat(formatC(s, width = 12), " |", paste(v, collapse = ""), "\n", sep = "")
    }
  }
  rule("-")
  if (star) {
    cat(strrep(" ", 6), "Legend: * p<0.05; ** p<0.01; *** p<0.001\n", sep = "")
  } else if (se) {
    cat(strrep(" ", 14), "Legend: b/se\n", sep = "")
  }
  invisible(NULL)
}"##;

impl Session {
    /// `estimates table [names] [, stats(scalars) star b[(fmt)] se[(fmt)]]`.
    pub(super) fn estimates_table(
        &mut self,
        c: &Command,
        names: &str,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let opts = Opts(&c.options);
        if let Some(o) = opts.other(&[("stats", 2), ("star", 4), ("b", 1), ("se", 2)]) {
            return Err(no(format!(
                "estimates table with the option {o} is not translated yet"
            )));
        }
        if c.options
            .iter()
            .any(|o| o.is("star", 4) && !o.arg().trim().is_empty())
        {
            return Err(no(
                "estimates table, star() with levels is not translated yet",
            ));
        }
        let mut list: Vec<String> = names.split_whitespace().map(str::to_string).collect();
        if list.is_empty() {
            if matches!(self.e_ret, Returned::Nothing | Returned::Lost(_)) {
                return Err(no(
                    "estimates table: the model Stata holds here is not known",
                ));
            }
            list.push(".".into());
        }
        for n in &list {
            if n.contains(['*', '?']) || n == "_all" {
                return Err(no(
                    "estimates table of a pattern of names is not translated yet",
                ));
            }
            if n != "." && self.stored_fit(n).is_none() {
                return Err(no(format!(
                    "estimates table: {n} is not stored by estimates store earlier in this do-file"
                )));
            }
        }
        let fmt = |o: &str, min: usize| -> Result<Option<String>, Untranslated> {
            match c.options.iter().find(|x| x.is(o, min)) {
                None => Ok(None),
                Some(x) => {
                    let a = x.arg().trim().to_string();
                    if a.is_empty() {
                        Ok(Some("%9.0g".into()))
                    } else if a.starts_with('%') && !a.contains(char::is_whitespace) {
                        Ok(Some(a))
                    } else {
                        Err(no(format!(
                            "estimates table, {o}({a}) is not translated yet"
                        )))
                    }
                }
            }
        };
        let bfmt = fmt("b", 1)?.unwrap_or_else(|| "%9.0g".into());
        let sefmt = fmt("se", 2)?;
        let stats: Vec<String> = opts
            .arg("stats", 2)
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if let Some(s) = stats.iter().find(|s| !is_name(s)) {
            return Err(no(format!(
                "estimates table, stats({s}): e() scalars are translated"
            )));
        }
        self.helpers.insert("stata_est_table");
        self.helpers.insert("stata_fmt");
        self.helpers.insert("stata_eb");
        self.helpers.insert("stata_names");
        self.helpers.insert("stata_returned");
        let call = format!(
            "stata_est_table(c({}), stats = c({}), star = {}, bfmt = {}, se = {}, sefmt = {})",
            list.iter()
                .map(|n| r_text(n))
                .collect::<Vec<_>>()
                .join(", "),
            stats
                .iter()
                .map(|s| r_text(s))
                .collect::<Vec<_>>()
                .join(", "),
            r_logical(opts.has("star", 4)),
            r_text(&bfmt),
            r_logical(sefmt.is_some()),
            r_text(sefmt.as_deref().unwrap_or("%9.0g")),
        );
        if !self.quiet(c) {
            out.line(call);
        }
        let shown: Vec<&str> = list
            .iter()
            .map(|n| if n == "." { "Active" } else { n.as_str() })
            .collect();
        out.line(format!(
            "stata_r <- list(names = {})",
            r_text(&shown.join(" "))
        ));
        self.r_new = Some((super::returned::R_EST_TABLE, "estimates".into()));
        out.caveat("printed in R's layout; r(coef) and r(stats) are not kept");
        Ok(())
    }
}
