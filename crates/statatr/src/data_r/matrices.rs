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
//! line of 80 (Stata's width when nothing else sets one).

use super::{no, Opts, Out, Session};
use crate::expr_r::{r_text, Untranslated};
use crate::reader::Command;

/// `stata_eb(b, V, cmd, depvar)`: Stata's `e(b)` and `e(V)`.
pub(super) const EB_HELPER: &str = r##"stata_eb <- function(b, V, cmd, depvar) {
  fit <- stata_estimates
  V <- as.matrix(V)
  # R's columns, when Stata's cannot be told
  approx <- function(why) {
    st <- stata_bnames()
    out <- list(b = matrix(b, 1, dimnames = list("y1", st)), V = V)
    dimnames(out$V) <- list(st, st)
    attr(out$b, "stata_names") <- why
    attr(out$V, "stata_names") <- why
    out
  }
  if (cmd %in% c("anova", "margins")) return(approx(paste0("Stata's e(b) after ", cmd, " has columns RCraft does not rebuild yet")))
  # a variable xtreg left out is still a column of Stata's, a zero
  om <- fit[["omitted"]]
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
  part <- function(p) {
    if (p == "(Intercept)") return(list(kind = "cons", key = p))
    m <- regmatches(p, regexec("^(factor\\(([^()]+)\\))(.+)$", p))[[1]]
    if (length(m)) return(list(kind = "f", key = m[2], var = m[3], lev = m[4]))
    m <- regmatches(p, regexec("^(stata_ib\\(([^,]+), \"[^\"]*\"\\))(.+)$", p))[[1]]
    if (length(m)) return(list(kind = "f", key = m[2], var = m[3], lev = m[4]))
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
      # the levels in Stata's order, the base one (R's first) a zero
      for (l in L[order(Ln)]) {
        if (l == L[1]) {
          add(paste0(l, "b.", f$var, with_o), NA_character_)
          next
        }
        i <- ii[match(l, have)]
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
        n0 <- got[[1]]$n
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
    ml <- cmd %in% c("logit", "probit", "poisson", "nbreg", "ologit", "oprobit") ||
      (cmd == "xtreg" && identical(fit$type, "mle"))
    eq <- if (ml) paste0(depvar, ":") else ""
    names_out <- paste0(eq, r$n)
    from <- r$f
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
  ab <- function(s) ifelse(nchar(s) > 12, paste0(substr(s, 1, 10), "~", substr(s, nchar(s), nchar(s))), s)
  cn <- colnames(m)
  if (is.null(cn)) cn <- paste0("c", seq_len(ncol(m)))
  rn <- rownames(m)
  if (is.null(rn)) rn <- paste0("r", seq_len(nrow(m)))
  has_eq <- grepl(":", cn, fixed = TRUE)
  eq <- ifelse(has_eq, paste0(sub(":.*$", "", cn), ":"), "")
  short <- sub("^[^:]*:", "", cn)
  # a factor's prefix on its own line above the name: 0b. over g, 1.g# over c.x
  top <- ifelse(grepl("#", short, fixed = TRUE), sub("^(.*#).*$", "\\1", short),
                ifelse(grepl(".", short, fixed = TRUE), sub("^(.*\\.).*$", "\\1", short), ""))
  bottom <- ab(substring(short, nchar(top) + 1))
  rl <- ab(rn)
  vals <- matrix(vapply(as.vector(m), function(v) stata_fmt(v, format, trim = TRUE), ""), nrow(m))
  W <- 2 + max(nchar(vals), nchar(bottom))
  L <- max(nchar(rl))
  per <- max(1, (78 - L) %/% W)
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
    /// `matrix list` of what a command left in e() or r().
    pub(super) fn matrix(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let main = c.parts.main.trim();
        let (sub, what) = match main.split_once(char::is_whitespace) {
            Some((s, w)) => (s, w.trim()),
            None => (main, ""),
        };
        if !matches!(sub, "list" | "li" | "lis") {
            return Err(no(format!(
                "matrix {sub} is not translated yet (RCraft lists the matrices commands leave, as matrix list e(b))"
            )));
        }
        let (which, name) = match what
            .strip_suffix(')')
            .and_then(|w| w.split_once('('))
        {
            Some((w @ ("e" | "r"), n)) if super::is_name(n.trim()) => (w, n.trim()),
            _ => {
                return Err(no(format!(
                    "matrix list {what}: only the matrices a command leaves (e(b), e(V), r(C)) are translated yet"
                )))
            }
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
        let shown = format!("{which}({name})");
        let source = self.returned_matrix(which, name)?;
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
        for h in ["stata_matrix_list", "stata_fmt"] {
            self.helpers.insert(h);
        }
        if Self::quiet(c) {
            return Ok(());
        }
        out.line(format!("stata_matrix_list({})", args.join(", ")));
        Ok(())
    }
}
