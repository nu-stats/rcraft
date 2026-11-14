//! `e(b)` and `e(V)` as Stata keeps them, and `matrix list` (P2B.6).
//!
//! Stata's `e(b)` has a column for every coefficient it names, including
//! the base level of a factor (`0b.g`, a zero), a level or a variable left
//! out (`o.z`), the base outcome of `mlogit` (`2:o.x`), with each equation's
//! constant last and the terms in the order the model was written. R's
//! coefficients have none of those and put interactions after the main
//! effects, so `stata_eb()` rebuilds Stata's columns from the fitted model.
//! Where it cannot be sure of them (two factors interacted, a factor R kept
//! no levels for), `e(b)` keeps R's columns and says so, and `matrix list`
//! stops with that reason rather than print columns Stata would not have.
//!
//! `matrix list` prints Stata's layout, decoded from Stata 19: each column
//! as wide as its widest entry and two spaces, a line of equation names and
//! one of factor prefixes (`0b.`, `1.g#`) set one place to the right, names
//! past 12 characters shortened as Stata does (`averyveryv~e`), the lower
//! half of a symmetric matrix, and blocks of as many columns as fit in a
//! line of 80 (Stata's width when nothing else sets one; `set linesize` sets it).

use super::{no, Opts, Out, Session};
use crate::expr_r::{r_text, Untranslated};
use crate::reader::Command;

/// `stata_eb(b, V, cmd, depvar)`: Stata's `e(b)` and `e(V)`.
pub(super) const EB_HELPER: &str = r##"stata_eb <- function(b, V, cmd, depvar) {
  fit <- stata_estimates
  V <- as.matrix(V)
  # R's columns, when Stata's cannot be told
  approx <- function(why) {
    # margins names its own columns as Stata does (1.g, 5b.g, 1._at)
    st <- if (cmd == "margins") names(b) else stata_bnames()
    # R's own columns: without the omitted terms added below
    out <- list(b = matrix(b[seq_along(st)], 1, dimnames = list("y1", st)), V = V)
    dimnames(out$V) <- list(st, st)
    attr(out$b, "stata_names") <- why
    attr(out$V, "stata_names") <- why
    out
  }
  # suest names its columns as Stata does (co_chosen:1.actor, with the bases),
  # and so does margins, post of effects alone (1.g with 4b.g, or x)
  if (cmd == "suest" || (cmd == "margins" && isTRUE(attr(b, "stata_layout")))) {
    b <- c(b)
    V <- as.matrix(V)
    dimnames(V) <- list(names(b), names(b))
    return(list(b = matrix(b, 1, dimnames = list("y1", names(b))), V = V))
  }
  if (cmd %in% c("anova", "margins")) return(approx(paste0("Stata's e(b) after ", cmd, " has columns RCraft does not rebuild yet")))
  # a variable xtreg left out is still a column of Stata's, a zero
  om <- fit[["omitted"]]
  # mlogit: in each equation
  if (length(om) && cmd == "mlogit") {
    eqs <- unique(sub(":.*$", "", names(b)))
    om <- paste0(rep(eqs, each = length(om)), ":", om)
  }
  if (length(om)) b <- c(b, stats::setNames(rep(NA_real_, length(om)), om))
  nm <- names(b)
  # the terms in the order the model was written
  written <- character(0)
  fo <- tryCatch(stats::formula(fit), error = function(e) NULL)
  if (!is.null(fo) && length(fo) == 3) {
    rhs <- paste(deparse(fo[[3]], width.cutoff = 500L), collapse = "")
    for (t in strsplit(rhs, "+", fixed = TRUE)[[1]]) {
      t <- trimws(sub("-\\s*1\\s*$", "", trimws(t)))
      if (t %in% c("", "0", "1")) next
      written <- c(written, paste(sort(trimws(strsplit(t, ":", fixed = TRUE)[[1]])), collapse = ":"))
    }
  }
  # a model fitted again without its collinear terms keeps the order as written
  if (!is.null(fit[["stata_written"]])) written <- fit[["stata_written"]]
  part <- function(p) {
    if (p == "(Intercept)") return(list(kind = "cons", key = p))
    m <- regmatches(p, regexec("^(factor\\(([^()]+)\\))(.+)$", p))[[1]]
    if (length(m)) return(list(kind = "f", key = m[2], var = m[3], lev = m[4]))
    m <- regmatches(p, regexec("^(stata_ib\\(([^,]+), \"[^\"]*\"\\))(.+)$", p))[[1]]
    if (length(m)) return(list(kind = "f", key = m[2], var = m[3], lev = m[4]))
    # ibn.x: every level its own column, none the base (1.x 2.x ..., as Stata names them)
    m <- regmatches(p, regexec("^(stata_ibn\\(([^()]+)\\))(.+)$", p))[[1]]
    if (length(m)) return(list(kind = "f", key = m[2], var = m[3], lev = m[4], nobase = TRUE))
    # two factors interacted alone: one factor of cells (stata_cells), level a#b
    m <- regmatches(p, regexec("^(stata_cells\\(([^,]+), ([^,]+), \"([^\"]*)\", \"([^\"]*)\"\\))(.+)$", p))[[1]]
    if (length(m)) return(list(kind = "cells", key = m[2], vars = m[3:4], lev = m[7]))
    m <- regmatches(p, regexec("^I\\(([^()^]+)\\^([0-9]+)\\)$", p))[[1]]
    if (length(m)) return(list(kind = "c", key = p, var = m[2], pow = as.integer(m[3])))
    list(kind = "c", key = p, var = p, pow = 1L)
  }
  cont <- function(cs, op) paste(unlist(lapply(cs, function(p) rep(paste0(op, p$var), p$pow))), collapse = "#")
  # one equation's columns: Stata's names and the R coefficient each is (NA: a zero)
  eq_cols <- function(idx, short) {
    ps <- lapply(short, function(n) lapply(strsplit(n, ":", fixed = TRUE)[[1]], part))
    key <- vapply(ps, function(q) paste(sort(vapply(q, function(p) p$key, "")), collapse = ":"), "")
    pos <- match(key, written)
    pos[is.na(pos)] <- length(written) + match(key[is.na(pos)], unique(key))
    pos[key == "(Intercept)"] <- Inf
    out_n <- character(0)
    out_f <- character(0)
    add <- function(n, f) {
      out_n <<- c(out_n, n)
      out_f <<- c(out_f, f)
    }
    for (k in unique(key[order(pos)])) {
      ii <- which(key == k)
      q <- ps[[ii[1]]]
      if (q[[1]]$kind == "cons") {
        add("_cons", nm[idx[ii[1]]])
        next
      }
      fs <- Filter(function(p) p$kind == "f", q)
      cs <- Filter(function(p) p$kind == "c", q)
      # two factors interacted: every cell, the first factor outermost; a
      # factor at its base is marked b, one in a cell not estimated o. Alone
      # they are one factor of cells whose first level is the base cell.
      cl <- Filter(function(p) p$kind == "cells", q)
      two <- length(fs) == 2 && length(cs) == 0 && length(q) == 2
      if (two || (length(cl) == 1 && length(q) == 1)) {
        if (two) {
          Ls <- lapply(fs, function(f) fit$xlevels[[f$key]])
          if (any(vapply(Ls, is.null, TRUE))) return(NULL)
          base <- vapply(Ls, function(L) L[1], "")
          vars <- vapply(fs, function(f) f$var, "")
          cell <- vapply(ii, function(i) paste(vapply(ps[[i]], function(p) p$lev, ""), collapse = "#"), "")
        } else {
          C <- fit$xlevels[[cl[[1]]$key]]
          if (is.null(C)) return(NULL)
          ab <- strsplit(C, "#", fixed = TRUE)
          Ls <- lapply(1:2, function(j) unique(vapply(ab, function(x) x[j], "")))
          base <- ab[[1]]
          vars <- cl[[1]]$vars
          cell <- vapply(ii, function(i) ps[[i]][[1]]$lev, "")
        }
        Lns <- lapply(Ls, function(L) suppressWarnings(as.numeric(L)))
        if (any(vapply(Lns, anyNA, TRUE))) return(NULL)
        fs <- lapply(vars, function(v) list(var = v))
        for (l1 in Ls[[1]][order(Lns[[1]])]) {
          for (l2 in Ls[[2]][order(Lns[[2]])]) {
            i <- ii[match(paste(l1, l2, sep = "#"), cell)]
            est <- !is.na(i) && !is.na(b[idx[i]])
            mark <- function(l, j) if (l == base[j]) "b" else if (!est) "o" else ""
            add(paste0(l1, mark(l1, 1), ".", fs[[1]]$var, "#", l2, mark(l2, 2), ".", fs[[2]]$var),
                if (est) nm[idx[i]] else NA_character_)
          }
        }
        next
      }
      if (length(fs) > 1) return(NULL)
      if (length(fs) == 0) {
        name <- if (length(cs) == 1 && cs[[1]]$pow == 1) cs[[1]]$var else cont(cs, "c.")
        for (i in ii) {
          if (!is.na(b[idx[i]])) {
            add(name, nm[idx[i]])
          } else if (length(cs) == 1 && cs[[1]]$pow == 1) {
            add(paste0("o.", name), NA_character_)
          } else {
            return(NULL)
          }
        }
        next
      }
      f <- fs[[1]]
      L <- fit$xlevels[[f$key]]
      Ln <- suppressWarnings(as.numeric(L))
      if (is.null(L) || anyNA(Ln)) return(NULL)
      have <- vapply(ii, function(i) Filter(function(p) p$kind == "f", ps[[i]])[[1]]$lev, "")
      with_c <- if (length(cs)) paste0("#", cont(cs, "c.")) else ""
      with_o <- if (length(cs)) paste0("#", cont(cs, "co.")) else ""
      # the levels in Stata's order, the base one (R's first) a zero; with a
      # continuous variable and not the factor's own term (g#c.x alone), R
      # gives the first level a coefficient too, and so does Stata
      for (l in L[order(Ln)]) {
        i <- ii[match(l, have)]
        if (l == L[1] && !isTRUE(f$nobase) && (is.na(i) || !length(cs))) {
          add(paste0(l, "b.", f$var, with_o), NA_character_)
          next
        }
        if (!is.na(i) && !is.na(b[idx[i]])) {
          add(paste0(l, ".", f$var, with_c), nm[idx[i]])
        } else if (length(cs) == 0) {
          add(paste0(l, "o.", f$var), NA_character_)
        } else {
          return(NULL)
        }
      }
    }
    list(n = out_n, f = out_f)
  }
  why <- "two factor variables interacted, or a factor R kept no levels for"
  aux <- startsWith(nm, "/")
  names_out <- character(0)
  from <- character(0)
  if (cmd == "mlogit") {
    eqs <- as.character(stata_levels)
    got <- list()
    for (q in eqs[eqs != stata_base]) {
      idx <- which(startsWith(nm, paste0(q, ":")))
      r <- eq_cols(idx, sub("^[^:]*:", "", nm[idx]))
      if (is.null(r)) return(approx(why))
      got[[q]] <- r
    }
    for (q in eqs) {
      if (q == stata_base) {
        # the base outcome's equation: Stata's columns of the others, left out
        # an omitted term is o. there already
        n0 <- sub("^o\\.", "", got[[1]]$n)
        if (any(grepl("[.#]", n0))) return(approx(why))
        names_out <- c(names_out, paste0(q, ":o.", n0))
        from <- c(from, rep(NA_character_, length(n0)))
      } else {
        names_out <- c(names_out, paste0(q, ":", got[[q]]$n))
        from <- c(from, got[[q]]$f)
      }
    }
  } else {
    idx <- which(!aux)
    r <- eq_cols(idx, nm[idx])
    if (is.null(r)) return(approx(why))
    ml <- cmd %in% c("logit", "probit", "poisson", "nbreg", "ologit", "oprobit", "clogit") ||
      (cmd == "xtreg" && identical(fit$type, "mle"))
    eq <- if (cmd == "asclogit") paste0(fit[["stata_altvar"]], ":") else if (ml) paste0(depvar, ":") else ""
    names_out <- paste0(eq, r$n)
    from <- r$f
    # a variable named twice: its own column again right after it, omitted
    for (v in fit[["stata_repeats"]]) {
      p <- match(paste0(eq, v), names_out)
      if (is.na(p)) next
      names_out <- append(names_out, paste0(eq, "o.", v), after = p)
      from <- append(from, NA_character_, after = p)
    }
  }
  # cut points and the like: Stata's equation "/" (xtreg, mle's sigma_u:_cons)
  if (any(aux)) {
    a <- sub("^/", "", nm[aux])
    names_out <- c(names_out, if (cmd == "xtreg") paste0(a, ":_cons") else paste0("/:", a))
  }
  from <- c(from, nm[aux])
  k <- length(names_out)
  bv <- ifelse(is.na(from), 0, b[from])
  Vv <- matrix(0, k, k, dimnames = list(names_out, names_out))
  ok <- !is.na(from) & from %in% rownames(V)
  Vv[ok, ok] <- V[from[ok], from[ok]]
  list(b = matrix(unname(bv), 1, dimnames = list("y1", names_out)), V = Vv)
}"##;

/// `stata_matrix_list(m, name, ...)`: Stata's `matrix list`.
pub(super) const MATRIX_LIST_HELPER: &str = r##"stata_matrix_list <- function(m, name, format = "%10.0g", half = TRUE, header = TRUE, blank = TRUE,
                              title = NULL) {
  if (is.null(m)) stop("matrix ", name, " not found")
  why <- attr(m, "stata_names")
  if (!is.null(why)) stop(name, " is not laid out here as Stata keeps it: ", why)
  m <- as.matrix(m)
  sym <- nrow(m) == ncol(m) && isSymmetric(unname(m))
  # names past 12 characters, as Stata shortens them
  ab <- function(s) stata_tab_abbrev(s, 12)
  cn <- colnames(m)
  if (is.null(cn)) cn <- paste0("c", seq_len(ncol(m)))
  rn <- rownames(m)
  if (is.null(rn)) rn <- paste0("r", seq_len(nrow(m)))
  has_eq <- grepl(":", cn, fixed = TRUE)
  # an equation name cut to 11 (its colon makes 12), as Stata lists it: threatcr~ry:
  eq <- ifelse(has_eq, paste0(stata_tab_abbrev(sub(":.*$", "", cn), 11), ":"), "")
  short <- sub("^[^:]*:", "", cn)
  # a factor's prefix on its own line above the name: 0b. over g, 1.g# over c.x
  top <- ifelse(grepl("#", short, fixed = TRUE), sub("^(.*#).*$", "\\1", short),
                ifelse(grepl(".", short, fixed = TRUE), sub("^(.*\\.).*$", "\\1", short), ""))
  bottom <- ab(substring(short, nchar(top) + 1))
  # over()'s names split at @: c.y@ over 0.b, 0.b@ over 1.g (Stata 19, after mean and proportion)
  at <- grepl("@", short, fixed = TRUE)
  rest <- gsub("([0-9])bn\\.", "\\1.", sub("^.*@", "", short))
  # two variables in over(): three lines, c.y@ over 1.g# over 0.b
  deep <- at & grepl("#", rest, fixed = TRUE)
  eq <- ifelse(deep, sub("^(.*@).*$", "\\1", short), eq)
  top <- ifelse(at, ifelse(deep, sub("^(.*#).*$", "\\1", rest), sub("^(.*@).*$", "\\1", short)), top)
  bottom <- ifelse(at, ab(ifelse(deep, sub("^.*#", "", rest), rest)), bottom)
  # 1bn._at shows as 1. over _at, as Stata lists it
  top <- gsub("([0-9])bn\\.", "\\1.", top)
  rl <- ab(rn)
  vals <- matrix(vapply(as.vector(m), function(v) stata_fmt(v, format, trim = TRUE), ""), nrow(m))
  # as wide as its widest entry, name or equation name (Stata 19: lags: over 1)
  W <- 2 + max(nchar(vals), nchar(bottom), nchar(eq), nchar(top))
  L <- max(nchar(rl))
  # set linesize, kept in R; Stata's width is 80 when nothing sets one
  ls <- if (exists("stata_linesize")) stata_linesize else 80
  per <- max(1, (ls - 2 - L) %/% W)
  pad <- function(s, w) formatC(s, width = w)
  out <- character(0)
  if (blank) out <- ""
  if (header) {
    out <- c(out, paste0(if (sym) "symmetric " else "", name, "[", nrow(m), ",", ncol(m), "]",
                         if (!is.null(title)) paste0(":  ", title) else ""))
  }
  blocks <- split(seq_len(ncol(m)), (seq_len(ncol(m)) - 1) %/% per)
  for (k in seq_along(blocks)) {
    j <- blocks[[k]]
    if (k > 1) out <- c(out, "")
    if (any(eq != "")) out <- c(out, paste0(strrep(" ", L + 1), paste(pad(eq[j], W), collapse = "")))
    if (any(top != "")) out <- c(out, paste0(strrep(" ", L + 1), paste(pad(top[j], W), collapse = "")))
    out <- c(out, paste0(strrep(" ", L), paste(pad(bottom[j], W), collapse = "")))
    rows <- if (sym && half) seq(min(j), nrow(m)) else seq_len(nrow(m))
    for (i in rows) {
      cols <- if (sym && half) j[j <= i] else j
      out <- c(out, paste0(pad(rl[i], L), paste(pad(vals[i, cols], W), collapse = "")))
    }
  }
  cat(out, sep = "\n")
  invisible(m)
}"##;

impl Session {
    /// `matrix`: define (`matrix A = exp`, `matrix A[i,j] = exp`), the row
    /// and column names, `drop`, and `list`.
    pub(super) fn matrix(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let rest = super::returned::after_name(c);
        let rest = rest.trim();
        self.helpers.insert("stata_matrix");
        let (word, tail) = match rest.split_once(char::is_whitespace) {
            Some((w, t)) => (w, t.trim()),
            None => (rest, ""),
        };
        if let Some(target) = rest
            .split_once('=')
            .and_then(|(l, e)| Target::read(l).map(|t| (t, e)))
        {
            return self.matrix_define(target.0, target.1, out);
        }
        if matches!(word, "de" | "def" | "defi" | "defin" | "define") {
            return match tail
                .split_once('=')
                .and_then(|(l, e)| Target::read(l).map(|t| (t, e)))
            {
                Some((t, e)) => self.matrix_define(t, e, out),
                None => Err(no("matrix define needs `name = exp`")),
            };
        }
        let names_word = |full: &str| word.len() >= 4 && full.starts_with(word);
        if names_word("rownames") || names_word("colnames") {
            return self.matrix_names(word.starts_with("row"), tail, out);
        }
        if word == "drop" {
            return self.matrix_drop(tail, out);
        }
        if !matches!(word, "list" | "l" | "li" | "lis") {
            return Err(no(format!("matrix {word} is not translated yet")));
        }
        let what = c.parts.main.trim();
        let what = what
            .split_once(char::is_whitespace)
            .map_or("", |(_, w)| w.trim());
        let (source, shown) = if super::is_name(what) {
            (self.user_matrix(what)?, what.to_string())
        } else {
            let (which, name) = match what.strip_suffix(')').and_then(|w| w.split_once('(')) {
                Some((w @ ("e" | "r"), n)) if super::is_name(n.trim()) => (w, n.trim()),
                _ => {
                    return Err(no(format!(
                        "matrix list {what}: only a matrix by name and those a command leaves (e(b), e(V), r(C)) are translated yet"
                    )))
                }
            };
            (
                self.returned_matrix(which, name)?,
                format!("{which}({name})"),
            )
        };
        let opts = Opts(&c.options);
        if let Some(o) = opts.other(&[
            ("format", 1),
            ("nohalf", 3),
            ("noheader", 5),
            ("noblank", 4),
            ("title", 1),
        ]) {
            return Err(no(format!(
                "matrix list with the option {o} is not translated yet"
            )));
        }
        let mut args = vec![source, r_text(&shown)];
        if let Some(f) = opts.arg("format", 1) {
            let f = f.trim();
            if !super::returned::valid_format(f) || f.ends_with('s') {
                return Err(no(format!("format({f}) is not a numeric format")));
            }
            args.push(format!("format = {}", r_text(f)));
        }
        for (opt, min, arg) in [
            ("nohalf", 3, "half = FALSE"),
            ("noheader", 5, "header = FALSE"),
            ("noblank", 4, "blank = FALSE"),
        ] {
            if opts.has(opt, min) {
                args.push(arg.into());
            }
        }
        if let Some(t) = opts.arg("title", 1) {
            args.push(format!("title = {}", r_text(t.trim().trim_matches('"'))));
        }
        for h in ["stata_matrix_list", "stata_fmt", "stata_tables"] {
            self.helpers.insert(h);
        }
        if self.quiet(c) {
            // nothing shown, but a matrix that is not there still stops it
            out.line(format!(
                "invisible({})",
                args.first().map_or("NULL", |s| s.as_str())
            ));
            return Ok(());
        }
        out.line(format!("stata_matrix_list({})", args.join(", ")));
        Ok(())
    }
}

/// What `matrix ... =` assigns to: a whole matrix, or one place in it.
struct Target {
    name: String,
    at: Option<(String, String)>,
}

impl Target {
    fn read(lhs: &str) -> Option<Target> {
        let lhs = lhs.trim();
        let (name, at) = match lhs.split_once('[') {
            Some((n, rest)) => {
                let inside = rest.trim().strip_suffix(']')?;
                let (i, j) = inside.split_once(',')?;
                (n.trim(), Some((i.trim().to_string(), j.trim().to_string())))
            }
            None => (lhs, None),
        };
        (super::is_name(name) && name.len() <= 32).then(|| Target {
            name: name.to_string(),
            at,
        })
    }
}

impl Session {
    /// A matrix the do-file made, as R; why not when it is not known: never
    /// made (Stata stops, r(111)), or changed by a line not translated.
    pub(super) fn user_matrix(&self, name: &str) -> Result<String, Untranslated> {
        if self.user_mats.contains(name) {
            // moved into e() by a line under capture, or not: R knows
            if self.maybe_moved.contains(name) {
                return Ok(format!(
                    "(if (is.null(stata_mat[[{0}]])) stop({1}, call. = FALSE) else stata_mat[[{0}]])",
                    r_text(name),
                    r_text(&format!("matrix {name} not found (Stata: r(111))"))
                ));
            }
            return Ok(format!("stata_mat[[{}]]", r_text(name)));
        }
        match self.lost_mats.get(name) {
            Some(line) if line.contains(crate::dofile::KNOWN_WHEN_RUN) => Err(no(format!(
                "matrix {name} is {} (a line before it may change it: {line})",
                crate::dofile::KNOWN_WHEN_RUN
            ))),
            Some(line) => Err(no(format!(
                "matrix {name} is not known here: a line that may change it was not translated ({line})"
            ))),
            // never made: Stata stops there, and so does R (no translated command
            // makes a matrix the translator does not know of, and a refused line
            // marks what it may make as lost)
            None => Ok(format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("matrix {name} not found (Stata: r(111))"))
            )),
        }
    }

    /// A matrix the line makes.
    pub(super) fn matrix_made(&mut self, name: &str) {
        self.lost_mats.remove(name);
        self.maybe_moved.remove(name);
        self.user_mats.insert(name.to_string());
    }
}

impl Session {
    fn matrix_define(&mut self, t: Target, exp: &str, out: &mut Out) -> Result<(), Untranslated> {
        let (code, _) = self.matrix_expr(exp.trim(), out)?;
        self.helpers.insert("stata_matrix");
        if code == "NULL" {
            // nullmat(name) alone, of no matrix: Stata stops, and so does R
            let n = exp
                .trim()
                .trim_start_matches("nullmat(")
                .trim_end_matches(')')
                .trim();
            out.line(format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("matrix {n} not found (Stata: r(111))"))
            ));
            return Ok(());
        }
        let place = format!("stata_mat[[{}]]", r_text(&t.name));
        match &t.at {
            None => {
                out.line(format!("{place} <- stata_mnamed({code})"));
                self.matrix_made(&t.name);
            }
            // one place in a matrix that is there (else R stops, as Stata)
            Some((i, j)) => {
                let m = self.user_matrix(&t.name)?;
                let (i, _) = self.display_value(i, out)?;
                let (j, _) = self.display_value(j, out)?;
                out.line(format!("{place} <- stata_mset({m}, {i}, {j}, {code})"));
            }
        }
        Ok(())
    }

    /// `matrix rownames A = names` (and colnames): as many names as rows.
    fn matrix_names(&mut self, rows: bool, text: &str, out: &mut Out) -> Result<(), Untranslated> {
        let (name, names) = text
            .split_once('=')
            .ok_or_else(|| no("matrix rownames needs `name = names`"))?;
        let name = name.trim();
        let m = self.user_matrix(name)?;
        // a quoted name is taken as written, spaces and all ("18--24", "65+",
        // "Quota diff"), as Stata 19 keeps it
        let mut words: Vec<String> = Vec::new();
        let mut rest = names.trim();
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix('"') {
                let end = r
                    .find('"')
                    .ok_or_else(|| no("matrix rownames: a quoted name is not closed"))?;
                words.push(r.get(..end).unwrap_or("").to_string());
                rest = r.get(end + 1..).unwrap_or("").trim_start();
            } else {
                let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
                words.push(rest.get(..end).unwrap_or("").to_string());
                rest = rest.get(end..).unwrap_or("").trim_start();
            }
        }
        if words.is_empty()
            || words
                .iter()
                .any(|w| w.is_empty() || w.contains([':', '.', '#', '"', '`', '\'']))
        {
            return Err(no(
                "these row or column names (equations, factor variables) are not translated yet",
            ));
        }
        let list = words
            .iter()
            .map(|w| r_text(w))
            .collect::<Vec<_>>()
            .join(", ");
        let place = format!("stata_mat[[{}]]", r_text(name));
        let rows = if rows { "TRUE" } else { "FALSE" };
        out.line(format!(
            "{place} <- stata_mnames({m}, c({list}), rows = {rows})"
        ));
        Ok(())
    }

    fn matrix_drop(&mut self, text: &str, out: &mut Out) -> Result<(), Untranslated> {
        let names: Vec<&str> = text.split_whitespace().collect();
        if names.is_empty() {
            return Err(no("matrix drop needs the matrices to drop, or _all"));
        }
        if names == ["_all"] {
            self.user_mats.clear();
            self.lost_mats.clear();
            out.line("stata_mat <- list()");
            return Ok(());
        }
        for n in &names {
            out.line(format!("invisible({})", self.user_matrix(n)?));
        }
        for n in &names {
            self.user_mats.remove(*n);
            out.line(format!("stata_mat[[{}]] <- NULL", r_text(n)));
        }
        Ok(())
    }
}

impl Session {
    /// `mkmat varlist [if] [in], matrix(name) [nomissing]`: the variables
    /// as the columns of a matrix (rows r1, r2, ...), or without matrix()
    /// a column vector for each, named after it.
    pub(super) fn mkmat(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let d = self.data()?;
        if let Some(o) = opts.other(&[("matrix", 3), ("nomissing", 5)]) {
            return Err(no(format!(
                "mkmat with the option {o} is not translated yet"
            )));
        }
        if c.prefixes
            .iter()
            .any(|p| matches!(p, crate::reader::Prefix::By { .. }))
        {
            return Err(no("by with mkmat is not translated yet"));
        }
        let vars = super::names(&c.parts.main)?;
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("dplyr::filter({d}, {cond})")
            }
            None => d.to_string(),
        };
        let list = vars
            .iter()
            .map(|v| r_text(v))
            .collect::<Vec<_>>()
            .join(", ");
        let nomiss = if opts.has("nomissing", 5) {
            "TRUE"
        } else {
            "FALSE"
        };
        self.helpers.insert("stata_matrix");
        let made = format!("stata_mkmat({rows}, c({list}), {nomiss})");
        match opts.arg("matrix", 3) {
            Some(m) => {
                let m = m.trim();
                if !super::is_name(m) || m.len() > 32 {
                    return Err(no(format!("{m} is not a valid matrix name")));
                }
                out.line(format!("stata_mat[[{}]] <- {made}", r_text(m)));
                self.matrix_made(m);
            }
            None => {
                out.line(format!("stata_v <- {made}"));
                for v in &vars {
                    if v.len() > 32 {
                        return Err(no(format!("{v} is not a valid matrix name")));
                    }
                    out.line(format!(
                        "stata_mat[[{0}]] <- stata_v[, {0}, drop = FALSE]",
                        r_text(v)
                    ));
                    self.matrix_made(v);
                }
            }
        }
        Ok(())
    }
}

/// Whether a line that was not translated may have changed the matrix `m`:
/// a `matrix` line only through what it assigns to (or drops), a line that
/// only reads values never, any other through any mention of the name.
pub(super) fn may_write(line: &str, m: &str) -> bool {
    let words = |t: &str| {
        t.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|w| w == m)
    };
    let word = crate::dofile::command_name(line.trim());
    let full = crate::reader::read(&word)
        .commands()
        .first()
        .map(|c| c.name.clone())
        .unwrap_or(word);
    match full.as_str() {
        "display" | "scalar" | "local" | "global" | "assert" | "generate" | "replace" => false,
        "matrix" => {
            // past any prefix and the command word
            let rest = crate::reader::read(line.trim())
                .commands()
                .first()
                .map(|c| super::returned::after_name(c))
                .unwrap_or_default();
            let rest = rest.trim();
            match rest.split_once('=') {
                Some((lhs, _)) if Target::read(lhs).is_some() => {
                    Target::read(lhs).is_some_and(|t| t.name == m)
                }
                Some((lhs, _)) => words(lhs),
                None => words(rest),
            }
        }
        _ => words(line),
    }
}

impl Session {
    /// `svmat [float|double] A [, names(col|stub)]`: the matrix's columns as
    /// new variables (A1, A2, ... by default), floats unless double, with
    /// observations added when it has more rows than the data, as Stata 19.
    pub(super) fn svmat(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("names", 1)]) {
            return Err(no(format!(
                "svmat with the option {o} is not translated yet"
            )));
        }
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (double, name) = match words.as_slice() {
            [n] => (false, *n),
            ["float", n] => (false, *n),
            ["double", n] => (true, *n),
            _ => return Err(no("svmat takes a type (float or double) and one matrix")),
        };
        let m = self.user_matrix(name)?;
        let names = match opts.arg("names", 1).as_deref().map(str::trim) {
            None => name.to_string(),
            Some("col") => "col".to_string(),
            Some(s @ ("eqcol" | "matcol")) => {
                return Err(no(format!("svmat, names({s}) is not translated yet")))
            }
            Some(s) if super::is_name(s) => s.to_string(),
            Some(s) => return Err(no(format!("svmat, names({s}): not a name (Stata: r(198))"))),
        };
        let d = match self.data.clone() {
            Some(d) => d,
            None if self.data_lost.is_some() => self.data()?.to_string(),
            None => {
                self.data = Some("data".into());
                "NULL".into()
            }
        };
        let place = self.data.clone().unwrap_or_else(|| "data".into());
        for h in [
            "stata_matrix",
            "stata_set_obs",
            "stata_float",
            "stata_svmat",
        ] {
            self.helpers.insert(h);
        }
        let quiet = if self.quiet(c) { "FALSE" } else { "TRUE" };
        out.line(format!(
            "{place} <- stata_svmat({d}, {m}, {}, double = {}, show = {quiet})",
            r_text(&names),
            if double { "TRUE" } else { "FALSE" }
        ));
        Ok(())
    }
}

/// `svmat`'s R.
pub(super) const SVMAT_HELPER: &str = r##"stata_svmat <- function(d, m, names, double = FALSE, show = TRUE) {
  m <- stata_mnamed(m)
  nm <- if (identical(names, "col")) colnames(m) else paste0(names, seq_len(ncol(m)))
  old <- if (is.null(d)) character(0) else names(d)
  ok <- grepl("^[A-Za-z_][A-Za-z0-9_]*$", nm) & nchar(nm) <= 32
  if (!all(ok) || anyDuplicated(nm) || any(nm %in% old))
    stop("new variables cannot be uniquely named or already defined (Stata: r(110))", call. = FALSE)
  n0 <- if (is.null(d)) 0 else nrow(d)
  if (nrow(m) > n0) {
    if (show) cat("number of observations will be reset to ", nrow(m), "\nPress any key to continue, or Break to abort\n",
                  "Number of observations (_N) was ", n0, ", now ", nrow(m), ".\n", sep = "")
    d <- stata_set_obs(d, nrow(m))
  }
  n <- nrow(d)
  for (k in seq_along(nm)) {
    v <- c(unname(m[, k]), rep(NA_real_, n - nrow(m)))
    d[[nm[k]]] <- if (double) v else stata_float(v)
  }
  d
}"##;

/// The matrices a line names as what it makes: a `matrix` line's target
/// (or what it drops), `mkmat`'s matrix() or its variables, and the
/// matrix options other commands take (matcell(), matrow(), matrix() ...).
pub(super) fn written(line: &str) -> Vec<String> {
    let read = crate::reader::read(line.trim());
    let Some(c) = read.commands().first().cloned() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    match c.name.as_str() {
        "matrix" => {
            let rest = super::returned::after_name(c);
            let rest = rest.trim();
            if let Some(t) = rest.split_once('=').and_then(|(l, _)| Target::read(l)) {
                out.push(t.name);
            } else {
                let mut ws = rest.split_whitespace();
                let word = ws.next().unwrap_or("");
                if matches!(word, "de" | "def" | "defi" | "defin" | "define") {
                    if let Some(t) = ws
                        .collect::<Vec<_>>()
                        .join(" ")
                        .split_once('=')
                        .and_then(|(l, _)| Target::read(l))
                    {
                        out.push(t.name);
                    }
                } else if word.len() >= 4
                    && ("rownames".starts_with(word) || "colnames".starts_with(word))
                {
                    if let Some(n) = ws.next() {
                        out.push(n.trim_end_matches('=').to_string());
                    }
                } else if word == "drop" {
                    out.extend(ws.map(str::to_string));
                } else if word != "list" && !matches!(word, "l" | "li" | "lis" | "dir") {
                    // other subcommands (accum, input, ...): the word after them
                    out.extend(ws.next().map(|n| n.trim_end_matches('=').to_string()));
                }
            }
        }
        "mkmat" => {
            let opts = super::Opts(&c.options);
            match opts.arg("matrix", 3) {
                Some(m) => out.push(m.trim().to_string()),
                None => out.extend(c.parts.main.split_whitespace().map(str::to_string)),
            }
        }
        // lines that only read values make no matrix
        "display" | "scalar" | "local" | "global" | "assert" | "generate" | "replace" => {}
        // any other command (mat_rapp AB : A B, an ado): any name it mentions
        _ => out.extend(
            super::returned::after_name(c)
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .filter(|w| w.len() <= 32)
                .map(str::to_string),
        ),
    }
    let opts = super::Opts(&c.options);
    for o in ["matcell", "matrow", "matcol", "matrix", "mat", "matname"] {
        if let Some(m) = opts.arg(o, o.len()) {
            out.push(m.trim().to_string());
        }
    }
    out.retain(|n| super::is_name(n));
    out
}
