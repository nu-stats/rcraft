//! `contract` and `recast`, as Stata 19 does them on made-up data
//! (`plan/stata/contract.md`).

use super::*;
use crate::lang::syntax::WeightKind;

/// `contract varlist`: one row per combination of the variables' values, in
/// their sort order (missing last, "" first), with its frequency.
///
/// Observed in Stata 19:
/// - the new variables come after the varlist in the order freq, cfreq,
///   percent, cpercent; freq is not labelled, the others are "Cumulative
///   frequency", "Percent" and "Cumulative percent"; the percents are
///   doubles (floats with `float`) shown %8.2f (or `format()`);
/// - percent is 100 * f / N and cpercent 100 * cumulative f / N, bit for bit;
/// - a missing value is a category of its own unless `nomiss` (which drops
///   "" too); `zero` adds every combination of the values seen, with 0;
/// - fweights add up; a missing weight drops the row, a fraction is r(401);
/// - a new name that is already a variable is r(110), even one contract
///   would drop; no rows is r(2000); the data are then left as they were;
/// - the varlist's labels, value labels and formats stay; the data are
///   sorted by the varlist.
pub(super) const CONTRACT_HELPER: &str = r#"stata_contract <- function(d, vars, keep = NULL, w = NULL, freq = "_freq", cfreq = NULL, percent = NULL, cpercent = NULL, float = FALSE, format = "%8.2f", zero = FALSE, nomiss = FALSE, tags = FALSE) {
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  for (v in c(freq, cfreq, percent, cpercent)) if (v %in% names(d)) stop("variable ", v, " already defined (Stata: r(110))", call. = FALSE)
  use <- if (is.null(keep)) rep(TRUE, nrow(d)) else (!is.na(keep) & keep)
  if (!is.null(w)) {
    w <- as.numeric(w)
    use <- use & !is.na(w)
    if (any(use & w != trunc(w))) stop("may not use noninteger frequency weights (Stata: r(401))", call. = FALSE)
    if (any(use & w <= 0)) stop("contract with weights of zero or less is not translated yet", call. = FALSE)
  }
  bare <- lapply(d[vars], function(x) { if (is.factor(x)) x <- as.integer(x); attributes(x) <- NULL; x })
  if (nomiss) for (x in bare) use <- use & !is.na(x) & !(is.character(x) & x %in% "")
  if (!any(use)) stop("no observations (Stata: r(2000))", call. = FALSE)
  rows <- which(use)
  # with tags each of . and .a to .z is a category of its own: their places
  # (stata_mv) are keys beside the values
  mv <- lapply(bare, function(x) if (tags && is.double(x) && anyNA(x)) stata_mv(x))
  if (tags && zero && any(!vapply(mv, is.null, TRUE))) stop("contract, zero with .a to .z is not translated yet", call. = FALSE)
  kb <- c(bare, Filter(Negate(is.null), mv))
  names(kb) <- paste0("k", seq_along(kb))
  b <- as.data.frame(lapply(kb, `[`, rows), stringsAsFactors = FALSE)
  wt <- if (is.null(w)) rep(1, length(rows)) else w[rows]
  g <- vctrs::vec_group_id(b)
  first <- rows[!duplicated(g)]
  f <- as.numeric(tapply(wt, g, sum))
  if (zero) {
    seen <- lapply(seq_along(vars), function(j) rows[!duplicated(b[[j]])])
    grid <- expand.grid(seen, KEEP.OUT.ATTRS = FALSE)
    key <- as.data.frame(lapply(seq_along(vars), function(j) bare[[j]][grid[[j]]]), stringsAsFactors = FALSE)
    names(key) <- vars
    have <- as.data.frame(lapply(bare, `[`, first), stringsAsFactors = FALSE)
    m <- vctrs::vec_match(key, have)
    f <- ifelse(is.na(m), 0, f[m])
    pick <- as.data.frame(grid)
  } else {
    pick <- as.data.frame(lapply(vars, function(v) first))
  }
  names(pick) <- vars
  by <- unlist(lapply(seq_along(vars), function(j) {
    x <- bare[[j]][pick[[j]]]
    if (is.null(mv[[j]])) list(x) else list(x, mv[[j]][pick[[j]]])
  }), recursive = FALSE)
  ord <- do.call(order, c(by, list(na.last = TRUE, method = "radix")))
  pick <- pick[ord, , drop = FALSE]
  f <- f[ord]
  out <- list()
  for (v in vars) {
    x <- d[[v]][pick[[v]]]
    mostattributes(x) <- attributes(d[[v]])
    out[[v]] <- x
  }
  out <- as.data.frame(out, stringsAsFactors = FALSE, optional = TRUE)
  N <- sum(f)
  out[[freq]] <- f
  pct <- function(x, label) {
    if (float) x <- stata_float(x)
    structure(x, label = label, format.stata = format)
  }
  if (!is.null(cfreq)) out[[cfreq]] <- structure(cumsum(f), label = "Cumulative frequency")
  if (!is.null(percent)) out[[percent]] <- pct(100 * f / N, "Percent")
  if (!is.null(cpercent)) out[[cpercent]] <- pct(100 * cumsum(f) / N, "Cumulative percent")
  rownames(out) <- NULL
  out
}"#;

/// `recast type varlist [, force]` for numeric types.
///
/// Observed in Stata 19: a value the new type cannot hold is truncated
/// toward zero (byte, int, long) or rounded to float, and one outside the
/// type's range becomes missing; without `force` nothing changes when a
/// value would, and "x:  3 values would be changed; not changed" is shown
/// (the type stays); with it, "x:  3 values changed" ("1 value"); nothing
/// is shown when no value changes. A string to a number type, or the
/// reverse, is "x:  float invalid", r(109).
pub(super) const RECAST_HELPER: &str = r#"stata_recast <- function(d, type, vars, force = FALSE, quiet = FALSE) {
  lim <- list(byte = c(-127, 100), int = c(-32767, 32740), long = c(-2147483647, 2147483620))
  for (v in vars) {
    if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    x <- d[[v]]
    if (is.character(x)) stop(v, ":  ", type, " invalid (Stata: r(109))", call. = FALSE)
    y <- as.numeric(x)
    if (type == "float") y <- stata_float(y)
    if (type %in% names(lim)) {
      y <- trunc(y)
      y[!is.na(y) & (y < lim[[type]][1] | y > lim[[type]][2])] <- NA
    }
    changed <- sum(!is.na(x) & (is.na(y) | y != x))
    s <- if (changed == 1) "value" else "values"
    if (changed > 0 && !force) {
      if (!quiet) cat(v, ":  ", changed, " ", s, " would be changed; not changed\n", sep = "")
      next
    }
    if (changed > 0 && !quiet) cat(v, ":  ", changed, " ", s, " changed\n", sep = "")
    attributes(y) <- attributes(x)
    d[[v]] <- y
  }
  d
}"#;

impl Session {
    pub(super) fn contract(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("freq", 4),
            ("cfreq", 5),
            ("percent", 7),
            ("cpercent", 8),
            ("float", 5),
            ("format", 3),
            ("zero", 4),
            ("nomiss", 6),
        ]) {
            return Err(no(format!(
                "contract with the option {o} is not translated yet"
            )));
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if vars.is_empty() {
            return Err(no("varlist required (Stata: r(100))"));
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "contract {v}: names are translated (patterns are when the do-file runs)"
            )));
        }
        let name = |o: &str, min: usize| -> Result<Option<String>, Untranslated> {
            match opts.arg(o, min) {
                None => Ok(None),
                Some(a) => {
                    let a = a.trim().to_string();
                    if is_name(&a) {
                        Ok(Some(a))
                    } else {
                        Err(no(format!("contract, {o}({a}): one new name is needed")))
                    }
                }
            }
        };
        let freq = name("freq", 4)?.unwrap_or_else(|| "_freq".into());
        let cfreq = name("cfreq", 5)?;
        let percent = name("percent", 7)?;
        let cpercent = name("cpercent", 8)?;
        let format = match opts.arg("format", 3) {
            Some(f) => {
                if percent.is_none() && cpercent.is_none() {
                    return Err(no("percent or cpercent must be specified (Stata: r(198))"));
                }
                f.trim().to_string()
            }
            None => "%8.2f".into(),
        };
        let weight = match &c.parts.weight {
            None => None,
            Some(w) if matches!(w.kind, WeightKind::Frequency) => Some(w.exp.clone()),
            Some(_) => return Err(no("contract takes frequency weights only (Stata: r(101))")),
        };
        let d = self.data()?;
        let keep = self.condition(c, out)?;
        let w = match &weight {
            Some(e) => Some(self.expr(e, Use::Value, out)?),
            None => None,
        };
        self.helpers.insert("stata_contract");
        self.helpers.insert("stata_float");
        out.package("vctrs");
        let with = |e: Option<String>| match e {
            Some(e) => format!("dplyr::pull(dplyr::mutate({d}, stata_rows = {e}), stata_rows)"),
            None => "NULL".into(),
        };
        let text = |n: &Option<String>| match n {
            Some(n) => r_text(n),
            None => "NULL".into(),
        };
        // . and .a to .z each a category of its own (Stata 19)
        let tags = if self.mv_on(out) { ", tags = TRUE" } else { "" };
        out.line(format!(
            "{d} <- stata_contract({d}, c({}), keep = {}, w = {}, freq = {}, cfreq = {}, percent = {}, cpercent = {}, float = {}, format = {}, zero = {}, nomiss = {}{tags})",
            vars.iter().map(|v| r_text(v)).collect::<Vec<_>>().join(", "),
            with(keep),
            with(w),
            r_text(&freq),
            text(&cfreq),
            text(&percent),
            text(&cpercent),
            super::stats::r_logical(opts.has("float", 5)),
            r_text(&format),
            super::stats::r_logical(opts.has("zero", 4)),
            super::stats::r_logical(opts.has("nomiss", 6)),
        ));
        let keep_vars: Vec<String> = vars.iter().map(|v| v.to_string()).collect();
        self.kinds.retain(|k, _| keep_vars.contains(k));
        self.storage.retain(|k, _| keep_vars.contains(k));
        for n in [Some(freq), cfreq.clone()].into_iter().flatten() {
            self.kinds.insert(n, Kind::Number);
        }
        for n in [percent, cpercent].into_iter().flatten() {
            self.kinds.insert(n.clone(), Kind::Number);
            self.storage.insert(
                n,
                if opts.has("float", 5) {
                    Storage::Float
                } else {
                    Storage::Double
                },
            );
        }
        Ok(())
    }

    pub(super) fn recast(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("force", 5)]) {
            return Err(no(format!(
                "recast with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || c.parts.weight.is_some() {
            return Err(no("recast takes no if, in or weights (Stata: r(101))"));
        }
        let mut words = c.parts.main.split_whitespace();
        let ty = words.next().unwrap_or("");
        let vars: Vec<&str> = words.collect();
        let st = match ty {
            "byte" | "int" | "long" | "float" | "double" => storage(ty),
            t if t.starts_with("str") => {
                return Err(no(format!(
                    "recast {t} is not translated yet (number types are)"
                )))
            }
            t => {
                return Err(no(format!(
                    "recast {t}: a storage type is needed (Stata: r(198))"
                )))
            }
        };
        if vars.is_empty() {
            return Err(no("varlist required (Stata: r(100))"));
        }
        if let Some(v) = vars.iter().find(|v| !is_name(v)) {
            return Err(no(format!(
                "recast {ty} {v}: names are translated (patterns are when the do-file runs)"
            )));
        }
        let d = self.data()?;
        self.helpers.insert("stata_recast");
        self.helpers.insert("stata_float");
        let force = opts.has("force", 5);
        out.line(format!(
            "{d} <- stata_recast({d}, {}, c({}), force = {}, quiet = {})",
            r_text(ty),
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", "),
            super::stats::r_logical(force),
            super::stats::r_logical(self.quiet(c)),
        ));
        for v in vars {
            // widening never changes a value; without force a narrower type is
            // taken only when no value changes, which only R knows
            if force || ty == "double" {
                if let Some(st) = st {
                    self.storage.insert(v.to_string(), st);
                }
            } else {
                self.storage.remove(v);
            }
        }
        Ok(())
    }
}

impl Session {
    /// `isvar names` (Nick Cox's isvar, SSC), as observed in Stata 19: the
    /// dofile runner has already split the names into "variables | others"
    /// with the data's names. r() is cleared and holds r(varlist) and
    /// r(badlist), each only when it has a name; Stata shows "variable(s):"
    /// and "not variable(s):" lines.
    pub(super) fn isvar(
        &mut self,
        c: &Command,
        _opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if !c.options.is_empty() || c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("isvar takes names only"));
        }
        let Some((good, bad)) = c.parts.main.split_once(" | ") else {
            if c.parts.main.trim().is_empty() {
                return Err(no("isvar needs names (Stata: r(100))"));
            }
            return Err(no(format!(
                "isvar: which names are variables is {}",
                crate::dofile::KNOWN_WHEN_RUN
            )));
        };
        let (good, bad) = (good.trim(), bad.trim());
        let mut items = Vec::new();
        if !good.is_empty() {
            items.push(format!("varlist = {}", r_text(good)));
        }
        if !bad.is_empty() {
            items.push(format!("badlist = {}", r_text(bad)));
        }
        out.line(format!("stata_r <- list({})", items.join(", ")));
        if !self.quiet(c) {
            let show = |label: &str, list: &str| {
                let s = if list.split_whitespace().count() == 1 {
                    ""
                } else {
                    "s"
                };
                format!("cat({})", r_text(&format!("{label}{s}: {list}\n")))
            };
            if !good.is_empty() {
                out.line(show("variable", good));
            }
            if !bad.is_empty() {
                out.line(show("not variable", bad));
            }
        }
        self.r_new = Some((super::returned::R_ISVAR, "isvar".into()));
        Ok(())
    }
}
