//! `statsby` and `rolling` (batch I of the StatCraft port): a command run on
//! each group of the data, or on each window of the time series, and the
//! dataset of results they leave. The rules are in `plan/stata/resampling.md`.

use super::resample::{exp_items, split_prefixed};
use super::*;

/// R's `statsby` and `rolling`: the runs of a command on groups or windows
/// of the data, and the dataset of results they leave. The statistics are
/// kept as floats, as Stata keeps them; `_b` and `_se` give a variable for
/// each coefficient (`_b_x`, `_b_cons`, `_se_x`).
pub(super) const STATSBY_HELPER: &str = r##"stata_sb_f <- function(v) {
  r <- stata_float(v)
  names(r) <- names(v)
  r
}
stata_sb_vec <- function(se = FALSE) {
  b <- stata_bs_b()
  v <- if (se) {
    s <- sqrt(diag(stata_e$V))
    nmf <- colnames(stata_e$b)
    keep <- !grepl("(^|[:#])[0-9]*[bo]n?\\.", nmf) & !grepl("(^|:)o\\.", nmf)
    if (length(s) == length(nmf)) s[keep] else { names(s) <- colnames(stata_e$V); s[names(b)] }
  } else b
  out <- as.vector(v)
  # logit and probit name their variables after the equation (yb_b_x, labelled [yb]_b[x])
  dep <- if (!is.null(stata_e$cmd) && stata_e$cmd %in% c("logit", "probit")) stata_e$depvar else NULL
  kind <- if (se) "se" else "b"
  nm <- paste0(if (is.null(dep)) "" else dep, "_", kind, "_", ifelse(names(b) == "_cons", "cons", names(b)))
  names(out) <- nm
  for (j in seq_along(nm)) {
    assign(nm[j], list(kind = kind, eq = dep, coef = names(b)[j],
                       label = paste0(if (is.null(dep)) "" else paste0("[", dep, "]"), "_", kind, "[", names(b)[j], "]")),
           envir = stata_sb_reg)
  }
  out
}
stata_sb_reg <- new.env()
stata_sb_write <- function(out, path, label) {
  haven::write_dta(out, path, label = label)
  invisible(NULL)
}
stata_sb_begin <- function(cmdname, command, names, exps, by, nodots, header = TRUE, saving = NULL, replace = FALSE) {
  if (!is.null(saving) && !replace && file.exists(saving)) stop("file ", saving, " already exists (Stata: r(602))", call. = FALSE)
  if (!nodots) cat("(running ", cmdname, " on estimation sample)\n", sep = "")
  if (header) {
    pad <- function(s) formatC(s, width = 13)
    cat("\n", pad("Command"), ": ", command, "\n", sep = "")
    for (j in seq_along(names)) cat(pad(names[j]), ": ", exps[j], "\n", sep = "")
    cat(pad("By"), ": ", if (length(by)) paste(by, collapse = " ") else "<none>", "\n\n", sep = "")
    if (!is.null(saving) && !file.exists(saving)) cat("(file ", saving, " not found)\n", sep = "")
    if (!nodots) cat("Statsby groups:\n")
  }
  invisible(NULL)
}
stata_sb_groups <- function(d, by, missing = FALSE) {
  for (v in by) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  keep <- rep(TRUE, nrow(d))
  if (!missing) for (v in by) {
    x <- d[[v]]
    keep <- keep & !is.na(x) & !(is.character(x) & x == "")
  }
  rows <- which(keep)
  if (!length(by)) return(list(rows = list(rows), first = list(rows[1])))
  key <- lapply(by, function(v) { x <- d[[v]][rows]; attributes(x) <- NULL; x })
  ord <- do.call(order, c(key, list(na.last = TRUE, method = "radix")))
  rows <- rows[ord]
  key <- lapply(key, function(x) x[ord])
  ks <- do.call(paste, c(lapply(key, as.character), sep = "\r"))
  gi <- match(ks, unique(ks))
  g <- unname(split(rows, gi))
  list(rows = g, first = g)
}
stata_sb_table <- function(res, exps = character()) {
  ok <- Filter(Negate(is.null), res)
  nm <- unique(unlist(lapply(ok, names)))
  m <- matrix(NA_real_, length(res), length(nm), dimnames = list(NULL, nm))
  for (i in seq_along(res)) if (!is.null(res[[i]])) m[i, names(res[[i]])] <- res[[i]]
  lab <- vapply(nm, function(n) {
    if (n %in% names(exps)) return(exps[[n]])
    info <- get0(n, envir = stata_sb_reg, inherits = FALSE)
    if (!is.null(info)) return(info$label)
    n
  }, "")
  list(m = m, lab = lab)
}
stata_sb_result <- function(full, g, res, by, total = FALSE, exps = character()) {
  tb <- stata_sb_table(res, exps)
  out <- list()
  for (v in by) {
    x <- full[[v]][vapply(g$first, `[`, 0L, 1L)]
    if (total) x <- c(x, if (is.character(x)) "" else NA)
    mostattributes(x) <- attributes(full[[v]])
    out[[v]] <- x
  }
  for (j in seq_len(ncol(tb$m))) {
    nm <- colnames(tb$m)[j]
    out[[nm]] <- structure(stata_float(tb$m[, j]), label = unname(tb$lab[j]), format.stata = "%9.0g")
  }
  out <- as.data.frame(out, stringsAsFactors = FALSE, optional = TRUE)
  rownames(out) <- NULL
  out
}
stata_roll_windows <- function(d, time, window, step = 1, recursive = FALSE, delta = 1, rrecursive = FALSE) {
  if (!time %in% names(d)) stop("variable ", time, " not found (Stata: r(111))", call. = FALSE)
  tv <- d[[time]]
  attributes(tv) <- NULL
  if (anyNA(tv) || is.unsorted(tv)) stop("rolling with a time variable that is missing or not in order is not translated yet", call. = FALSE)
  # a window is `window` periods of the time variable, whether or not each has an observation
  lo <- min(tv)
  hi <- max(tv)
  span <- (window - 1) * delta
  if (hi - lo < span - 1e-9 * abs(delta)) stop("window() is larger than the sample (Stata: r(2001))", call. = FALSE)
  k <- floor((hi - lo - span) / (step * delta) + 1e-9) + 1
  ends <- lo + span + (seq_len(k) - 1) * step * delta
  starts <- if (recursive) rep(lo, length(ends)) else ends - span
  # rrecursive: the end stays at the last period, the start moves on
  if (rrecursive) {
    starts <- ends - span
    ends <- rep(hi, length(starts))
  }
  tol <- 1e-9 * max(abs(delta), 1)
  rows <- lapply(seq_along(ends), function(i) which(tv >= starts[i] - tol & tv <= ends[i] + tol))
  list(start = starts, end = ends, rows = rows)
}
stata_roll_result <- function(full, time, w, res, exps = character()) {
  tb <- stata_sb_table(res, exps)
  out <- list()
  for (nm in c("start", "end")) {
    x <- w[[nm]]
    mostattributes(x) <- attributes(full[[time]])
    out[[nm]] <- x
  }
  for (j in seq_len(ncol(tb$m))) {
    nm <- colnames(tb$m)[j]
    out[[nm]] <- structure(stata_float(tb$m[, j]), label = unname(tb$lab[j]), format.stata = "%9.0g")
  }
  out <- as.data.frame(out, stringsAsFactors = FALSE, optional = TRUE)
  rownames(out) <- NULL
  out
}"##;

/// Commands after which Stata 19 still counts the data as saved (each checked:
/// `statsby` without `clear` then runs instead of stopping with r(4)).
const KEEP_SAVED: &[&str] = &[
    "sort",
    "gsort",
    "tsset",
    "xtset",
    "order",
    "summarize",
    "list",
    "describe",
    "display",
    "count",
    "tabulate",
    "codebook",
    "correlate",
    "ttest",
    "regress",
    "logit",
    "return",
    "ereturn",
    "estimates",
    "preserve",
    "restore",
    "frame",
    "frames",
    "cwf",
    "pwf",
];

/// Commands that leave the data changed since they were saved (checked as above).
const CHANGE_SAVED: &[&str] = &["generate", "label", "format", "import", "insheet", "input"];

/// Whether Stata counts the data as saved after the command `c` (`None`: not known).
pub(super) fn saved_after(c: &Command, ok: bool, before: Option<bool>) -> Option<bool> {
    let name = c.name.as_str();
    if !ok {
        return None;
    }
    match name {
        // the data a plain `use file` reads, or what `save` wrote, are saved; no data is no change
        "use" if c.parts.if_exp.is_none() && c.parts.in_range.is_none() && c.using.is_none() => {
            Some(true)
        }
        "save" | "saveold" | "clear" => Some(true),
        "set" if c.parts.main.trim_start().starts_with("obs") => Some(false),
        _ if KEEP_SAVED.contains(&name) => before,
        _ if CHANGE_SAVED.contains(&name) => Some(false),
        _ => None,
    }
}

/// `saving = path, replace = flag` for the R call, when `saving()` was given.
fn saving_args(saving: &Option<(String, bool, bool)>) -> String {
    saving.as_ref().map_or(String::new(), |(p, r, _)| {
        format!(
            ", saving = {}, replace = {}",
            super::r_text(p),
            super::stats::r_logical(*r)
        )
    })
}

/// The R of each statistic, and the named expressions with their texts.
type Stats = (Vec<String>, Vec<(String, String)>);

/// The helpers every run of a command on parts of the data needs.
const NEEDS: [&str; 7] = [
    "stata_rng",
    "stata_bootstrap",
    "stata_statsby",
    "stata_fmt",
    "stata_float",
    "stata_names",
    "stata_returned",
];

impl Session {
    /// The statistics of `statsby` and `rolling`: `_b`, `_se`, `name=exp`
    /// and `exp` (named `_stat_1`, ...), each as an R expression giving a
    /// named vector; and the named expressions with their texts.
    fn vector_stats(
        &mut self,
        prefix: &str,
        explist: &str,
        inner: &Command,
        eclass: bool,
        body: &mut Out,
    ) -> Result<Stats, Untranslated> {
        let items = exp_items(explist);
        let items = if items.is_empty() && eclass {
            vec!["_b".to_string()]
        } else {
            items
        };
        if items.is_empty() {
            return Err(no(format!(
                "{prefix}: expression list required (Stata: r(100))"
            )));
        }
        let mut codes = Vec::new();
        let mut named = Vec::new();
        let mut unnamed = 0;
        for item in &items {
            if item == "_b" || item == "_se" {
                if !eclass || !matches!(inner.name.as_str(), "regress" | "logit" | "probit") {
                    return Err(no(format!(
                        "{prefix} {item} is translated after regress, logit and probit; not here yet"
                    )));
                }
                codes.push(format!(
                    "stata_sb_vec({})",
                    if item == "_se" { "TRUE" } else { "FALSE" }
                ));
                continue;
            }
            let (name, exp) = match item.split_once('=') {
                Some((n, e)) if super::is_name(n.trim()) && !e.starts_with('=') => {
                    (n.trim().to_string(), e.trim().to_string())
                }
                _ => {
                    unnamed += 1;
                    (format!("_stat_{unnamed}"), item.clone())
                }
            };
            let exp = exp
                .strip_prefix('(')
                .and_then(|e| e.strip_suffix(')'))
                .map_or(exp.clone(), str::to_string);
            let (code, kind) = self.value(&exp, body)?;
            if kind == Kind::Text {
                return Err(no(format!("{exp} is a string (Stata: type mismatch)")));
            }
            codes.push(format!("stats::setNames({code}, {})", super::r_text(&name)));
            named.push((name, exp));
        }
        Ok((codes, named))
    }

    /// Translates the inner command once (to learn its R and what it
    /// leaves), with `quietly` on it, as a prefix command does.
    fn prefix_inner(
        &mut self,
        prefix: &str,
        inner_text: &str,
        body: &mut Out,
    ) -> Result<(Command, bool), Untranslated> {
        let program = crate::reader::read(inner_text);
        let mut inner = match program.commands().as_slice() {
            [one] => (*one).clone(),
            _ => {
                return Err(no(format!(
                    "{prefix} of more than one command is not translated"
                )))
            }
        };
        if !inner.prefixes.is_empty()
            || matches!(
                inner.name.as_str(),
                "bootstrap"
                    | "bsample"
                    | "jackknife"
                    | "permute"
                    | "simulate"
                    | "statsby"
                    | "rolling"
            )
        {
            return Err(no(format!(
                "{prefix} of {} with a prefix, or of another resampling command, is not translated yet",
                inner.name
            )));
        }
        if inner.parts.if_exp.is_some() || inner.parts.in_range.is_some() {
            return Err(no(format!(
                "{prefix} of a command with if or in is not translated yet"
            )));
        }
        inner.prefixes.push(crate::reader::Prefix::Quietly);
        self.e_new = false;
        self.r_new = None;
        self.dispatch(&inner, body)?;
        let eclass = self.e_new;
        let r_left = self.r_new.take();
        self.track_returned(&inner, true, false, r_left);
        Ok((inner, eclass))
    }

    /// What `statsby` and `rolling` leave behind: the data are the results,
    /// floats but for the by variables; no model, no `tsset`, no r().
    fn resampled_data(&mut self, keep: &[String]) {
        let by_kinds: Vec<(String, Kind)> = keep
            .iter()
            .filter_map(|v| self.kinds.get(v).map(|k| (v.clone(), *k)))
            .collect();
        self.kinds.clear();
        for (v, k) in by_kinds {
            self.kinds.insert(v, k);
        }
        self.storage.retain(|k, _| keep.contains(k));
        self.forget_ts();
        self.esample = false;
        self.drew.set(false);
        self.estimates = None;
        self.estimates_lost = Some(
            "the data are the results of a resampling command: the model before them is gone"
                .into(),
        );
        self.e_ret = super::returned::Returned::Lost(
            "e() after statsby or rolling is the last run of the command, which RCraft does not keep"
                .into(),
        );
        self.chars_lost = Some("statsby and rolling replace the data".into());
    }

    /// With no `clear` the data are as they were; the model and r() are not known.
    fn keep_data_after_resampling(&mut self) {
        self.esample = false;
        self.drew.set(false);
        self.estimates = None;
        self.estimates_lost = Some(
            "statsby or rolling ran the command on parts of the data: the model before them is gone".into(),
        );
        self.e_ret = super::returned::Returned::Lost(
            "e() after statsby or rolling is the last run of the command, which RCraft does not keep".into(),
        );
    }

    fn take_helpers(&mut self, body: &mut Out, out: &mut Out) {
        for h in NEEDS {
            self.helpers.insert(h);
        }
        for h in std::mem::take(&mut body.helpers) {
            self.helpers.insert(h);
        }
        for pk in &body.packages {
            out.package(pk);
        }
    }

    /// `statsby exp_list [, by(varlist) clear total nodots missing]: command`.
    pub(super) fn statsby(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        self.no_by(c, "statsby")?;
        let p = split_prefixed(c, "statsby")?;
        let opt_list = p.options.clone();
        let opts = Opts(&opt_list);
        if let Some(o) = opts.other(&[
            ("by", 2),
            ("clear", 5),
            ("total", 5),
            ("nodots", 4),
            ("saving", 4),
        ]) {
            return Err(no(format!(
                "statsby with the option {o} is not translated yet"
            )));
        }
        let clear = opts.has("clear", 5);
        let saving = match opts.arg("saving", 4) {
            Some(sv) => Some(super::permute_run::saving_of(&sv)?),
            None => None,
        };
        if saving.as_ref().is_some_and(|sv| sv.2) {
            return Err(no("statsby, saving(, double) is not translated yet"));
        }
        if clear && saving.is_some() {
            out.line("stop(\"clear and saving() are mutually exclusive options (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        }
        // neither: Stata replaces data it counts as saved, and stops with r(4) otherwise
        let mut clear = clear;
        if !clear && saving.is_none() {
            match self.saved_now {
                Some(true) => clear = true,
                Some(false) => {
                    out.line("stop(\"no; dataset in memory has changed since last saved (Stata: r(4))\", call. = FALSE)");
                    return Ok(());
                }
                None => {
                    return Err(no(
                        "statsby without clear or saving(): Stata stops (r(4)) when the data have changed since they were saved, which RCraft does not know here",
                    ))
                }
            }
        }
        let by_arg = opts.arg("by", 2).unwrap_or_default();
        let (by_list, by_rest) = match by_arg.split_once(',') {
            Some((a, b)) => (a.to_string(), b.trim().to_string()),
            None => (by_arg.clone(), String::new()),
        };
        if !by_rest.is_empty() && by_rest != "missing" {
            return Err(no(format!(
                "statsby, by(, {by_rest}) is not translated yet"
            )));
        }
        let by: Vec<String> = by_list.split_whitespace().map(str::to_string).collect();
        if by.iter().any(|v| !super::is_name(v)) {
            return Err(no("statsby, by() takes plain variable names"));
        }
        let total = opts.has("total", 5);
        let nodots = opts.has("nodots", 4);
        let missing = by_rest == "missing";
        let d = self.data()?;
        let mut body = Out::default();
        let (inner, eclass) = self.prefix_inner("statsby", &p.inner, &mut body)?;
        let (codes, named) = self.vector_stats("statsby", &p.explist, &inner, eclass, &mut body)?;
        self.take_helpers(&mut body, out);
        let code = body.code.join("\n");
        let q = |v: &[String]| {
            v.iter()
                .map(|s| super::r_text(s))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let by_r = q(&by);
        let names: Vec<String> = named.iter().map(|(n, _)| n.clone()).collect();
        let exps: Vec<String> = named.iter().map(|(_, e)| e.clone()).collect();
        let exp_labels = named
            .iter()
            .map(|(n, e)| format!("{} = {}", super::r_text(n), super::r_text(e)))
            .collect::<Vec<_>>()
            .join(", ");
        let tf = super::stats::r_logical;
        out.line(format!("stata_sb_full <- {d}"));
        out.line(format!(
            "stata_sb_g <- stata_sb_groups(stata_sb_full, c({by_r}), missing = {})",
            tf(missing)
        ));
        out.line(format!(
            "stata_sb_rows <- c(stata_sb_g$rows{})",
            if total {
                ", list(seq_len(nrow(stata_sb_full)))"
            } else {
                ""
            }
        ));
        out.line(format!(
            "stata_sb_begin({}, {}, c({}), c({}), c({by_r}), {}{})",
            super::r_text(&inner.name),
            super::r_text(&p.inner),
            q(&names),
            q(&exps),
            tf(nodots),
            saving_args(&saving)
        ));
        out.line("stata_sb_res <- vector(\"list\", length(stata_sb_rows))");
        out.line(format!(
            "for (stata_sb_i in seq_along(stata_sb_rows)) {{\n{d} <- stata_sb_full[stata_sb_rows[[stata_sb_i]], , drop = FALSE]\nstata_sb_res[stata_sb_i] <- list(tryCatch({{\n{code}\nstata_sb_f(c({}))\n}}, error = function(e) NULL))\nif (!{} && stata_sb_i <= length(stata_sb_g$rows)) cat(\".\")\n}}\nif (!{}) cat(\"\\n\")",
            codes.join(", "),
            tf(nodots),
            tf(nodots)
        ));
        out.line(format!(
            "stata_sb_out <- stata_sb_result(stata_sb_full, stata_sb_g, stata_sb_res, c({by_r}), total = {}, exps = c({exp_labels}))",
            tf(total)
        ));
        if let Some((path, _, _)) = &saving {
            out.package("haven");
            out.line(format!(
                "stata_sb_write(stata_sb_out, {}, {})",
                super::r_text(path),
                super::r_text(&format!("statsby: {}", inner.name))
            ));
        }
        // without clear the data in memory stay, sorted by the by() variables
        out.line(format!(
            "{d} <- {}",
            if clear {
                "stata_sb_out".to_string()
            } else if by.is_empty() {
                "stata_sb_full".to_string()
            } else {
                format!(
                    "stata_sb_full[do.call(order, c(lapply(c({by_r}), function(v) {{ x <- stata_sb_full[[v]]; attributes(x) <- NULL; x }}), list(na.last = TRUE, method = \"radix\"))), , drop = FALSE]"
                )
            }
        ));
        out.line("rm(stata_sb_full, stata_sb_g, stata_sb_rows, stata_sb_res, stata_sb_out)");
        if clear {
            self.resampled_data(&by);
            for n in &names {
                self.kinds.insert(n.clone(), Kind::Number);
                self.storage.insert(n.clone(), Storage::Float);
            }
        } else {
            self.keep_data_after_resampling();
            out.caveat("Stata sorts the data by the by() variables; rows with equal values may come in another order there");
        }
        out.caveat("each group runs the command on that group's rows; the statistics are kept as floats, as Stata keeps them");
        Ok(())
    }

    /// `rolling [exp_list], window(#) [recursive stepsize(#) clear nodots]:
    /// command` over the data `tsset` described.
    pub(super) fn rolling(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        self.no_by(c, "rolling")?;
        let p = split_prefixed(c, "rolling")?;
        let opt_list = p.options.clone();
        let opts = Opts(&opt_list);
        if let Some(o) = opts.other(&[
            ("window", 6),
            ("recursive", 9),
            ("rrecursive", 10),
            ("stepsize", 4),
            ("clear", 5),
            ("nodots", 4),
            ("saving", 4),
        ]) {
            return Err(no(format!(
                "rolling with the option {o} is not translated yet"
            )));
        }
        let clear = opts.has("clear", 5);
        let saving = match opts.arg("saving", 4) {
            Some(sv) => Some(super::permute_run::saving_of(&sv)?),
            None => None,
        };
        if saving.as_ref().is_some_and(|sv| sv.2) {
            return Err(no("rolling, saving(, double) is not translated yet"));
        }
        if clear && saving.is_some() {
            out.line("stop(\"clear and saving() are mutually exclusive options (Stata: r(198))\", call. = FALSE)");
            return Ok(());
        }
        // neither: Stata replaces data it counts as saved, and stops with r(4) otherwise
        let mut clear = clear;
        if !clear && saving.is_none() {
            match self.saved_now {
                Some(true) => clear = true,
                Some(false) => {
                    out.line("stop(\"no; dataset in memory has changed since last saved (Stata: r(4))\", call. = FALSE)");
                    return Ok(());
                }
                None => {
                    return Err(no(
                        "rolling without clear or saving(): Stata stops (r(4)) when the data have changed since they were saved, which RCraft does not know here",
                    ))
                }
            }
        }
        let window: u32 = opts
            .arg("window", 6)
            .and_then(|w| w.trim().parse().ok())
            .filter(|w| *w >= 1)
            .ok_or_else(|| no("rolling needs window(#) with a whole number (Stata: r(198))"))?;
        let step: u32 = match opts.arg("stepsize", 4) {
            None => 1,
            Some(s) => s
                .trim()
                .parse()
                .ok()
                .filter(|s| *s >= 1)
                .ok_or_else(|| no("stepsize(#) takes a whole number of 1 or more"))?,
        };
        let recursive = opts.has("recursive", 9);
        let rrecursive = opts.has("rrecursive", 10);
        if recursive && rrecursive {
            return Err(no(
                "rolling with recursive and rrecursive together is not translated",
            ));
        }
        let nodots = opts.has("nodots", 4);
        let (Some(time), None) = (self.panel_time.clone(), self.panel.clone()) else {
            return Err(no(
                "rolling is translated on data tsset by a time variable alone (not xtset, not unset)",
            ));
        };
        if self.ts_unknown {
            return Err(no("rolling: whether the data was tsset is not known here"));
        }
        let d = self.data()?;
        let mut body = Out::default();
        let (inner, eclass) = self.prefix_inner("rolling", &p.inner, &mut body)?;
        let (codes, named) = self.vector_stats("rolling", &p.explist, &inner, eclass, &mut body)?;
        self.take_helpers(&mut body, out);
        let code = body.code.join("\n");
        let exp_labels = named
            .iter()
            .map(|(n, e)| format!("{} = {}", super::r_text(n), super::r_text(e)))
            .collect::<Vec<_>>()
            .join(", ");
        let tf = super::stats::r_logical;
        out.line(format!("stata_sb_full <- {d}"));
        out.line(format!(
            "stata_sb_w <- stata_roll_windows(stata_sb_full, {}, {window}, {step}, {}, delta = {}, rrecursive = {})",
            super::r_text(&time),
            tf(recursive),
            self.ts_delta.clone().unwrap_or_else(|| "1".to_string()),
            tf(rrecursive)
        ));
        out.line(format!(
            "stata_sb_begin({}, {}, character(), character(), character(), {}, header = FALSE{})",
            super::r_text(&inner.name),
            super::r_text(&p.inner),
            tf(nodots),
            saving_args(&saving)
        ));
        out.line("stata_sb_n <- length(stata_sb_w$end)");
        out.line("stata_sb_res <- vector(\"list\", stata_sb_n)");
        out.line(format!(
            "if (!{}) cat(\"\\nRolling replications (\", stata_sb_n, \"): \", sep = \"\")",
            tf(nodots)
        ));
        out.line(format!(
            "for (stata_sb_i in seq_len(stata_sb_n)) {{\n{d} <- stata_sb_full[stata_sb_w$rows[[stata_sb_i]], , drop = FALSE]\nstata_sb_res[stata_sb_i] <- list(tryCatch({{\n{code}\nstata_sb_f(c({}))\n}}, error = function(e) NULL))\nstata_bs_dot(stata_sb_i, stata_sb_n, {})\n}}",
            codes.join(", "),
            tf(nodots)
        ));
        out.line(format!(
            "stata_sb_out <- stata_roll_result(stata_sb_full, {}, stata_sb_w, stata_sb_res, exps = c({exp_labels}))",
            super::r_text(&time)
        ));
        if let Some((path, _, _)) = &saving {
            out.package("haven");
            out.line(format!(
                "stata_sb_write(stata_sb_out, {}, {})",
                super::r_text(path),
                super::r_text(&format!("rolling: {}", inner.name))
            ));
        }
        out.line(format!(
            "{d} <- {}",
            if clear {
                "stata_sb_out"
            } else {
                "stata_sb_full"
            }
        ));
        out.line("rm(stata_sb_full, stata_sb_w, stata_sb_n, stata_sb_res, stata_sb_out)");
        if clear {
            self.resampled_data(&[]);
            for n in ["start", "end"] {
                self.kinds.insert(n.to_string(), Kind::Number);
            }
            for (n, _) in &named {
                self.kinds.insert(n.clone(), Kind::Number);
                self.storage.insert(n.clone(), Storage::Float);
            }
        } else {
            self.keep_data_after_resampling();
        }
        out.caveat("each window runs the command on its own rows of the data in time order; the statistics are kept as floats, as Stata keeps them");
        Ok(())
    }
}
