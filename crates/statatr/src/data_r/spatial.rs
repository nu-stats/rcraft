//! `spset` and `spmatrix`, as Stata 19 runs them. The method is StatCraft's
//! decoding of Stata's (its `plan/stata/spatial.md`, probes 730-737) and ours
//! (`plan/stata/spmatrix.md`), written here in R:
//!
//! - `spset id, coord(x y)` (planar): `_ID`, `_CX` and `_CY` and Stata's block;
//! - weighting matrices held in R (`stata_spw`, kept across `clear` as in
//!   Stata): inverse distance with normalize() and vtruncate(), dir, summarize
//!   (its r()), drop, copy, note, export and import in Stata's text form,
//!   fromdata, and save/use in RCraft's own file.
//!
//! Not translated: contiguity matrices (shapefiles), latitude and longitude,
//! Mata (matafromsp, spfrommata), and `spregress` (see [`SPREGRESS_REFUSED`]).

use super::*;

/// Why `spregress` is refused: probes in `plan/stata/spmatrix.md`.
pub(super) const SPREGRESS_REFUSED: &str = "spregress is not translated yet: Stata 19 stops its \
     concentrated-likelihood iterations where its numerical derivatives lead (not decoded), so its \
     estimates are not the likelihood's maximum: they differ from it from the 4th to the 7th digit, \
     and on some data Stata reaches another optimum";

pub(super) const SPATIAL_HELPER: &str = r##"# spset and spmatrix as Stata 19 runs them (StatCraft's decoding, plan/stata/spatial.md)
if (!exists("stata_spw", inherits = TRUE)) stata_spw <- list()
if (!exists("stata_spid", inherits = TRUE)) stata_spid <- NULL
stata_sp_stop <- function(msg, rc) stop(paste0(msg, " (Stata: r(", rc, "))"), call. = FALSE)
stata_sp_no <- function(what) stop(paste0("RCraft: ", what, " is not translated yet"), call. = FALSE)
stata_sp_num <- function(d, v) { u <- unclass(d[[v]]); attributes(u) <- NULL; as.numeric(u) }
stata_sp_show <- function(d, modify = FALSE) {
  id <- if (is.null(stata_spid)) "" else paste0(" (equal to ", stata_spid, ")")
  cat("\n      Sp dataset: <current>\nLinked shapefile: <none>\n            Data: Cross sectional\n",
      " Spatial-unit ID: _ID", id, "\n     Coordinates: ",
      if (all(c("_CX", "_CY") %in% names(d))) "_CX, _CY (planar)" else "<none>", "\n", sep = "")
  if (modify) cat("\n")
}
# spset [idvar] [, modify coordinates(x y)]
stata_spset <- function(d, id = NULL, coord = NULL, modify = FALSE, show = TRUE) {
  set <- "_ID" %in% names(d)
  if (is.null(id) && is.null(coord)) {
    if (!set) stata_sp_stop("data not spset", 459)
    if (show) stata_sp_show(d, modify)
    return(d)
  }
  if (set && !modify) stata_sp_stop("data already spset\n    Use spset, clear first or use spset, modify.", 459)
  if (!set && is.null(id)) {
    if (!modify) stata_sp_stop("option modify required", 198)
    stata_sp_no("spset, modify on data not spset")
  }
  for (v in c(id, coord)) {
    if (!v %in% names(d)) stata_sp_stop(paste0("variable ", v, " not found"), 111)
    if (is.character(d[[v]])) stata_sp_no("spset with a string variable")
  }
  if (!is.null(id)) {
    iv <- stata_sp_num(d, id)
    if (anyNA(iv)) stata_sp_no("spset with a missing id")
    if (anyDuplicated(iv)) stata_sp_stop("variable _ID does not uniquely identify the observations\n    Do these data need to be xtset?", 459)
    fmt <- attr(d[[id]], "format.stata", exact = TRUE)
    d[["_ID"]] <- structure(iv, label = "Spatial-unit ID", format.stata = fmt)
    stata_spid <<- id
  }
  if (!is.null(coord)) {
    d[["_CX"]] <- structure(stata_sp_num(d, coord[1]), format.stata = "%10.0g")
    d[["_CY"]] <- structure(stata_sp_num(d, coord[2]), format.stata = "%10.0g")
  }
  if (show) stata_sp_show(d, modify)
  d
}
stata_sp_spset_or_stop <- function(d) if (!"_ID" %in% names(d)) stata_sp_stop("data not spset", 459)
# normalize(): spectral (the largest absolute eigenvalue), row, minmax, none
stata_sp_normalize <- function(w, how) {
  if (how == "row") {
    s <- rowSums(w); k <- s > 0
    w[k, ] <- w[k, , drop = FALSE] / s[k]
    return(w)
  }
  sc <- switch(how,
    # the general eigen solver gives Stata 19's last bits for symmetric matrices too
    spectral = max(Mod(eigen(w, symmetric = FALSE, only.values = TRUE)$values)),
    minmax = min(max(rowSums(w)), max(colSums(w))),
    0)
  if (sc > 0) w <- w / sc
  w
}
stata_sp_exists <- function(name) stata_sp_stop(paste0("weighting matrix ", name, " already exists"), 110)
stata_sp_get <- function(name) {
  m <- stata_spw[[name]]
  if (is.null(m)) stata_sp_stop(paste0("weighting matrix ", name, " not found"), 111)
  m
}
stata_sp_put <- function(name, m) { l <- stata_spw; l[[name]] <- m; stata_spw <<- l[order(names(l), method = "radix")] }
stata_sp_kind <- function(w) if (all(w == 0 | w == 1)) "contiguity" else "custom"
# spmatrix create idistance name [if] [in] [, normalize() vtruncate() replace]
stata_spmatrix_create <- function(d, sel, name, norm = "spectral", vtrunc = 0, replace = FALSE) {
  stata_sp_spset_or_stop(d)
  if (!replace && !is.null(stata_spw[[name]])) stata_sp_exists(name)
  if (!all(c("_CX", "_CY") %in% names(d))) stata_sp_stop("coordinate variables must be numeric", 498)
  rows <- which(sel)
  x <- stata_sp_num(d, "_CX")[rows]; y <- stata_sp_num(d, "_CY")[rows]
  if (anyNA(x) || anyNA(y)) stata_sp_no("spmatrix create with missing coordinates")
  n <- length(rows)
  if (n > 5000) stata_sp_no("spmatrix create over 5,000 places")
  dx <- outer(x, x, "-"); dy <- outer(y, y, "-"); d2 <- dx * dx + dy * dy
  off <- row(d2) != col(d2)
  if (any(d2[off] == 0)) stata_sp_no("spmatrix create idistance with two places at the same coordinates")
  # 1 / exp(ln(d2) / 2) gives Stata 19's last bits, which d2^(-1/2) and 1 / sqrt(d2) do not
  w <- matrix(0, n, n); w[off] <- 1 / exp(0.5 * log(d2[off]))
  w[w < vtrunc] <- 0
  w <- stata_sp_normalize(w, norm)
  # a matrix of 0 and 1 (two places) is contiguity (Stata 19)
  bin <- all(abs(w) < 1e-12 | abs(w - 1) < 1e-12)
  if (bin) w <- round(w)
  stata_sp_put(name, list(ids = stata_sp_num(d, "_ID")[rows], w = w, type = if (bin) "contiguity" else "idistance",
                          norm = norm, note = NULL))
  invisible(NULL)
}
stata_spmatrix_dir <- function() {
  if (!length(stata_spw)) { cat("  (no weighting matrices found)\n"); return(invisible(NULL)) }
  r <- strrep("-", 72)
  cat("\n", r, "\n   Weighting matrix name           N x N      Type         Normalization\n", r, "\n", sep = "")
  for (nm in names(stata_spw)) {
    m <- stata_spw[[nm]]; n <- length(m$ids)
    cat(formatC(nm, width = 24), formatC(paste(n, "x", n), width = 16), "    ", formatC(m$type, width = -17), m$norm, "\n", sep = "")
  }
  cat(r, "\n", sep = "")
}
stata_spmatrix_summarize <- function(name, show = TRUE) {
  # a failed summarize leaves r() empty (Stata 19)
  stata_r <<- list()
  m <- stata_sp_get(name); w <- m$w; n <- length(m$ids)
  mn <- min(w); mx <- max(w); pos <- w[w > 0]; mn0 <- if (length(pos)) min(pos) else NA_real_
  mean <- sum(rowSums(w) / n) / n
  g <- function(v) stata_fmt(v, "%9.0g", trim = TRUE)
  ln <- function(l, v) cat(formatC(l, width = -15), " |", formatC(v, width = 21), "\n", sep = "")
  if (show) {
    r <- strrep("-", 39)
    cat("\nWeighting matrix ", name, "\n", r, "\n", sep = "")
    ln("           Type", m$type); ln("  Normalization", m$norm); ln("      Dimension", paste(n, "x", n))
    cat("Elements        |\n")
    ln("   minimum     ", g(mn)); ln("   minimum > 0 ", g(mn0)); ln("   mean        ", g(mean)); ln("   max         ", g(mx))
    if (m$type == "contiguity") {
      nb <- rowSums(w != 0)
      cat("Neighbors       |\n")
      ln("   minimum     ", g(min(nb))); ln("   mean        ", g(mean(nb))); ln("   maximum     ", g(max(nb)))
    }
    cat(r, "\n", sep = "")
  }
  stata_r <<- list(max = mx, min0 = mn0, mean = mean, min = mn, n = n, normalization = m$norm, type = m$type)
  invisible(NULL)
}
stata_spmatrix_drop <- function(name) {
  stata_sp_get(name); l <- stata_spw; l[[name]] <- NULL; stata_spw <<- l; invisible(NULL)
}
stata_spmatrix_copy <- function(from, to) {
  m <- stata_sp_get(from)
  # an existing matrix of that name is dropped before Stata complains (probe 737)
  if (!is.null(stata_spw[[to]])) { stata_spmatrix_drop(to); stata_sp_exists(to) }
  stata_sp_put(to, m); invisible(NULL)
}
stata_spmatrix_note <- function(name, text = NULL) {
  m <- stata_sp_get(name)
  if (is.null(text)) { if (!is.null(m$note)) cat("  ", m$note, "\n", sep = ""); return(invisible(NULL)) }
  m$note <- text; l <- stata_spw; l[[name]] <- m; stata_spw <<- l; invisible(NULL)
}
# save and use: RCraft's own file (R's serialization); Stata's .stswm files are not read
stata_spmatrix_save <- function(name, file, shown, replace = FALSE) {
  m <- stata_sp_get(name)
  if (file.exists(file) && !replace) stata_sp_stop(paste0(shown, " already exists"), 602)
  saveRDS(list(rcraft_spmatrix = 1L, m = m), file)
  cat("  (matrix ", name, " saved in file ", shown, ")\n", sep = "")
  invisible(NULL)
}
stata_spmatrix_use <- function(name, file, shown) {
  if (!file.exists(file)) stata_sp_stop(paste0("file ", shown, " not found"), 601)
  x <- tryCatch(readRDS(file), error = function(e) NULL)
  if (!is.list(x) || is.null(x$rcraft_spmatrix)) stata_sp_no(paste0("spmatrix use of ", shown, " (a file RCraft did not save)"))
  if (!is.null(x$m$note)) cat("  (", x$m$note, ")\n", sep = "")
  stata_sp_put(name, x$m); invisible(NULL)
}
# export: n on line 1, then each place's id and weights, as many digits as fit 16 characters
stata_sp_txt <- function(v) {
  idig <- ifelse(abs(v) < 1, 0, nchar(sprintf("%.0f", trunc(abs(v)))))
  s <- sprintf(paste0("%.", pmax(15 - idig, 0), "f"), v)
  s <- ifelse(grepl(".", s, fixed = TRUE), sub("\\.?0+$", "", s), s)
  s <- sub("^(-?)0\\.", "\\1.", s)
  s[s == "-0"] <- "0"
  s
}
stata_spmatrix_export <- function(name, file, shown, replace = FALSE) {
  m <- stata_sp_get(name)
  if (file.exists(file) && !replace) stata_sp_stop(paste0(shown, " already exists"), 602)
  n <- length(m$ids)
  lines <- c(as.character(n), vapply(seq_len(n), function(i) paste(stata_sp_txt(c(m$ids[i], m$w[i, ])), collapse = " "), ""))
  writeLines(lines, file)
  cat("  (matrix ", name, " saved in file ", shown, ")\n", sep = "")
  invisible(NULL)
}
stata_spmatrix_import <- function(name, file, shown, replace = FALSE) {
  if (!file.exists(file)) stata_sp_stop(paste0("file ", shown, " not found"), 601)
  if (!is.null(stata_spw[[name]]) && !replace) { stata_spmatrix_drop(name); stata_sp_exists(name) }
  txt <- readLines(file, warn = FALSE)
  first <- strsplit(trimws(if (length(txt)) txt[1] else ""), "[[:space:]]+")[[1]]
  n <- suppressWarnings(as.integer(first))
  if (length(first) != 1 || is.na(n) || n < 1)
    stata_sp_stop(paste0("error in line 1 of file ", shown, ": invalid bands\nerror reading file ", shown), 198)
  if (n > 5000) stata_sp_no("spmatrix import over 5,000 places")
  ids <- numeric(n); w <- matrix(0, n, n)
  for (i in seq_len(n)) {
    tok <- strsplit(trimws(if (length(txt) > i) txt[i + 1] else ""), "[[:space:]]+")[[1]]
    v <- suppressWarnings(as.numeric(tok))
    if (length(v) != n + 1)
      stata_sp_stop(paste0("incorrect number of columns in row ", i, " of weighting matrix\nerror reading file ", shown), 198)
    if (anyNA(v)) stata_sp_no("spmatrix import of a file with values that are not numbers")
    ids[i] <- v[1]; w[i, ] <- v[-1]
  }
  stata_sp_put(name, list(ids = ids, w = w, type = stata_sp_kind(w), norm = "none", note = NULL))
  invisible(NULL)
}
# spmatrix fromdata name = varlist [if] [in] [, normalize() replace]
stata_spmatrix_fromdata <- function(d, sel, name, vars, norm = "spectral", replace = FALSE) {
  stata_sp_spset_or_stop(d)
  for (v in vars) if (!v %in% names(d)) stata_sp_stop(paste0("variable ", v, " not found"), 111)
  rows <- which(sel)
  if (length(rows) != length(vars)) stata_sp_stop("matname: number of rows must be equal to number of columns", 498)
  if (!replace && !is.null(stata_spw[[name]])) stata_sp_exists(name)
  w <- sapply(vars, function(v) stata_sp_num(d, v)[rows]); w <- matrix(w, length(rows)); dimnames(w) <- NULL
  if (anyNA(w)) stata_sp_no("spmatrix fromdata with missing values")
  if (any(diag(w) != 0)) stata_sp_stop("matrix main diagonal must all be zero", 498)
  ty <- stata_sp_kind(w)
  stata_sp_put(name, list(ids = stata_sp_num(d, "_ID")[rows], w = stata_sp_normalize(w, norm), type = ty, norm = norm, note = NULL))
  invisible(NULL)
}
"##;

impl Session {
    /// A Stata error, raised in R so that `capture` sees its return code.
    fn sp_fail(out: &mut Out, msg: &str, rc: u32) -> Result<(), Untranslated> {
        out.line(format!(
            "stop({}, call. = FALSE)",
            r_text(&format!("{msg} (Stata: r({rc}))"))
        ));
        Ok(())
    }

    fn sp_helpers(&mut self) {
        for h in ["stata_spatial", "stata_fmt", "stata_returned"] {
            self.helpers.insert(h);
        }
    }

    /// The rows `if` and `in` choose, as an R logical vector.
    fn sp_rows(&self, c: &Command, d: &str, out: &mut Out) -> Result<String, Untranslated> {
        Ok(match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!("local({{ s <- dplyr::transmute({d}, s = {cond})$s; !is.na(s) & s }})")
            }
            None => format!("rep(TRUE, nrow({d}))"),
        })
    }

    /// `spset [idvar] [, modify coordinates(x y) coordsys(planar)]`.
    pub(super) fn spset(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "spset")?;
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() || c.parts.weight.is_some() {
            return Err(no("spset with if, in or weights is not translated"));
        }
        if let Some(o) = opts.other(&[("modify", 3), ("coordinates", 5), ("coordsys", 8)]) {
            return Err(no(format!(
                "spset with the option {o} is not translated yet (shapefiles and spset, clear are not)"
            )));
        }
        if let Some(cs) = opts.arg("coordsys", 8) {
            let cs = cs.trim();
            if cs.is_empty() || !"planar".starts_with(cs) {
                return Err(no(format!(
                    "spset, coordsys({cs}) is not translated yet (planar coordinates are)"
                )));
            }
        }
        let modify = opts.has("modify", 3);
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        if words.len() > 1 || !words.iter().all(|w| is_name(w)) {
            return Err(no("spset takes one id variable, by its name"));
        }
        let coord: Option<Vec<String>> = opts
            .arg("coordinates", 5)
            .map(|a| a.split_whitespace().map(str::to_string).collect());
        if let Some(cv) = &coord {
            match cv.len() {
                0 | 1 => return Self::sp_fail(out, "coord():  too few variables specified", 102),
                2 => {}
                _ => {
                    return Err(no(
                        "spset, coord() with more than two variables is not translated",
                    ))
                }
            }
            if !cv.iter().all(|w| is_name(w)) {
                return Err(no("spset, coord() is translated with variable names"));
            }
        }
        let Ok(d) = self.data() else {
            return Self::sp_fail(out, "no dataset in use", 3);
        };
        self.sp_helpers();
        let id = words.first().map_or("NULL".to_string(), |w| r_text(w));
        let co = coord.as_ref().map_or("NULL".to_string(), |v| {
            format!(
                "c({})",
                v.iter().map(|s| r_text(s)).collect::<Vec<_>>().join(", ")
            )
        });
        out.line(format!(
            "{d} <- stata_spset({d}, id = {id}, coord = {co}, modify = {}, show = {})",
            super::stats::r_logical(modify),
            super::stats::r_logical(!self.quiet(c))
        ));
        if let Some(w) = words.first() {
            let st = self.storage.get(*w).copied().unwrap_or(Storage::Double);
            self.storage.insert("_ID".into(), st);
            self.kinds.insert("_ID".into(), Kind::Number);
        }
        if coord.is_some() {
            for v in ["_CX", "_CY"] {
                self.storage.insert(v.into(), Storage::Double);
                self.kinds.insert(v.into(), Kind::Number);
            }
        }
        Ok(())
    }

    /// The `normalize()` a matrix asks for, or Stata's error.
    fn sp_normalize(opts: &Opts) -> Result<String, String> {
        match opts.arg("normalize", 4) {
            None => Ok("spectral".into()),
            Some(a) => {
                let a = a.trim().to_string();
                ["spectral", "row", "minmax", "none"]
                    .iter()
                    .find(|f| !a.is_empty() && f.starts_with(a.as_str()))
                    .map(|f| (*f).to_string())
                    .ok_or(a)
            }
        }
    }

    /// Stata's `option name not allowed`, `name()` when it took an argument.
    fn sp_not_allowed(c: &Command, known: &[(&str, usize)]) -> Option<String> {
        c.options
            .iter()
            .find(|o| !known.iter().any(|(f, m)| o.is(f, *m)))
            .map(|o| {
                let shown = if o.raw.is_some() {
                    format!("{}()", o.name)
                } else {
                    o.name.clone()
                };
                format!("option {shown} not allowed")
            })
    }

    /// `spmatrix subcommand ...`: create idistance, dir, summarize, drop, copy,
    /// note, save, use, export, import and fromdata.
    #[allow(clippy::too_many_lines)]
    pub(super) fn spmatrix(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "spmatrix")?;
        if c.parts.weight.is_some() {
            return Err(no("spmatrix with weights is not translated"));
        }
        let main = c.parts.main.trim().to_string();
        let (sub, rest) = first_word(&main);
        let sub = sub.to_string();
        let rest = rest.to_string();
        let selects = c.parts.if_exp.is_some() || c.parts.in_range.is_some();
        if selects && !matches!(sub.as_str(), "create" | "fromdata") {
            return Err(no(format!(
                "spmatrix {sub} with if or in is not translated"
            )));
        }
        self.sp_helpers();
        let quiet = self.quiet(c);
        match sub.as_str() {
            "" => Self::sp_fail(out, "No subcommand specified", 198),
            "create" => {
                let (kind, name) = first_word(&rest);
                match kind {
                    "" => return Self::sp_fail(out, " unknown subcommand", 198),
                    "idistance" => {}
                    "contiguity" => {
                        let Ok(d) = self.data() else {
                            return Self::sp_fail(out, "data not spset", 459);
                        };
                        out.line(format!("stata_sp_spset_or_stop({d})"));
                        return Self::sp_fail(
                            out,
                            "Sp data not linked to shapefile\n    Type spset, modify filename(<filename>_shp.dta) to link a _shp.dta file to the dataset in memory.",
                            601,
                        );
                    }
                    k => return Self::sp_fail(out, &format!("{k} unknown subcommand"), 198),
                }
                let Ok(d) = self.data() else {
                    return Self::sp_fail(out, "data not spset", 459);
                };
                out.line(format!("stata_sp_spset_or_stop({d})"));
                if let Some(msg) =
                    Self::sp_not_allowed(c, &[("normalize", 4), ("vtruncate", 4), ("replace", 7)])
                {
                    return Self::sp_fail(out, &msg, 198);
                }
                if name.is_empty() {
                    return Self::sp_fail(out, "weighting matrix name required", 100);
                }
                if !is_name(name) {
                    return Err(no("spmatrix create: one matrix name is translated"));
                }
                let norm = match Self::sp_normalize(opts) {
                    Ok(n) => n,
                    Err(a) => {
                        return Self::sp_fail(out, &format!("unrecognized normalize() : {a} "), 199)
                    }
                };
                let vt = match opts.arg("vtruncate", 4) {
                    None => 0.0,
                    Some(a) => a.trim().parse::<f64>().map_err(|_| {
                        no(format!("spmatrix create, vtruncate({a}) is not translated"))
                    })?,
                };
                let sel = self.sp_rows(c, &d, out)?;
                out.line(format!(
                    "stata_spmatrix_create({d}, {sel}, {}, norm = {}, vtrunc = {}, replace = {})",
                    r_text(name),
                    r_text(&norm),
                    r_number(vt),
                    super::stats::r_logical(opts.has("replace", 7))
                ));
                Ok(())
            }
            "dir" => {
                if !rest.is_empty() || !c.options.is_empty() {
                    return Err(no("spmatrix dir takes nothing more"));
                }
                out.line("stata_spmatrix_dir()");
                Ok(())
            }
            s if s.len() >= 4 && "summarize".starts_with(s) => {
                if !c.options.is_empty() {
                    return Err(no("spmatrix summarize with options is not translated"));
                }
                let names: Vec<&str> = rest.split_whitespace().collect();
                match names.as_slice() {
                    [] => Self::sp_fail(out, "weighting matrix required", 100),
                    [one] if is_name(one) => {
                        out.line(format!(
                            "stata_spmatrix_summarize({}, show = {})",
                            r_text(one),
                            super::stats::r_logical(!quiet)
                        ));
                        self.r_new = Some((SP_R_TEXTS, "spmatrix".into()));
                        Ok(())
                    }
                    _ => Err(no("spmatrix summarize: one matrix name is translated")),
                }
            }
            "drop" => {
                let names: Vec<&str> = rest.split_whitespace().collect();
                match names.as_slice() {
                    [] => Self::sp_fail(out, "weighting matrix required", 100),
                    [one] => {
                        out.line(format!("stata_spmatrix_drop({})", r_text(one)));
                        Ok(())
                    }
                    _ => Self::sp_fail(out, "weighting matrix:  too many specified", 103),
                }
            }
            "copy" => {
                if !c.parts.options.trim().is_empty() {
                    return Self::sp_fail(out, "options not allowed", 198);
                }
                let names: Vec<&str> = rest.split_whitespace().collect();
                match names.as_slice() {
                    [a, b] => {
                        out.line(format!("stata_spmatrix_copy({}, {})", r_text(a), r_text(b)));
                        Ok(())
                    }
                    [] | [_] => Self::sp_fail(out, "weighting matrices:  too few specified", 102),
                    _ => Self::sp_fail(out, "weighting matrices:  too many specified", 103),
                }
            }
            "note" => {
                if !c.parts.options.trim().is_empty() {
                    return Self::sp_fail(out, "invalid syntax", 198);
                }
                match rest.split_once(':') {
                    Some((name, text)) => {
                        out.line(format!(
                            "stata_spmatrix_note({}, {})",
                            r_text(name.trim()),
                            r_text(text.trim())
                        ));
                    }
                    None => {
                        out.line(format!("stata_spmatrix_note({})", r_text(rest.trim())));
                    }
                }
                Ok(())
            }
            "save" | "use" | "export" | "import" => {
                let Some(file) = &c.using else {
                    return Err(no(format!("spmatrix {sub} needs using file")));
                };
                let name = rest.trim();
                if !is_name(name) {
                    return Err(no(format!("spmatrix {sub}: one matrix name is translated")));
                }
                let known: &[(&str, usize)] = &[("replace", 7)];
                if let Some(msg) = Self::sp_not_allowed(c, known) {
                    return Self::sp_fail(out, &msg, 198);
                }
                let ext = if matches!(sub.as_str(), "save" | "use") {
                    "stswm"
                } else {
                    "txt"
                };
                let f = file_name(file, ext);
                let replace = if sub == "use" {
                    String::new()
                } else {
                    format!(
                        ", replace = {}",
                        super::stats::r_logical(opts.has("replace", 7))
                    )
                };
                out.line(format!(
                    "stata_spmatrix_{sub}({}, {}, {}{replace})",
                    r_text(name),
                    r_text(&f),
                    r_text(&f)
                ));
                if sub == "save" || sub == "use" {
                    out.caveat("spmatrix save and use keep RCraft's own file (R's format): Stata's .stswm files are not read");
                }
                Ok(())
            }
            "fromdata" => {
                let Some((name, vars)) = rest.split_once('=') else {
                    return Err(no("spmatrix fromdata needs name = varlist"));
                };
                let name = name.trim();
                let vars: Vec<&str> = vars.split_whitespace().collect();
                if !is_name(name) || vars.is_empty() || !vars.iter().all(|v| is_name(v)) {
                    return Err(no(
                        "spmatrix fromdata is translated with a name and variable names",
                    ));
                }
                let Ok(d) = self.data() else {
                    return Self::sp_fail(out, "data not spset", 459);
                };
                out.line(format!("stata_sp_spset_or_stop({d})"));
                if let Some(msg) = Self::sp_not_allowed(c, &[("normalize", 4), ("replace", 7)]) {
                    return Self::sp_fail(out, &msg, 198);
                }
                let norm = match Self::sp_normalize(opts) {
                    Ok(n) => n,
                    Err(a) => {
                        return Self::sp_fail(out, &format!("unrecognized normalize() : {a} "), 199)
                    }
                };
                let sel = self.sp_rows(c, &d, out)?;
                out.line(format!(
                    "stata_spmatrix_fromdata({d}, {sel}, {}, c({}), norm = {}, replace = {})",
                    r_text(name),
                    vars.iter()
                        .map(|v| r_text(v))
                        .collect::<Vec<_>>()
                        .join(", "),
                    r_text(&norm),
                    super::stats::r_logical(opts.has("replace", 7))
                ));
                Ok(())
            }
            "matafromsp" | "spfrommata" => Err(no(format!(
                "spmatrix {sub} (weighting matrices in Mata) is not translated yet"
            ))),
            "normalize" | "userdefined" | "clear" => {
                Err(no(format!("spmatrix {sub} is not translated yet")))
            }
            other => Self::sp_fail(out, &format!("{other} unknown subcommand"), 198),
        }
    }
}

/// The r() spmatrix summarize leaves that are text (the others are numbers).
const SP_R_TEXTS: &[&str] = &["type", "normalization"];

#[cfg(test)]
mod tests {
    use crate::data_r::translate;
    use crate::reader::read;

    fn r(source: &str) -> Vec<String> {
        translate(&read(source))
            .lines
            .into_iter()
            .map(|l| match l.result {
                Ok(r) => r.code,
                Err(e) => format!("NOT: {}", e.reason),
            })
            .collect()
    }

    const DATA: &str = "clear\nset obs 4\ngenerate int id = _n\ngenerate x = _n\n";

    #[test]
    fn spset_makes_the_id_and_coordinates() {
        let got = r(&format!("{DATA}spset id, coord(x x)\nspset, modify coord(x)\nspset, coordsys(latlong)\nspset id, clear\n"));
        assert!(
            got[4]
                .contains("stata_spset(data, id = \"id\", coord = c(\"x\", \"x\"), modify = FALSE"),
            "{}",
            got[4]
        );
        assert!(
            got[5].contains("coord():  too few variables specified (Stata: r(102))"),
            "{}",
            got[5]
        );
        assert!(
            got[6].starts_with("NOT: spset, coordsys(latlong)"),
            "{}",
            got[6]
        );
        assert!(
            got[7].starts_with("NOT: spset with the option clear"),
            "{}",
            got[7]
        );
        // with no data in memory, Stata's error
        assert!(r("spset\n")[0].contains("no dataset in use (Stata: r(3))"));
    }

    #[test]
    fn spmatrix_subcommands_and_statas_errors() {
        let got = r(&format!(
            "{DATA}spset id, coord(x x)\nspmatrix create idistance W if x > 1, normalize(row) vtruncate(.5) replace\n\
             spmatrix create idistance W2, normalize(bogus)\nspmatrix create idistance W3, dtruncate(2)\n\
             spmatrix create idistance\nspmatrix create bogus Z\nspmatrix\nspmatrix summarize W\n\
             spmatrix drop W W2\nspmatrix copy W\nspmatrix note W: a note\nspmatrix export W using f, replace\n\
             spmatrix matafromsp M ids = W\nspregress x id, ml dvarlag(W)\n"
        ));
        assert!(
            got[5].contains("(is.na(x) | x > 1)")
                && got[5].contains("\"W\", norm = \"row\", vtrunc = 0.5, replace = TRUE)"),
            "{}",
            got[5]
        );
        assert!(
            got[6].contains("unrecognized normalize() : bogus  (Stata: r(199))"),
            "{}",
            got[6]
        );
        assert!(
            got[7].contains("option dtruncate() not allowed (Stata: r(198))"),
            "{}",
            got[7]
        );
        assert!(
            got[8].contains("weighting matrix name required (Stata: r(100))"),
            "{}",
            got[8]
        );
        assert!(
            got[9].contains("bogus unknown subcommand (Stata: r(198))"),
            "{}",
            got[9]
        );
        assert!(
            got[10].contains("No subcommand specified (Stata: r(198))"),
            "{}",
            got[10]
        );
        assert_eq!(got[11], "stata_spmatrix_summarize(\"W\", show = TRUE)");
        assert!(
            got[12].contains("weighting matrix:  too many specified (Stata: r(103))"),
            "{}",
            got[12]
        );
        assert!(
            got[13].contains("weighting matrices:  too few specified (Stata: r(102))"),
            "{}",
            got[13]
        );
        assert_eq!(got[14], "stata_spmatrix_note(\"W\", \"a note\")");
        assert_eq!(
            got[15],
            "stata_spmatrix_export(\"W\", \"f.txt\", \"f.txt\", replace = TRUE)"
        );
        assert!(
            got[16].starts_with("NOT: spmatrix matafromsp"),
            "{}",
            got[16]
        );
        assert!(
            got[17].starts_with("NOT: spregress is not translated yet"),
            "{}",
            got[17]
        );
    }

    #[test]
    fn summarize_leaves_its_r_values() {
        let got = r(&format!(
            "{DATA}spset id, coord(x x)\nspmatrix create idistance W\nspmatrix summarize W\ndisplay r(max) r(type)\n"
        ));
        assert!(got[7].contains("stata_rget(\"max\")"), "{}", got[7]);
        assert!(
            got[7].contains("stata_rget(\"type\", text = TRUE)"),
            "{}",
            got[7]
        );
    }
}
