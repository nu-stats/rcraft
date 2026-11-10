//! The data commands that reshape, combine or convert: `recode`, `egen`,
//! `merge`, `append`, `reshape`, `collapse`, `duplicates`, `encode`,
//! `destring` and `tostring`. The rules are Stata's documented ones as
//! StatCraft implements them (`crates/engine/src/commands/{convert,egen,
//! combine,reshape,collapse,duplicates}.rs` there, each checked against Stata).

use super::*;

/// Small R functions with Stata's rules for statistics over a column:
/// missing values are skipped, and an all-missing column gives missing
/// (not R's `NaN`, `Inf` or an error).
pub(super) const STAT_HELPERS: &str = "stata_mean <- function(x) if (all(is.na(x))) NA_real_ else mean(x, na.rm = TRUE)\n\
stata_sd <- function(x) if (sum(!is.na(x)) < 2) NA_real_ else stats::sd(x, na.rm = TRUE)\n\
stata_min <- function(x) if (all(is.na(x))) NA_real_ else min(x, na.rm = TRUE)\n\
stata_max <- function(x) if (all(is.na(x))) NA_real_ else max(x, na.rm = TRUE)\n\
stata_pctile <- function(x, p) if (all(is.na(x))) NA_real_ else unname(stats::quantile(x, p / 100, type = 2, na.rm = TRUE))";

/// `collapse` where the data may hold `.a` to `.z`, as Stata 19 gives them:
/// (max) and (min) of a group with no number are its largest and smallest
/// missing value in Stata's order, (firstnm) and (lastnm) its first and last
/// value (`stata_mv` gives the order).
pub(super) const MV_STAT_HELPERS: &str = "stata_mv_max <- function(x) if (all(is.na(x))) x[which.max(stata_mv(x))] else max(x, na.rm = TRUE)\n\
stata_mv_min <- function(x) if (all(is.na(x))) x[which.min(stata_mv(x))] else min(x, na.rm = TRUE)\n\
stata_mv_firstnm <- function(x) { k <- which(!is.na(x)); x[if (length(k)) k[1] else 1] }\n\
stata_mv_lastnm <- function(x) { k <- which(!is.na(x)); x[if (length(k)) k[length(k)] else length(x)] }";

/// `append, force`: a variable of the using data whose kind (string or
/// number) is not the master's is missing there, `""` or `.` (Stata 19).
/// `merge` as Stata 19 does it (`plan/stata/merge.md`). Within a key, the
/// k-th row of the master goes with the k-th row of the using data, and the
/// shorter side's last row stands in for the rows it lacks: that is 1:1,
/// m:1, 1:m and m:m alike (`_n` keys on the row number). The master's rows
/// come first, sorted by the key; a using row beyond them, and a row only
/// the using data has, come after, sorted by the key. A variable in both
/// keeps the master's value, but with `update` a missing one takes the
/// using value (4) and two values that differ are a conflict (5), which
/// `replace` settles for the using value. The report, `assert()` (before
/// `keep()`, the merged data left in memory) and `keep()` as Stata has them.
pub(super) const MERGE_HELPER: &str = r#"stata_merge <- function(d, u, keys, kind, gen = "_merge", update = FALSE, replace = FALSE,
                        keep = NULL, assert = NULL, report = TRUE, generate = TRUE, target = NULL,
                        tags = FALSE) {
  for (k in keys) {
    if (!k %in% names(d)) stop("variable ", k, " not found")
    if (!k %in% names(u)) stop("variable ", k, " not found")
  }
  plain <- function(v) if (is.character(v)) v else as.numeric(v)
  key <- function(x) {
    if (length(keys) == 0) return(as.character(seq_len(nrow(x))))
    # led by a mark: R never looks up an empty name ("" is a key in Stata)
    do.call(paste, c("\002", lapply(keys, function(k) {
      v <- x[[k]]
      # a string is never missing in Stata: NA is ""
      # with tags, . and .a to .z are keys of their own (Stata 19)
      if (is.character(v)) ifelse(is.na(v), "", v) else ifelse(is.na(v), if (tags) paste0("\001", stata_mv(v)) else "\001", as.character(plain(v)))
    }), sep = "\r"))
  }
  sorted <- function(x) {
    if (length(keys) == 0) return(seq_len(nrow(x)))
    by <- unlist(lapply(keys, function(k) {
      p <- plain(x[[k]])
      if (tags && is.double(p)) list(p, stata_mv(p)) else list(p)
    }), recursive = FALSE)
    do.call(order, c(by, list(method = "radix", na.last = TRUE)))
  }
  where <- if (kind == "1:1") "master and using data" else if (kind == "1:m") "master data" else "using data"
  kd0 <- key(d)
  ku <- key(u)
  klist <- paste(keys, collapse = " ")
  if (kind %in% c("1:1", "1:m") && anyDuplicated(kd0)) stop("variable", if (length(keys) > 1) "s" else "", " ", klist, " do", if (length(keys) > 1) "" else "es", " not uniquely identify observations in the master data")
  if (kind %in% c("1:1", "m:1") && anyDuplicated(ku)) stop("variable", if (length(keys) > 1) "s" else "", " ", klist, " do", if (length(keys) > 1) "" else "es", " not uniquely identify observations in the using data")
  od <- sorted(d)
  kd <- kd0[od]
  jd <- stats::ave(seq_along(kd), kd, FUN = seq_along)
  ju <- stats::ave(seq_along(ku), ku, FUN = seq_along)
  nm <- table(kd)
  nu <- table(ku)
  nu_d <- as.vector(nu[kd])
  pidx <- unname(stats::setNames(seq_along(ku), paste(ku, ju, sep = "\r\r"))[paste(kd, pmin(jd, nu_d), sep = "\r\r")])
  nm_u <- as.vector(nm[ku])
  extra <- which(!is.na(nm_u) & ju > nm_u)
  only <- which(is.na(nm_u))
  last <- stats::setNames(seq_along(kd), paste(kd, jd, sep = "\r\r"))
  epart <- unname(last[paste(ku[extra], nm_u[extra], sep = "\r\r")])
  midx <- c(od, od[epart], rep(NA_integer_, length(only)))
  uidx <- c(pidx, extra, only)
  code <- c(ifelse(is.na(pidx), 1, 3), rep(3, length(extra)), rep(2, length(only)))
  # the rows after the master's own, sorted by the key
  after <- seq_along(midx) > length(od)
  if (any(after)) {
    tail_u <- u[uidx[after], , drop = FALSE]
    o <- which(after)[sorted(tail_u)]
    midx <- c(midx[!after], midx[o])
    uidx <- c(uidx[!after], uidx[o])
    code <- c(code[!after], code[o])
  }
  out <- d[midx, , drop = FALSE]
  new <- setdiff(names(u), names(d))
  if (length(new)) out <- dplyr::bind_cols(out, u[uidx, new, drop = FALSE])
  # a string missing from a row is empty in Stata, not NA
  for (v in names(out)) {
    if (is.character(out[[v]])) {
      gap <- if (v %in% names(d)) is.na(midx) else is.na(uidx)
      out[[v]][gap & is.na(out[[v]])] <- ""
    }
  }
  put <- function(x, i, y) {
    a <- attributes(x)
    xv <- as.vector(unclass(x))
    yv <- as.vector(unclass(y))
    if (is.character(xv) != is.character(yv)) stop("a variable is a string in one file and a number in the other (Stata: r(106))")
    xv[i] <- yv[i]
    attributes(xv) <- a
    xv
  }
  ux <- function(v) u[[v]][uidx]
  for (v in intersect(names(u), names(d))) {
    y <- ux(v)
    from_u <- is.na(midx)
    if (!v %in% keys && (update || replace)) {
      xv <- as.vector(unclass(out[[v]]))
      yv <- as.vector(unclass(y))
      miss <- function(z) is.na(z) | (is.character(z) & z == "")
      matched <- code >= 3
      upd <- matched & miss(xv) & !miss(yv)
      conf <- matched & !miss(xv) & !miss(yv) & xv != yv
      if (update) code[upd & code < 5] <- 4
      code[conf] <- 5
      take <- from_u | (update & upd) | (replace & conf)
    } else {
      take <- from_u
    }
    if (any(take)) out[[v]] <- put(out[[v]], take, y)
  }
  labs <- c("Master only (1)" = 1, "Using only (2)" = 2, "Matched (3)" = 3, "Missing updated (4)" = 4, "Nonmissing conflict (5)" = 5)
  if (!is.null(assert) && !all(code %in% assert)) {
    out[[gen]] <- haven::labelled(code, labs, label = "Matching result from merge")
    if (!is.null(target)) assign(target, out, envir = parent.frame())
    stop("after merge, not all observations matched\n(merged result left in memory)")
  }
  if (!is.null(keep)) {
    k <- code %in% keep
    out <- out[k, , drop = FALSE]
    code <- code[k]
  }
  rownames(out) <- NULL
  if (report) {
    n <- function(c) sum(code %in% c)
    line <- function(label, x, tag) {
      num <- format(x, big.mark = ",", scientific = FALSE)
      cat(label, formatC(num, width = 45 - nchar(label)), if (is.null(tag)) "" else if (generate) paste0("  (", gen, "==", tag, ")") else "  ", "\n", sep = "")
    }
    cat("\n    Result                      Number of obs\n    -----------------------------------------\n")
    line("    Not matched", n(1:2), NULL)
    if (n(1:2) > 0) {
      line("        from master", n(1), 1)
      line("        from using", n(2), 2)
    }
    cat("\n")
    if (update) {
      line("    Matched", n(3:5), NULL)
      line("        not updated", n(3), 3)
      line("        missing updated", n(4), 4)
      line("        nonmissing conflict", n(5), 5)
    } else {
      line("    Matched", n(3), 3)
    }
    cat("    -----------------------------------------\n")
  }
  if (generate) out[[gen]] <- haven::labelled(code, labs, label = "Matching result from merge")
  out
}"#;

/// `isid` as Stata 19 says it: the whole list named ("variables a, b, and
/// c"), missing values checked first (a string's is ""), both r(459).
pub(super) const ISID_HELPER: &str = r#"stata_isid <- function(d, vars, missok = FALSE) {
  for (v in vars) if (!v %in% names(d)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
  k <- length(vars)
  named <- if (k == 1) paste("variable", vars) else if (k == 2) paste("variables", vars[1], "and", vars[2]) else
    paste0("variables ", paste(vars[-k], collapse = ", "), ", and ", vars[k])
  if (!missok) {
    gone <- vapply(vars, function(v) { x <- d[[v]]; if (is.character(x)) any(is.na(x) | x == "") else any(is.na(x)) }, TRUE)
    if (any(gone)) stop(named, " should never be missing (Stata: r(459))", call. = FALSE)
  }
  if (anyDuplicated(as.data.frame(d)[vars]) > 0)
    stop(named, if (k == 1) " does not" else " do not", " uniquely identify the observations (Stata: r(459))", call. = FALSE)
  invisible(NULL)
}"#;

pub(super) const FORCED_HELPER: &str = r#"stata_forced <- function(d, u) {
  for (v in intersect(names(d), names(u))) {
    if (is.character(d[[v]]) != is.character(u[[v]])) {
      u[[v]] <- if (is.character(d[[v]])) rep("", nrow(u)) else rep(NA_real_, nrow(u))
    }
  }
  u
}"#;

/// A file read in has its `%td` dates as R dates, while dates the do-file
/// made are Stata's day numbers: the appended file's are put in the form the
/// data in memory have (days since 1 January 1960), as Stata keeps one form.
pub(super) const SAME_DATES_HELPER: &str = r#"stata_same_dates <- function(d, u) {
  for (v in intersect(names(d), names(u))) {
    x <- d[[v]]
    y <- u[[v]]
    if (inherits(y, "Date") && is.numeric(x) && !inherits(x, "Date")) {
      u[[v]] <- structure(as.numeric(y - as.Date("1960-01-01")), label = attr(y, "label", exact = TRUE), format.stata = attr(y, "format.stata", exact = TRUE))
    } else if (inherits(x, "Date") && is.numeric(y) && !inherits(y, "Date")) {
      u[[v]] <- structure(as.Date("1960-01-01") + y, label = attr(y, "label", exact = TRUE))
    }
  }
  u
}"#;

/// `destring`: Stata's rules for which text is a number (`""` and `.` are
/// missing; "1,000", "5%" and "NA" are not numbers); without `force` a
/// variable with any other text is left alone, with Stata's message.
pub(super) const DESTRING_HELPER: &str = "stata_destring <- function(x, name, ignore = \"\", force = FALSE, generate = FALSE) {\n  if (!is.character(x)) {\n    message(name, \" already numeric; no \", if (generate) \"generate\" else \"replace\")\n    return(if (generate) NULL else x)\n  }\n  y <- x\n  for (ch in strsplit(ignore, \"\")[[1]]) y <- gsub(ch, \"\", y, fixed = TRUE)\n  y <- trimws(y)\n  number <- grepl(\"^[+-]?([0-9]+[.]?[0-9]*|[.][0-9]+)([eEdD][+-]?[0-9]+)?$\", y)\n  missing <- is.na(y) | y == \"\" | grepl(\"^[.][a-z]?$\", y)\n  if (!force && !all(number | missing)) {\n    message(name, \": contains nonnumeric characters; no \", if (generate) \"generate\" else \"replace\")\n    return(if (generate) NULL else x)\n  }\n  out <- suppressWarnings(as.numeric(sub(\"[dD]\", \"e\", y)))\n  out[!number] <- NA\n  structure(out, label = attr(x, \"label\", exact = TRUE))\n}";

/// `tostring` of whole numbers, written as Stata's `%12.0g` writes them;
/// missing becomes ".". Other numbers stop with a message (not translated).
pub(super) const TOSTRING_HELPER: &str = "stata_tostring <- function(x, name, generate = FALSE) {\n  if (is.character(x)) {\n    message(name, \" already string; no \", if (generate) \"generate\" else \"replace\")\n    return(if (generate) NULL else x)\n  }\n  v <- as.numeric(x)\n  s <- ifelse(is.na(v), \".\", formatC(v, format = \"f\", digits = 0))\n  if (any(!is.na(v) & (v != trunc(v) | nchar(s) > 12))) stop(name, \": tostring of numbers that are not whole (or have more than 12 digits) is not translated yet\")\n  structure(s, label = attr(x, \"label\", exact = TRUE))\n}";

/// A missing value written out (`.`, `.a` to `.z`) by its place in Stata's
/// order: 1 for `.`, 2 to 27 for `.a` to `.z` (as `stata_mv` gives it).
fn missing_place(t: &str) -> Option<u32> {
    match t.as_bytes() {
        [b'.'] => Some(1),
        [b'.', c] if c.is_ascii_lowercase() => Some(u32::from(*c - b'a') + 2),
        _ => None,
    }
}

/// A `recode` rule's left side as an R condition on `v`. A missing value
/// written out matches itself only (Stata 19: `(. = 8)` leaves `.a` alone,
/// `(.a/.w = 5)` takes `.a` to `.w`); `mv` is set when that needs `stata_mv`
/// (where the data may hold `.a` to `.z`, `tags`).
fn rule_condition(
    values: &str,
    v: &str,
    tags: bool,
    mv: &mut bool,
) -> Result<String, Untranslated> {
    let mut conds = Vec::new();
    let mut numbers = Vec::new();
    let mut places = Vec::new();
    for tok in values.split_whitespace() {
        match tok {
            "else" | "*" => conds.push("TRUE".to_string()),
            "missing" | "miss" => conds.push(format!("is.na({v})")),
            "nonmissing" | "nonmiss" => conds.push(format!("!is.na({v})")),
            t if t.contains('/') => {
                let (a, b) = t.split_once('/').unwrap_or((t, t));
                // between two missing values: by their places
                if let (Some(pa), Some(pb)) = (missing_place(a), missing_place(b)) {
                    *mv = true;
                    conds.push(format!("(stata_mv({v}) >= {pa} & stata_mv({v}) <= {pb})"));
                    continue;
                }
                // from a missing value to a number (or max): none, as Stata
                // 19 makes no change (the missing values come after them all)
                if missing_place(a).is_some() {
                    if b != "max" && number_literal(b).filter(|n| n != "NA_real_").is_none() {
                        return Err(no(format!("recode: {b} is not a number")));
                    }
                    conds.push(format!("rep_len(FALSE, length({v}))"));
                    continue;
                }
                let end = |e: &str| -> Result<String, Untranslated> {
                    match e {
                        // min and max are over the variable's nonmissing values, all rows
                        "min" => Ok(format!("min({v}, na.rm = TRUE)")),
                        "max" => Ok(format!("max({v}, na.rm = TRUE)")),
                        _ => number_literal(e)
                            .filter(|n| n != "NA_real_")
                            .ok_or_else(|| no(format!("recode: {e} is not a number"))),
                    }
                };
                // from a number (or min, max) to a missing value: the numbers
                // from it up and the missing values up to that one, by
                // Stata's order (`(1/.b = 9)` takes 1, 2, . and .b)
                if let Some(pb) = missing_place(b) {
                    let numbers = format!("(!is.na({v}) & {v} >= {})", end(a)?);
                    let missing = if tags {
                        *mv = true;
                        format!("(stata_mv({v}) >= 1 & stata_mv({v}) <= {pb})")
                    } else {
                        format!("is.na({v})")
                    };
                    conds.push(format!("({numbers} | {missing})"));
                    continue;
                }
                // a range of numbers never matches a missing value
                conds.push(format!(
                    "(!is.na({v}) & {v} >= {} & {v} <= {})",
                    end(a)?,
                    end(b)?
                ));
            }
            "min" => conds.push(format!("({v} %in% min({v}, na.rm = TRUE))")),
            "max" => conds.push(format!("({v} %in% max({v}, na.rm = TRUE))")),
            t => match missing_place(t) {
                Some(p) => places.push(p),
                None => numbers.push(
                    number_literal(t).ok_or_else(|| no(format!("recode: {t} is not a number")))?,
                ),
            },
        }
    }
    if !numbers.is_empty() {
        conds.push(format!("{v} %in% c({})", numbers.join(", ")));
    }
    if !places.is_empty() {
        if tags || places.iter().any(|p| *p > 1) {
            *mv = true;
            let list: Vec<String> = places.iter().map(u32::to_string).collect();
            conds.push(format!("stata_mv({v}) %in% c({})", list.join(", ")));
        } else {
            conds.push(format!("is.na({v})"));
        }
    }
    match conds.len() {
        0 => Err(no("recode: a rule has no values")),
        1 => Ok(conds.join("")),
        _ => Ok(format!("({})", conds.join(" | "))),
    }
}

/// The pieces of `name(args)`.
fn call_parts(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    if close < open || !text.get(close + 1..).unwrap_or("").trim().is_empty() {
        return None;
    }
    Some((text.get(..open)?.trim(), text.get(open + 1..close)?.trim()))
}

impl Session {
    fn need_helper(&mut self, name: &'static str) {
        self.helpers.insert(name);
    }

    pub(super) fn recode(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("generate", 1),
            ("prefix", 3),
            ("copyrest", 4),
            ("label", 5),
        ]) {
            return Err(no(format!(
                "recode with the option {o} is not translated yet"
            )));
        }
        let main = c.parts.main.trim();
        let open = main
            .find('(')
            .ok_or_else(|| no("recode needs rules in parentheses"))?;
        let vars = names(main.get(..open).unwrap_or(""))?;
        let rules_text = main.get(open..).unwrap_or("");
        let mut rules = Vec::new();
        let mut labels: Vec<(f64, String)> = Vec::new();
        for raw in rules_text.split(')') {
            let r = raw.trim().trim_start_matches('(').trim();
            if r.is_empty() {
                continue;
            }
            let (left, right) = r
                .split_once('=')
                .ok_or_else(|| no(format!("recode: ({r}) needs `=`")))?;
            let right = right.trim();
            // `1 "18-29"`: the new value and its label
            let (number, text) = match right.split_once('"') {
                Some((n, t)) => (
                    n.trim(),
                    Some(t.trim_end().trim_end_matches('"').to_string()),
                ),
                None => (right, None),
            };
            let target = match missing_place(number) {
                // .a to .z stored as haven's tagged NAs, as Stata keeps them
                Some(p) if p > 1 => {
                    self.extended_missing.set(true);
                    self.tags.set(true);
                    out.package("haven");
                    format!(
                        "haven::tagged_na({})",
                        r_text(number.get(1..).unwrap_or(""))
                    )
                }
                _ => number_literal(number)
                    .ok_or_else(|| no(format!("recode: {number} is not a number")))?,
            };
            if let Some(t) = text {
                let v = number.parse::<f64>().map_err(|_| {
                    no(format!(
                        "recode: a label for {number}, which is not a value"
                    ))
                })?;
                labels.push((v, t));
            }
            rules.push((left.trim().to_string(), target));
        }
        let new_names: Vec<String> = match (opts.arg("generate", 1), opts.arg("prefix", 3)) {
            (Some(g), _) => names(&g)?,
            (None, Some(p)) => vars.iter().map(|v| format!("{p}{v}")).collect(),
            (None, None) => Vec::new(),
        };
        if !new_names.is_empty() && new_names.len() != vars.len() {
            return Err(no(
                "recode: generate() needs one new name for each variable",
            ));
        }
        // labels in the rules make a label set for each new variable, named
        // after it or label(); one already there gains them, the later text
        // of a value winning (as Stata 19 does)
        let mut sets = Vec::new();
        if !labels.is_empty() {
            if new_names.is_empty() {
                return Err(no(
                    "Rules defining value labels not allowed when overwriting a variable (Stata: r(198))",
                ));
            }
            let given = opts.arg("label", 5).map(|l| l.trim().to_string());
            if given.is_some() && new_names.len() > 1 {
                return Err(no(
                    "recode with label() and more than one variable is not translated yet",
                ));
            }
            for n in &new_names {
                let name = given.clone().unwrap_or_else(|| n.clone());
                if !is_name(&name) {
                    return Err(no(format!("{name} is not a label name")));
                }
                let set = self.label_sets.entry(name.clone()).or_default();
                for (v, t) in &labels {
                    match set.iter_mut().find(|(x, _)| x == v) {
                        Some(slot) => slot.1 = t.clone(),
                        None => set.push((*v, t.clone())),
                    }
                }
                set.sort_by(|a, b| a.0.total_cmp(&b.0));
                let body: Vec<String> = set
                    .iter()
                    .map(|(v, t)| format!("{} = {}", r_text(t), r_number(*v)))
                    .collect();
                out.line(format!(
                    "{} <- c({})",
                    super::label_object(&name),
                    body.join(", ")
                ));
                sets.push(super::label_object(&name));
            }
        }
        let cond = self.condition(c, out)?;
        let mut assigns = Vec::new();
        for (i, v) in vars.iter().enumerate() {
            let rv = r_name(v);
            let mut cases = Vec::new();
            for (left, target) in &rules {
                // the first rule that matches wins
                let mut mv = false;
                let when = rule_condition(left, &rv, self.tags.get(), &mut mv)?;
                if mv {
                    self.helpers.insert("stata_mv");
                    out.package("haven");
                }
                cases.push(format!("{when} ~ {target}"));
            }
            let recoded = format!("dplyr::case_when({}, .default = {rv})", cases.join(", "));
            match new_names.get(i) {
                Some(n) => {
                    // rows outside if/in get missing, unless copyrest
                    let outside = if opts.has("copyrest", 4) {
                        rv.clone()
                    } else {
                        "NA_real_".into()
                    };
                    let value = match &cond {
                        Some(k) => format!("dplyr::if_else({k}, {recoded}, {outside})"),
                        None => recoded,
                    };
                    assigns.push(match sets.get(i) {
                        Some(set) => {
                            out.package("haven");
                            format!(
                                "{} = haven::labelled(as.numeric({value}), labels = {set}, label = \"RECODE of {v}\")",
                                r_name(n)
                            )
                        }
                        None => format!(
                            "{} = structure(as.numeric({value}), label = \"RECODE of {v}\")",
                            r_name(n)
                        ),
                    });
                    self.kinds.insert(n.clone(), Kind::Number);
                }
                None => {
                    let value = match &cond {
                        Some(k) => format!("dplyr::if_else({k}, {recoded}, {rv})"),
                        None => recoded,
                    };
                    assigns.push(format!("{rv} = {value}"));
                }
            }
        }
        out.package("dplyr");
        self.step(c, format!("dplyr::mutate({})", assigns.join(", ")), out)
    }

    pub(super) fn egen(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let (lhs, rhs) = c
            .parts
            .main
            .split_once('=')
            .ok_or_else(|| no("egen needs `newvar = fcn(...)`"))?;
        let words: Vec<&str> = lhs.split_whitespace().collect();
        let (ty, var) = match words.as_slice() {
            [v] => (None, *v),
            [t, v] => (Some(*t), *v),
            _ => return Err(no("egen needs one new variable name")),
        };
        if !is_name(var) {
            return Err(no(format!("{var} is not a variable name")));
        }
        let (fname, args) =
            call_parts(rhs).ok_or_else(|| no("egen needs a function, like mean(x)"))?;
        let known: &[(&str, usize)] = &[
            ("by", 2),
            ("missing", 7),
            ("field", 5),
            ("track", 5),
            ("unique", 6),
            ("p", 1),
        ];
        if let Some(o) = opts.other(known) {
            return Err(no(format!(
                "egen {fname}() with the option {o} is not translated yet"
            )));
        }
        let cond = self.condition(c, out)?;
        let sel = cond.clone().unwrap_or_else(|| "TRUE".into());
        // the values a statistic is taken over: rows outside if/in are left out
        let over = |me: &Self, out: &mut Out| -> Result<String, Untranslated> {
            let e = me.expr(args, Use::Value, out)?;
            Ok(match &cond {
                Some(k) => format!("dplyr::if_else({k}, {e}, NA_real_)"),
                None => e,
            })
        };
        let mut byte = false;
        let value = match fname {
            "mean" | "sd" | "min" | "max" | "median" | "pctile" | "iqr" | "std" => {
                self.need_helper("stata_stats");
                let x = over(self, out)?;
                match fname {
                    "mean" => format!("stata_mean({x})"),
                    "sd" => format!("stata_sd({x})"),
                    "min" => format!("stata_min({x})"),
                    "max" => format!("stata_max({x})"),
                    "median" => format!("stata_pctile({x}, 50)"),
                    "pctile" => {
                        let p = opts.arg("p", 1).unwrap_or_else(|| "50".into());
                        let p: f64 = p
                            .trim()
                            .parse()
                            .map_err(|_| no(format!("egen pctile(): p({p}) is not a number")))?;
                        format!("stata_pctile({x}, {})", r_number(p))
                    }
                    "iqr" => format!("(\\(x) stata_pctile(x, 75) - stata_pctile(x, 25))({x})"),
                    _ => format!("(\\(x) (x - stata_mean(x)) / stata_sd(x))({x})"),
                }
            }
            "total" | "sum" => {
                let x = over(self, out)?;
                // missing counts as 0; with missing, an all-missing group is missing
                if opts.has("missing", 7) {
                    format!("(\\(x) if (all(is.na(x))) NA_real_ else sum(x, na.rm = TRUE))({x})")
                } else {
                    format!("sum({x}, na.rm = TRUE)")
                }
            }
            "count" => format!("sum(!is.na({}))", over(self, out)?),
            "rank" => {
                let x = over(self, out)?;
                if opts.has("field", 5) {
                    format!("(\\(x) vapply(x, \\(v) if (is.na(v)) NA_real_ else 1 + sum(x > v, na.rm = TRUE), 0))({x})")
                } else if opts.has("track", 5) {
                    format!("(\\(x) vapply(x, \\(v) if (is.na(v)) NA_real_ else 1 + sum(x < v, na.rm = TRUE), 0))({x})")
                } else if opts.has("unique", 6) {
                    format!("as.numeric(rank({x}, ties.method = \"first\", na.last = \"keep\"))")
                } else {
                    format!("rank({x}, ties.method = \"average\", na.last = \"keep\")")
                }
            }
            "group" | "tag" => {
                if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) || opts.has("by", 2) {
                    return Err(no(format!("egen {fname}() with by is not translated yet")));
                }
                let keys = select(args)?;
                let missing = opts.has("missing", 7);
                // "" is a missing value for text, as `.` is for numbers
                let k = if missing {
                    // each of . and .a to .z a value of its own, in that order
                    if self.mv_on(out) {
                        self.need_helper("stata_sort_keys");
                        format!("stata_sort_keys(dplyr::pick({keys}))")
                    } else {
                        format!("dplyr::pick({keys})")
                    }
                } else {
                    format!("dplyr::mutate(dplyr::pick({keys}), dplyr::across(dplyr::where(is.character), \\(v) dplyr::na_if(v, \"\")))")
                };
                if fname == "group" {
                    let incomplete = if missing { "rank" } else { "na" };
                    format!("(\\(k, s) {{ r <- rep(NA_real_, nrow(k)); r[s] <- vctrs::vec_rank(k[s, , drop = FALSE], ties = \"dense\", incomplete = \"{incomplete}\"); r }})({k}, {sel})")
                } else {
                    byte = true;
                    // 1 on the first row of each combination among the selected rows
                    let complete = if missing {
                        // every row counts (a lone TRUE would be NA past the first)
                        "rep(TRUE, nrow(k))"
                    } else {
                        "stats::complete.cases(k)"
                    };
                    format!("(\\(k, s) {{ s <- rep_len(s, nrow(k)); t <- rep(0, nrow(k)); t[s] <- as.numeric(!duplicated(k[s, , drop = FALSE]) & ({complete})[s]); t }})({k}, {sel})")
                }
            }
            "rowtotal" | "rowmean" | "rowmax" | "rowmin" | "rownonmiss" | "rowmiss"
            | "rowmedian" | "rowpctile" | "rowfirst" | "rowlast" => {
                let cols = format!("dplyr::pick({})", select(args)?);
                let tags = self.tags.get();
                if tags && fname == "rowmin" {
                    self.need_helper("stata_untag");
                }
                let v = match fname {
                    // a row with no number: the last variable's value, .a to
                    // .z kept (Stata 19); rowmin's is `.`
                    "rowmax" if tags => format!("(\\(m) {{ r <- do.call(pmax, c(as.list(m), na.rm = TRUE)); a <- is.na(r); r[a] <- m[[ncol(m)]][a]; r }})({cols})"),
                    "rowmin" if tags => format!("stata_untag(do.call(pmin, c(as.list({cols}), na.rm = TRUE)))"),
                    // the first (last) number in the row; a row with none has
                    // the last (first) variable's value, as Stata 19 gives it
                    "rowfirst" => format!("Reduce(\\(r, v) ifelse(is.na(v), r, v), rev(as.list({cols})))"),
                    "rowlast" => format!("Reduce(\\(r, v) ifelse(is.na(v), r, v), as.list({cols}))"),
                    "rowtotal" if opts.has("missing", 7) => {
                        format!("(\\(m) ifelse(rowSums(!is.na(m)) == 0, NA_real_, rowSums(m, na.rm = TRUE)))({cols})")
                    }
                    "rowtotal" => format!("rowSums({cols}, na.rm = TRUE)"),
                    "rowmean" => format!("(\\(m) ifelse(rowSums(!is.na(m)) == 0, NA_real_, rowMeans(m, na.rm = TRUE)))({cols})"),
                    "rowmax" => format!("do.call(pmax, c(as.list({cols}), na.rm = TRUE))"),
                    "rowmin" => format!("do.call(pmin, c(as.list({cols}), na.rm = TRUE))"),
                    "rownonmiss" => format!("as.numeric(rowSums(!is.na({cols})))"),
                    // Stata's percentile of each row's values (R's type 2); the median is p(50)
                    "rowmedian" | "rowpctile" => {
                        let p = if fname == "rowmedian" {
                            50.0
                        } else {
                            match opts.arg("p", 1) {
                                None => 50.0,
                                Some(p) => p.trim().parse::<f64>().map_err(|_| no(format!("p({p}) needs a number")))?,
                            }
                        };
                        format!(r"apply(as.matrix({cols}), 1, \(r) {{ r <- r[!is.na(r)]; if (length(r)) unname(stats::quantile(r, {}, type = 2)) else NA_real_ }})", r_number(p / 100.0))
                    }
                    _ => format!("as.numeric(rowSums(as.data.frame(lapply({cols}, \\(v) is.na(v) | v %in% \"\"))))"),
                };
                match &cond {
                    Some(k) => format!("dplyr::if_else({k}, {v}, NA_real_)"),
                    None => v,
                }
            }
            other => return Err(no(format!("egen {other}() is not translated yet"))),
        };
        // results are float unless a type is given (tag is a byte)
        let storage = match ty {
            Some(t) => storage(t).ok_or_else(|| no(format!("{t} is not a storage type")))?,
            None if byte => Storage::Byte,
            None => Storage::Float,
        };
        let mut value = if byte && ty.is_none() {
            value
        } else {
            stored(&value, storage, &mut self.helpers, out)
        };
        if !matches!(
            fname,
            "group"
                | "tag"
                | "rowtotal"
                | "rowmean"
                | "rowmax"
                | "rowmin"
                | "rownonmiss"
                | "rowmiss"
                | "rowmedian"
                | "rowpctile"
                | "rowfirst"
                | "rowlast"
        ) {
            if let Some(k) = &cond {
                value = format!("dplyr::if_else({k}, {value}, NA_real_)");
            }
        }
        out.package("dplyr");
        if matches!(fname, "group" | "tag") {
            out.package("vctrs");
        }
        let d = self.data()?;
        out.line(format!(
            "if ({} %in% names({d})) stop(\"variable {var} already defined\")",
            r_text(var)
        ));
        let step = format!("dplyr::mutate({} = {value})", r_name(var));
        match opts.arg("by", 2) {
            Some(b) if !c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) => {
                // by() groups without sorting and keeps the data's order
                let group = self.group_by(&select(&b)?, out);
                let ungroup = self.ungroup();
                out.line(format!("{d} <- {d} |> {group} |> {step} |> {ungroup}"));
            }
            Some(_) => return Err(no("egen with both a by prefix and by() is not translated")),
            None => self.step(c, step, out)?,
        }
        self.kinds.insert(var.to_string(), Kind::Number);
        Ok(())
    }

    pub(super) fn merge(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let known = [
            ("keep", 4),
            ("generate", 3),
            ("nogenerate", 5),
            ("keepusing", 5),
            ("nolabel", 5),
            ("noreport", 5),
            ("sorted", 6),
            ("update", 2),
            ("replace", 3),
            ("assert", 3),
        ];
        if let Some(o) = opts.other(&known) {
            return Err(no(format!(
                "merge with the option {o} is not translated yet"
            )));
        }
        let d = self.data()?;
        let file = c
            .using
            .clone()
            .ok_or_else(|| no("merge needs `using file`"))?;
        let path = file_name(&file, "dta");
        let (kind, keys) = first_word(&c.parts.main);
        if !matches!(kind, "1:1" | "m:1" | "1:m" | "m:m") {
            return Err(no(format!(
                "merge {kind}: the old syntax without 1:1, m:1, 1:m or m:m is not translated yet"
            )));
        }
        // 1:1 _n: the k-th row with the k-th row
        let keys = if keys.trim() == "_n" {
            if kind != "1:1" {
                return Err(no("merge _n is 1:1 only (Stata: r(198))"));
            }
            Vec::new()
        } else {
            names(keys)?
        };
        let key_vec = format!(
            "c({})",
            keys.iter()
                .map(|k| r_text(k))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let update = opts.has("update", 2);
        let replace = opts.has("replace", 3);
        if replace && !update {
            return Err(no("merge: replace needs update (Stata: r(198))"));
        }
        let codes = |text: &str| -> Result<String, Untranslated> {
            let mut out = Vec::new();
            for w in text.split_whitespace() {
                let w = w.to_ascii_lowercase();
                let at = |word: &str, min: usize| w.len() >= min && word.starts_with(w.as_str());
                out.push(if w == "1" || at("master", 3) {
                    "1"
                } else if w == "2" || at("using", 2) {
                    "2"
                } else if w == "4" || at("match_update", 7) {
                    "4"
                } else if w == "5" || at("match_conflict", 7) {
                    "5"
                } else if w == "3" || at("matched", 3) || w == "match" {
                    "3"
                } else {
                    return Err(no(format!("merge: {w} is not one of master, using, match, match_update or match_conflict")));
                });
            }
            Ok(format!("c({})", out.join(", ")))
        };
        let gen = opts.arg("generate", 3).unwrap_or_else(|| "_merge".into());
        let gen = gen.trim().to_string();
        let generate = !opts.has("nogenerate", 5);
        out.package("haven");
        out.package("dplyr");
        self.helpers.insert("stata_merge");
        // the file may hold .a to .z (tagged NAs)
        self.tags.set(true);
        out.line(format!("merge_using <- haven::read_dta({})", r_text(&path)));
        if let Some(k) = opts.arg("keepusing", 5) {
            out.line(format!(
                "merge_using <- dplyr::select(merge_using, dplyr::all_of({key_vec}), {})",
                select(&k)?
            ));
        }
        if opts.has("nolabel", 5) {
            out.line("merge_using <- haven::zap_labels(merge_using)");
        }
        if generate {
            out.line(format!(
                "if ({} %in% names({d})) stop(\"variable {gen} already defined\")",
                r_text(&gen)
            ));
        }
        let mut args = format!(
            "{d}, merge_using, {key_vec}, {}, gen = {}",
            r_text(kind),
            r_text(&gen)
        );
        if update {
            args.push_str(", update = TRUE");
        }
        if replace {
            args.push_str(", replace = TRUE");
        }
        if let Some(k) = opts.arg("keep", 4) {
            args.push_str(&format!(", keep = {}", codes(&k)?));
        }
        if let Some(a) = opts.arg("assert", 3) {
            args.push_str(&format!(
                ", assert = {}, target = {}",
                codes(&a)?,
                r_text(&d)
            ));
        }
        if opts.has("noreport", 5) || self.quiet(c) {
            args.push_str(", report = FALSE");
        }
        // . and .a to .z each a key of its own
        if self.mv_on(out) {
            args.push_str(", tags = TRUE");
        }
        if !generate {
            args.push_str(", generate = FALSE");
        }
        out.line(format!("{d} <- stata_merge({args})"));
        out.line("rm(merge_using)");
        if kind != "1:1" {
            out.caveat(
                "rows with the same key keep their order here; Stata's sort may put them in \
                 another order",
            );
        }
        Ok(())
    }

    pub(super) fn append(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("generate", 3),
            ("nolabel", 5),
            ("nonotes", 5),
            ("force", 5),
        ]) {
            return Err(no(format!(
                "append with the option {o} is not translated yet"
            )));
        }
        let d = self.data()?;
        let files = c
            .using
            .clone()
            .ok_or_else(|| no("append needs `using file`"))?;
        let force = opts.has("force", 5);
        self.need_helper("stata_same_dates");
        if force {
            self.need_helper("stata_forced");
        }
        // the files may hold .a to .z (tagged NAs)
        self.tags.set(true);
        let reads: Vec<String> = files
            .split_whitespace()
            .map(|f| {
                let read = format!(
                    "stata_same_dates({d}, haven::read_dta({}))",
                    r_text(&file_name(f, "dta"))
                );
                if force {
                    format!("stata_forced({d}, {read})")
                } else {
                    read
                }
            })
            .collect();
        out.package("haven");
        out.package("dplyr");
        match opts.arg("generate", 3) {
            Some(g) => {
                let g = g.trim();
                let n = reads.len();
                let labels: Vec<String> = std::iter::once("\"Master\" = 0".to_string())
                    .chain((1..=n).map(|k| format!("\"Appended dataset {k}\" = {k}")))
                    .collect();
                out.line(format!(
                    "{d} <- dplyr::bind_rows(list({d}, {}), .id = {}) |> dplyr::mutate({gn} = haven::labelled(as.numeric({gn}) - 1, c({}))) |> dplyr::relocate({gn}, .after = dplyr::all_of(utils::tail(names({d}), 1)))",
                    reads.join(", "),
                    r_text(g),
                    labels.join(", "),
                    gn = r_name(g)
                ));
                self.kinds.insert(g.to_string(), Kind::Number);
            }
            None => out.line(format!(
                "{d} <- dplyr::bind_rows({d}, {})",
                reads.join(", ")
            )),
        }
        if force {
            out.caveat("with force a variable that is a string in one file and a number in the other is missing in the appended rows, as in Stata (which notes each one)");
        } else {
            out.caveat("a string variable in one file and a number in the other stops with an error, as in Stata");
        }
        Ok(())
    }

    pub(super) fn reshape(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("i", 1), ("j", 1), ("string", 6)]) {
            return Err(no(format!(
                "reshape with the option {o} is not translated yet"
            )));
        }
        let d = self.data()?;
        let (way, stubs) = first_word(&c.parts.main);
        let stubs = names(stubs)
            .map_err(|_| no("reshape needs the stub names written out (no @ or patterns)"))?;
        let i = names(&opts.arg("i", 1).ok_or_else(|| no("reshape needs i()"))?)?;
        // reshape long without j() names the new variable _j (Stata 19)
        let j_text = match opts.arg("j", 1) {
            Some(t) => t,
            None if way == "long" => "_j".to_string(),
            None => return Err(no("reshape wide needs j()")),
        };
        let j = j_text.split_whitespace().next().unwrap_or("").to_string();
        if !is_name(&j) || j_text.split_whitespace().count() > 1 {
            return Err(no(
                "reshape j() with a list of values is not translated yet",
            ));
        }
        let string = opts.has("string", 6);
        let stub_re = stubs.join("|");
        let i_vec = format!(
            "c({})",
            i.iter().map(|k| r_text(k)).collect::<Vec<_>>().join(", ")
        );
        let i_sel = select(&i.join(" "))?;
        out.package("tidyr");
        out.package("dplyr");
        match way {
            "long" => {
                let suffix = if string { "(.+)" } else { "(-?[0-9]+)" };
                let re = format!("^({stub_re}){suffix}$");
                out.line(format!("if (anyDuplicated({d}[{i_vec}])) stop(\"variable {} does not uniquely identify the observations\")", i.join(" ")));
                let mut steps = vec![format!(
                    "tidyr::pivot_longer(cols = dplyr::matches({re}), names_to = c(\".value\", {j}), names_pattern = {re})",
                    re = r_text(&re),
                    j = r_text(&j)
                )];
                if !string {
                    steps.push(format!(
                        "dplyr::mutate({jn} = as.numeric({jn}))",
                        jn = r_name(&j)
                    ));
                }
                steps.push(format!(
                    "dplyr::arrange(dplyr::pick(c({i_sel}, {})))",
                    r_name(&j)
                ));
                steps.push(format!(
                    "dplyr::relocate(c({i_sel}, {}, {}))",
                    r_name(&j),
                    stubs
                        .iter()
                        .map(|s| r_name(s))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                out.line(format!("{d} <- {d} |> {}", steps.join(" |> ")));
                self.kinds
                    .insert(j.clone(), if string { Kind::Text } else { Kind::Number });
            }
            "wide" => {
                out.line(format!(
                    "if (anyDuplicated({d}[c({i_vec}, {})])) stop(\"values of variable {j} not unique within {}\")",
                    r_text(&j),
                    i.join(" ")
                ));
                let values = stubs
                    .iter()
                    .map(|s| r_name(s))
                    .collect::<Vec<_>>()
                    .join(", ");
                // every other variable must be the same within i; Stata names
                // each one that is not, a line apiece
                let stub_vec = stubs
                    .iter()
                    .map(|s| r_text(s))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.line(format!(
                    "local({{ other <- setdiff(names({d}), c({i_vec}, {jt}, {stub_vec})); bad <- other[vapply(other, function(v) any(tapply({d}[[v]], interaction({d}[{i_vec}], drop = TRUE), function(z) length(unique(z)) > 1)), TRUE)]; if (length(bad) > 0) stop(paste0(\"variable \", bad, \" not constant within {within}\", collapse = \"\\n\")) }})",
                    jt = r_text(&j),
                    within = i.join(" ")
                ));
                out.line(format!(
                    "{d} <- {d} |> tidyr::pivot_wider(names_from = {jn}, values_from = c({values}), names_glue = {glue}, names_vary = \"slowest\", names_sort = TRUE)",
                    jn = r_name(&j),
                    glue = r_text(&format!("{{.value}}{{{j}}}"))
                ));
                out.line(format!(
                    "{d} <- {d} |> dplyr::arrange(dplyr::pick({i_sel})) |> dplyr::relocate(c({i_sel}, dplyr::matches({})))",
                    r_text(&format!("^({stub_re}).+$"))
                ));
                self.kinds.remove(&j);
            }
            other => {
                return Err(no(format!(
                    "reshape {other} is not translated (long or wide)"
                )))
            }
        }
        Ok(())
    }

    pub(super) fn collapse(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if c.parts.weight.is_some() {
            return Err(no("collapse with weights is not translated yet"));
        }
        if let Some(o) = opts.other(&[("by", 2), ("cw", 2), ("fast", 4)]) {
            return Err(no(format!(
                "collapse with the option {o} is not translated yet"
            )));
        }
        let d = self.data()?;
        self.need_helper("stata_stats");
        let mut stat = "mean".to_string();
        let mut targets: Vec<(String, String)> = Vec::new(); // new name -> R expression
        let mut sources = Vec::new();
        let mut percents = Vec::new();
        let mut rest = c.parts.main.trim().to_string();
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix('(') {
                let end = r
                    .find(')')
                    .ok_or_else(|| no("collapse: a (stat) is not closed"))?;
                stat = r.get(..end).unwrap_or("").trim().to_string();
                rest = r.get(end + 1..).unwrap_or("").trim().to_string();
                continue;
            }
            let (tok, after) = first_word(&rest);
            let tok = tok.to_string();
            rest = after.to_string();
            // `new = var` may be written with spaces around `=`
            let (new, src) = if let Some((n, s)) = tok.split_once('=') {
                if s.is_empty() {
                    let (s2, after2) = first_word(&rest);
                    let s2 = s2.to_string();
                    rest = after2.to_string();
                    (n.to_string(), s2)
                } else {
                    (n.to_string(), s.to_string())
                }
            } else if rest.starts_with('=') {
                let (s2, after2) = first_word(rest.trim_start_matches('=').trim());
                let s2 = s2.to_string();
                rest = after2.to_string();
                (tok.clone(), s2)
            } else {
                (tok.clone(), tok.clone())
            };
            if !is_name(&new) || !is_name(&src) {
                return Err(no(
                    "collapse: only plain variable names are translated here",
                ));
            }
            let x = r_name(&src);
            let plain = |e: String| (stat.clone(), e);
            // a value itself, where .a to .z may be kept (Stata 19)
            let keeps = matches!(stat.as_str(), "max" | "min" | "firstnm" | "lastnm")
                && self.kinds.get(&src) != Some(&Kind::Text)
                && self.mv_on(out);
            if keeps {
                self.need_helper("stata_mv_stats");
            }
            let (label, expr) = match stat.as_str() {
                "max" if keeps => plain(format!("stata_mv_max({x})")),
                "min" if keeps => plain(format!("stata_mv_min({x})")),
                "firstnm" if keeps => plain(format!("stata_mv_firstnm({x})")),
                "lastnm" if keeps => plain(format!("stata_mv_lastnm({x})")),
                "mean" => plain(format!("stata_mean({x})")),
                // the label is written "(p 50) x"
                "median" => ("p 50".to_string(), format!("stata_pctile({x}, 50)")),
                "sd" => plain(format!("stata_sd({x})")),
                "semean" => plain(format!("stata_sd({x}) / sqrt(sum(!is.na({x})))")),
                "sum" | "rawsum" => plain(format!("sum({x}, na.rm = TRUE)")),
                "count" => plain(format!("as.numeric(sum(!is.na({x})))")),
                "max" => plain(format!("stata_max({x})")),
                "min" => plain(format!("stata_min({x})")),
                "iqr" => plain(format!("stata_pctile({x}, 75) - stata_pctile({x}, 25)")),
                "first" => plain(format!("dplyr::first({x})")),
                "last" => plain(format!("dplyr::last({x})")),
                "firstnm" => plain(format!("dplyr::first({x}, na_rm = TRUE)")),
                "lastnm" => plain(format!("dplyr::last({x}, na_rm = TRUE)")),
                "percent" => {
                    percents.push(src.clone());
                    plain(format!("100 * sum(!is.na({x})) / collapse_total_{src}"))
                }
                s if s.starts_with('p')
                    && s.get(1..)
                        .is_some_and(|n| n.parse::<u8>().is_ok_and(|n| (1..=99).contains(&n))) =>
                {
                    let n = s.get(1..).unwrap_or("50");
                    (format!("p {n}"), format!("stata_pctile({x}, {n})"))
                }
                other => return Err(no(format!("collapse ({other}) is not translated yet"))),
            };
            let expr = format!(
                "structure({expr}, label = {})",
                r_text(&format!("({label}) {src}"))
            );
            // a later target of the same name replaces the earlier one
            targets.retain(|(n, _)| n != &new);
            targets.push((new.clone(), expr));
            sources.push(src);
        }
        if targets.is_empty() {
            return Err(no("collapse needs variables"));
        }
        out.package("dplyr");
        for p in &percents {
            out.line(format!(
                "collapse_total_{p} <- sum(!is.na({d}${}))",
                r_name(p)
            ));
        }
        let mut steps = Vec::new();
        if opts.has("cw", 2) {
            let cols = sources
                .iter()
                .map(|s| r_name(s))
                .collect::<Vec<_>>()
                .join(", ");
            steps.push(format!(
                "dplyr::filter(dplyr::if_all(c({cols}), \\(v) !is.na(v)))"
            ));
        }
        let by = opts.arg("by", 2).map(|b| select(&b)).transpose()?;
        if let Some(b) = &by {
            steps.push(self.group_by(b, out));
        }
        // each under a name of its own first: summarise reads a column it has
        // just made, so (mean) y (sd) sy = y would take the sd of the mean
        let assigns: Vec<String> = targets
            .iter()
            .enumerate()
            .map(|(i, (_, e))| format!("stata_collapse_{} = {e}", i + 1))
            .collect();
        let renames: Vec<String> = targets
            .iter()
            .enumerate()
            .map(|(i, (n, _))| format!("{} = stata_collapse_{}", r_name(n), i + 1))
            .collect();
        steps.push(format!(
            "dplyr::summarise({}, .groups = \"drop\")",
            assigns.join(", ")
        ));
        steps.push(format!("dplyr::rename({})", renames.join(", ")));
        // each missing value of the by() variables was a group: in Stata's order
        if let Some(b) = by.as_ref().filter(|_| self.tags.get()) {
            out.package("vctrs");
            self.need_helper("stata_sort_keys");
            steps.push(format!(
                "dplyr::arrange(stata_sort_keys(dplyr::pick({b}))) |> dplyr::select(-stata_tag_k)"
            ));
        }
        steps.push("as.data.frame()".into());
        out.line(format!("{d} <- {d} |> {}", steps.join(" |> ")));
        for p in &percents {
            out.line(format!("rm(collapse_total_{p})"));
        }
        out.caveat(
            "Stata stores means and the like of float variables as floats; R keeps full precision",
        );
        self.kinds.clear();
        self.storage.clear();
        Ok(())
    }

    /// `xpose, clear`: observations become variables v1, v2, ... and
    /// variables observations; text variables are rows of missing values.
    /// The new variables are floats, doubles with `promote`; `varname` adds
    /// `_varname` with the old names (as Stata 19 does).
    pub(super) fn xpose(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("clear", 5), ("varname", 7), ("promote", 7)]) {
            return Err(no(format!(
                "xpose with the option {o} is not translated yet"
            )));
        }
        if !opts.has("clear", 5) {
            return Err(no("you must specify ', clear' (Stata: r(198))"));
        }
        if !c.parts.main.trim().is_empty() || c.parts.if_exp.is_some() || c.parts.in_range.is_some()
        {
            return Err(no("xpose takes only options"));
        }
        let d = self.data()?;
        self.need_helper("stata_xpose");
        self.need_helper("stata_float");
        out.line(format!(
            "{d} <- stata_xpose({d}, varname = {}, promote = {})",
            super::stats::r_logical(opts.has("varname", 7)),
            super::stats::r_logical(opts.has("promote", 7))
        ));
        self.kinds.clear();
        self.storage.clear();
        if opts.has("varname", 7) {
            self.kinds.insert("_varname".into(), Kind::Text);
        }
        self.label_sets.clear();
        Ok(())
    }

    pub(super) fn duplicates(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let d = self.data()?;
        let (sub, vars) = first_word(&c.parts.main);
        let sel = if vars.trim().is_empty() {
            "dplyr::everything()".to_string()
        } else {
            select(vars)?
        };
        out.package("dplyr");
        // each of . and .a to .z a value of its own (Stata 19)
        let (key, unkey) = if self.mv_on(out) {
            self.need_helper("stata_tag_key");
            (
                format!(", stata_tag_k = stata_tag_key(dplyr::pick({sel}))"),
                " |> dplyr::select(-stata_tag_k)",
            )
        } else {
            (String::new(), "")
        };
        match sub {
            "drop" => {
                if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
                    return Err(no("duplicates drop with if/in is not translated yet"));
                }
                if !vars.trim().is_empty() && !opts.has("force", 5) {
                    out.line("stop(\"force option required with duplicates drop varlist\")");
                    return Ok(());
                }
                // the first row of each group stays, in the data's order
                out.line(format!(
                    "{d} <- {d} |> dplyr::distinct(dplyr::pick({sel}){key}, .keep_all = TRUE){unkey}"
                ));
            }
            "tag" => {
                let g = opts
                    .arg("generate", 3)
                    .ok_or_else(|| no("duplicates tag needs generate()"))?;
                let g = g.trim().to_string();
                let cond = self.condition(c, out)?.unwrap_or_else(|| "TRUE".into());
                out.line(format!(
                    "{d} <- {d} |> dplyr::mutate(dup_rows = {cond}) |> dplyr::group_by(dplyr::pick({sel}), dup_rows{key}) |> dplyr::mutate({gn} = dplyr::if_else(dup_rows, as.numeric(dplyr::n() - 1), NA_real_)) |> dplyr::ungroup() |> dplyr::select(-dup_rows){unkey}",
                    gn = r_name(&g)
                ));
                self.kinds.insert(g, Kind::Number);
            }
            "report" => {
                if self.quiet(c) {
                    return Ok(());
                }
                out.line(format!(
                    "local({{ k <- dplyr::count({d}, dplyr::pick({sel}){key}, name = \"copies\"); r <- dplyr::count(k, copies, name = \"groups\"); r$observations <- r$copies * r$groups; r$surplus <- r$observations - r$groups; print(as.data.frame(r[c(\"copies\", \"observations\", \"surplus\")]), row.names = FALSE) }})"
                ));
                out.caveat("printed in R's layout, not Stata's table");
            }
            other => return Err(no(format!("duplicates {other} is not translated yet"))),
        }
        Ok(())
    }

    /// `isid varlist [, missok]`: nothing when the variables identify the
    /// rows; Stata 19's r(459) messages when one is missing or two rows
    /// share the values (`sort` is not translated yet).
    pub(super) fn isid(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("missok", 6)]) {
            return Err(no(format!(
                "isid with the option {o} is not translated yet"
            )));
        }
        if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
            return Err(no("isid with if or in is not translated yet"));
        }
        let vars: Vec<&str> = c.parts.main.split_whitespace().collect();
        if vars.is_empty() || vars.iter().any(|v| !is_name(v)) {
            return Err(no(
                "isid with names only is translated (patterns are not yet)",
            ));
        }
        let d = self.data()?;
        self.helpers.insert("stata_isid");
        out.line(format!(
            "stata_isid({d}, c({}), missok = {})",
            vars.iter()
                .map(|v| r_text(v))
                .collect::<Vec<_>>()
                .join(", "),
            if opts.has("missok", 6) {
                "TRUE"
            } else {
                "FALSE"
            }
        ));
        Ok(())
    }

    pub(super) fn encode(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("generate", 3)]) {
            return Err(no(format!(
                "encode with the option {o} is not translated yet"
            )));
        }
        let var = c.parts.main.trim();
        if !is_name(var) {
            return Err(no("encode takes one variable"));
        }
        let g = opts
            .arg("generate", 3)
            .ok_or_else(|| no("encode needs generate()"))?;
        let g = g.trim().to_string();
        let v = match self.condition(c, out)? {
            Some(k) => format!("dplyr::if_else({k}, {}, \"\")", r_name(var)),
            None => r_name(var),
        };
        let d = self.data()?;
        out.package("haven");
        out.package("dplyr");
        out.line(format!(
            "if ({} %in% names({d})) stop(\"variable {g} already defined\")",
            r_text(&g)
        ));
        // codes 1, 2, ... in byte order of the distinct nonempty strings; "" is missing
        out.line(format!(
            "{d} <- {d} |> dplyr::mutate({gn} = (\\(v, src) {{ lv <- sort(unique(v[!is.na(v) & v != \"\"]), method = \"radix\"); haven::labelled(as.numeric(match(v, lv)), labels = stats::setNames(as.numeric(seq_along(lv)), lv), label = attr(src, \"label\", exact = TRUE)) }})({v}, {src}))",
            gn = r_name(&g),
            src = r_name(var)
        ));
        // the label set encode makes is named after the new variable
        out.line(format!(
            "{} <- attr({d}${}, \"labels\")",
            label_object(&g),
            r_name(&g)
        ));
        self.kinds.insert(g, Kind::Number);
        Ok(())
    }

    /// `decode v, generate(s)`: each value's label; "" where the value has
    /// none, is missing or `if` and `in` leave the row out (Stata 19).
    pub(super) fn decode(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("generate", 3)]) {
            return Err(no(format!(
                "decode with the option {o} is not translated yet"
            )));
        }
        let var = c.parts.main.trim();
        if !is_name(var) {
            return Err(no("decode takes one variable"));
        }
        let g = opts
            .arg("generate", 3)
            .ok_or_else(|| no("decode needs generate()"))?;
        let g = g.trim().to_string();
        if !is_name(&g) {
            return Err(no(format!("{g} is not a variable name")));
        }
        let keep = self.condition(c, out)?.unwrap_or_else(|| "TRUE".into());
        let d = self.data()?;
        out.package("dplyr");
        out.line(format!(
            "if ({} %in% names({d})) stop(\"variable {g} already defined\")",
            r_text(&g)
        ));
        out.line(format!(
            "{d} <- {d} |> dplyr::mutate({gn} = (\\(v, keep) {{ if (is.character(v)) stop(\"not possible with string variable\"); l <- attr(v, \"labels\", exact = TRUE); if (is.null(l)) stop({unl}); s <- names(l)[match(as.numeric(v), as.numeric(l))]; s[is.na(s) | !keep] <- \"\"; s }})({src}, {keep}))",
            gn = r_name(&g),
            src = r_name(var),
            unl = r_text(&format!("{var} not labeled"))
        ));
        self.kinds.insert(g, Kind::Text);
        Ok(())
    }

    pub(super) fn destring(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("generate", 3),
            ("replace", 7),
            ("ignore", 6),
            ("force", 5),
            ("float", 5),
        ]) {
            return Err(no(format!(
                "destring with the option {o} is not translated yet"
            )));
        }
        let d = self.data()?;
        self.need_helper("stata_destring");
        let ignore = opts.arg("ignore", 6).unwrap_or_default();
        let force = if opts.has("force", 5) {
            "TRUE"
        } else {
            "FALSE"
        };
        let float = opts.has("float", 5);
        let wrap_float = |e: String, me: &mut Self| -> String {
            if float {
                me.helpers.insert("stata_float");
                format!("stata_float({e})")
            } else {
                e
            }
        };
        out.package("dplyr");
        // `destring(x), gen(y)`: Stata 19 takes the variables in parentheses too
        let main = c.parts.main.trim();
        let main = main
            .strip_prefix('(')
            .and_then(|m| m.strip_suffix(')'))
            .unwrap_or(main);
        let vars = if main.trim().is_empty() {
            "_all".to_string()
        } else {
            main.to_string()
        };
        if opts.has("replace", 7) {
            let call = format!(
                "stata_destring(v, dplyr::cur_column(), {}, {force})",
                r_text(&ignore)
            );
            out.line(format!(
                "{d} <- {d} |> dplyr::mutate(dplyr::across({}, \\(v) {}))",
                select(&vars)?,
                wrap_float(call, self)
            ));
            for n in vars.split_whitespace() {
                self.kinds.insert(n.to_string(), Kind::Number);
            }
        } else if let Some(g) = opts.arg("generate", 3) {
            let olds = names(&vars)?;
            let news = names(&g)?;
            if olds.len() != news.len() {
                return Err(no(
                    "destring: generate() needs one new name for each variable",
                ));
            }
            let mut assigns = Vec::new();
            for (o, n) in olds.iter().zip(news.iter()) {
                let call = format!(
                    "stata_destring({}, {}, {}, {force}, generate = TRUE)",
                    r_name(o),
                    r_text(o),
                    r_text(&ignore)
                );
                assigns.push(format!("{} = {}", r_name(n), wrap_float(call, self)));
                self.kinds.insert(n.clone(), Kind::Number);
            }
            out.line(format!(
                "{d} <- {d} |> dplyr::mutate({})",
                assigns.join(", ")
            ));
        } else {
            return Err(no(
                "destring needs generate() or replace (Stata refuses it otherwise)",
            ));
        }
        Ok(())
    }

    pub(super) fn tostring(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("generate", 3), ("replace", 7)]) {
            return Err(no(format!(
                "tostring with the option {o} is not translated yet"
            )));
        }
        let d = self.data()?;
        self.need_helper("stata_tostring");
        out.package("dplyr");
        if opts.has("replace", 7) {
            out.line(format!(
                "{d} <- {d} |> dplyr::mutate(dplyr::across({}, \\(v) stata_tostring(v, dplyr::cur_column())))",
                select(&c.parts.main)?
            ));
            for n in c.parts.main.split_whitespace() {
                self.kinds.insert(n.to_string(), Kind::Text);
            }
        } else if let Some(g) = opts.arg("generate", 3) {
            let olds = names(&c.parts.main)?;
            let news = names(&g)?;
            if olds.len() != news.len() {
                return Err(no(
                    "tostring: generate() needs one new name for each variable",
                ));
            }
            let assigns: Vec<String> = olds
                .iter()
                .zip(news.iter())
                .map(|(o, n)| {
                    format!(
                        "{} = stata_tostring({}, {}, generate = TRUE)",
                        r_name(n),
                        r_name(o),
                        r_text(o)
                    )
                })
                .collect();
            for n in &news {
                self.kinds.insert(n.clone(), Kind::Text);
            }
            out.line(format!(
                "{d} <- {d} |> dplyr::mutate({})",
                assigns.join(", ")
            ));
        } else {
            return Err(no(
                "tostring needs generate() or replace (Stata refuses it otherwise)",
            ));
        }
        Ok(())
    }
}

/// `xpose`'s new data: see [`Session::xpose`].
pub(super) const XPOSE_HELPER: &str = r#"stata_xpose <- function(d, varname = FALSE, promote = FALSE) {
  if ("_varname" %in% names(d)) stop("xpose of data with a variable _varname is not translated yet")
  n <- nrow(d)
  m <- matrix(NA_real_, ncol(d), n)
  for (j in seq_along(d)) {
    v <- d[[j]]
    if (!is.character(v)) m[j, ] <- as.numeric(v)
  }
  out <- as.data.frame(m)
  names(out) <- paste0("v", seq_len(n))
  if (!promote) out[] <- lapply(out, stata_float)
  if (varname) out[["_varname"]] <- names(d)
  out
}"#;

/// A label set for `label values v name` when the do-file does not define
/// `name`: one the run made (encode's), else the .dta file last used's, read
/// from its value-labels section (formats 117 to 119), as haven keeps the
/// labels each variable has but not the names of the sets, nor the sets no
/// variable has; none, so the values show as numbers as in Stata, when the
/// file has no such set.
pub(super) const DTA_LABEL_SET_HELPER: &str = r#"stata_label_set <- function(name, path) {
  # a set made as the do-file ran (by encode, say) comes first
  made <- paste0("label_", name)
  if (exists(made, envir = globalenv())) return(get(made, envir = globalenv()))
  if (is.null(path) || !file.exists(path)) return(NULL)
  key <- paste(normalizePath(path), file.mtime(path))
  if (!identical(attr(stata_dta_sets, "key"), key)) {
    sets <- tryCatch(stata_dta_read_sets(path), error = function(e) list())
    stata_dta_sets <<- structure(sets, key = key)
  }
  stata_dta_sets[[name]]
}
stata_dta_sets <- NULL
stata_dta_read_sets <- function(path) {
  out <- list()
  con <- file(path, "rb")
  on.exit(close(con))
  h <- readBin(con, "raw", 400)
  if (length(h) < 64 || rawToChar(h[1:11]) != "<stata_dta>") return(out)
  rel <- suppressWarnings(as.integer(rawToChar(h[29:31])))
  if (is.na(rel) || rel < 117) return(out)
  endian <- if (length(grepRaw("<byteorder>MSF", h, fixed = TRUE)) > 0) "big" else "little"
  at <- grepRaw("<map>", h, fixed = TRUE)
  if (length(at) == 0) return(out)
  # the map's 8-byte offsets; the 12th is the value labels', the 14th the end
  m <- readBin(h[(at + 5):(at + 5 + 14 * 8 - 1)], "integer", 28, 4, endian = endian)
  m <- ifelse(m < 0, m + 2^32, m)
  lo <- if (endian == "little") m[c(TRUE, FALSE)] else m[c(FALSE, TRUE)]
  hi <- if (endian == "little") m[c(FALSE, TRUE)] else m[c(TRUE, FALSE)]
  off <- lo + hi * 2^32
  seek(con, off[12])
  r <- readBin(con, "raw", off[14] - off[12])
  nlen <- if (rel == 117) 33 else 129
  enc <- if (rel == 117) "latin1" else "UTF-8"
  int <- function(at, n = 1) readBin(r[at:(at + 4 * n - 1)], "integer", n, 4, endian = endian)
  txt <- function(b) {
    b <- b[seq_len(match(as.raw(0), c(b, as.raw(0))) - 1)]
    s <- rawToChar(b)
    Encoding(s) <- enc
    s
  }
  p <- 15
  while (p + 4 <= length(r) && rawToChar(r[p:(p + 4)]) == "<lbl>") {
    len <- int(p + 5)
    name <- txt(r[(p + 9):(p + 8 + nlen)])
    t0 <- p + 9 + nlen + 3
    n <- int(t0)
    tl <- int(t0 + 4)
    o <- if (n > 0) int(t0 + 8, n) else integer(0)
    v <- if (n > 0) int(t0 + 8 + 4 * n, n) else integer(0)
    tx <- r[(t0 + 8 + 8 * n):(t0 + 7 + 8 * n + tl)]
    out[[name]] <- stats::setNames(as.numeric(v), vapply(o, function(k) txt(tx[(k + 1):length(tx)]), ""))
    p <- t0 + len + 6
  }
  out
}"#;

/// `xtile`: each value's category, 1 plus how many cut points lie below it
/// (a cut point repeated counts each time), as Stata 19 gives them; missing
/// where the value is missing or `if` and `in` leave the row out. The cut
/// points are Stata's percentiles of the rows kept (R's type 2; `altdef`,
/// (n + 1)p, is type 6), or every value of `cutpoints()`'s variable.
pub(super) const XTILE_HELPER: &str = r#"stata_xtile <- function(x, sel, nq = 2, cut = NULL, altdef = FALSE) {
  x <- as.numeric(x)
  use <- rep_len(sel, length(x)) & !is.na(x)
  cuts <- if (!is.null(cut)) {
    as.numeric(cut)[!is.na(cut)]
  } else if (any(use)) {
    unname(stats::quantile(x[use], seq_len(nq - 1) / nq, type = if (altdef) 6 else 2))
  } else {
    numeric(0)
  }
  out <- vapply(x, function(v) 1 + sum(cuts < v), 0)
  out[!use] <- NA
  out
}"#;

impl Session {
    /// `xtile newvar = exp [if] [in], nquantiles(#) | cutpoints(var) [altdef]`.
    pub(super) fn xtile(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        self.no_by(c, "xtile")?;
        if let Some(o) = opts.other(&[("nquantiles", 2), ("cutpoints", 1), ("altdef", 3)]) {
            return Err(no(format!(
                "xtile with the option {o} is not translated yet"
            )));
        }
        if c.parts.weight.is_some() {
            return Err(no("xtile with weights is not translated yet"));
        }
        let (lhs, rhs) = c
            .parts
            .main
            .split_once('=')
            .ok_or_else(|| no("xtile needs `newvar = exp`"))?;
        let var = lhs.trim();
        if !is_name(var) {
            return Err(no(format!("{var} is not a variable name")));
        }
        let cut = opts.arg("cutpoints", 1).map(|v| v.trim().to_string());
        let nq = match opts.arg("nquantiles", 2) {
            Some(n) => match n.trim().parse::<u32>() {
                Ok(k) if k >= 2 => k,
                _ => {
                    return Err(no(
                        "nquantiles() must be a whole number of 2 or more (Stata: r(198))",
                    ))
                }
            },
            None => 2,
        };
        if cut.is_some() && opts.arg("nquantiles", 2).is_some() {
            return Err(no(
                "xtile: nquantiles() and cutpoints() may not be combined",
            ));
        }
        if let Some(v) = &cut {
            if !is_name(v) {
                return Err(no(format!(
                    "cutpoints({v}): only a variable name is translated"
                )));
            }
        }
        let exp = rhs.trim();
        let (x, kind) = self.value(exp, out)?;
        if kind == Kind::Text {
            return Err(no("xtile of a string: Stata refuses this (type mismatch)"));
        }
        let sel = self.condition(c, out)?.unwrap_or_else(|| "TRUE".into());
        let d = self.data()?;
        self.helpers.insert("stata_xtile");
        out.package("dplyr");
        out.line(format!(
            "if ({} %in% names({d})) stop(\"variable {var} already defined\")",
            r_text(var)
        ));
        let how = match &cut {
            Some(v) => format!(", cut = {}", r_name(v)),
            None => format!(", nq = {nq}"),
        };
        let alt = if opts.has("altdef", 3) {
            ", altdef = TRUE"
        } else {
            ""
        };
        out.line(format!(
            "{d} <- dplyr::mutate({d}, {} = stata_xtile({x}, {sel}{how}{alt}))",
            r_name(var)
        ));
        let label = match &cut {
            Some(v) => format!("{exp} categorized by {v}"),
            None => format!("{nq} quantiles of {exp}"),
        };
        out.line(format!(
            "attr({d}${}, \"label\") <- {}",
            r_name(var),
            r_text(&label)
        ));
        self.kinds.insert(var.to_string(), Kind::Number);
        out.caveat("the new variable holds whole numbers, as Stata's byte does");
        Ok(())
    }
}

/// `insheet`, as Stata 19 reads a file: tab or comma (a tab in the first
/// line makes it tab), names from the first row when no cell of it is a
/// number (lowercase, with what is not a letter, digit or _ left out, the
/// header kept as the label where it differs), v1, v2, ... when every cell
/// is one; decimals as float unless double. What Stata's rules there are
/// not read off for (a first row of both, names it would have to make up,
/// an empty column) stops rather than guess.
pub(super) const INSHEET_HELPER: &str = r##"stata_insheet <- function(file, delim = NULL, names = NA, case = FALSE, double = FALSE, show = TRUE) {
  if (!file.exists(file)) stop("file ", file, " not found", call. = FALSE)
  lines <- sub("\r$", "", readLines(file, warn = FALSE))
  lines <- lines[nzchar(lines)]
  if (!length(lines)) stop("insheet of an empty file is not translated yet", call. = FALSE)
  if (is.null(delim)) delim <- if (grepl("\t", lines[1], fixed = TRUE)) "\t" else ","
  cells <- as.data.frame(readr::read_delim(I(paste0(lines, collapse = "\n")), delim = delim, col_names = FALSE,
    col_types = readr::cols(.default = "c"), na = character(), trim_ws = FALSE, skip_empty_rows = FALSE,
    show_col_types = FALSE))
  num <- function(v) grepl("^[-+]?([0-9]+[.]?[0-9]*|[.][0-9]+)([eE][-+]?[0-9]+)?$", v)
  first <- unlist(cells[1, ], use.names = FALSE)
  if (is.na(names)) {
    names <- if (all(nzchar(first) & !num(first))) TRUE else if (all(num(first))) FALSE else
      stop("insheet: a first row with both names and numbers is not translated yet", call. = FALSE)
  }
  if (names) {
    nm <- gsub("[^A-Za-z0-9_]", "", first)
    if (!case) nm <- tolower(nm)
    cells <- cells[-1, , drop = FALSE]
    if (any(!nzchar(nm) | grepl("^[0-9]", nm) | nchar(nm) > 32 | duplicated(nm)))
      stop("insheet: names Stata would make up for these headers are not translated yet", call. = FALSE)
  } else {
    nm <- paste0("v", seq_along(cells))
  }
  d <- list()
  for (k in seq_along(cells)) {
    v <- cells[[k]]
    if (!any(nzchar(v))) stop("insheet: an empty column is not translated yet", call. = FALSE)
    if (all(num(v) | !nzchar(v))) {
      x <- suppressWarnings(as.numeric(ifelse(nzchar(v), v, NA)))
      if (!double && any(!is.na(x) & x != trunc(x))) x <- stata_float(x)
      v <- x
    }
    if (names && first[k] != nm[k]) {
      if (case) stop("insheet, case with a header that is not a name is not translated yet", call. = FALSE)
      attr(v, "label") <- first[k]
    }
    d[[nm[k]]] <- v
  }
  d <- structure(d, names = nm, row.names = .set_row_names(nrow(cells)), class = "data.frame")
  if (show) cat("(", ncol(d), " vars, ", nrow(d), " obs)\n", sep = "")
  d
}"##;

impl Session {
    /// `insheet using file [, clear tab comma delimiter() names nonames case double]`.
    pub(super) fn insheet(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[
            ("clear", 5),
            ("tab", 3),
            ("comma", 5),
            ("delimiter", 5),
            ("names", 5),
            ("nonames", 7),
            ("case", 4),
            ("double", 6),
        ]) {
            return Err(no(format!(
                "insheet with the option {o} is not translated yet"
            )));
        }
        if !c.parts.main.trim().is_empty() {
            return Err(no(
                "insheet of chosen variables (varlist using) is not translated yet",
            ));
        }
        let file = c
            .using
            .clone()
            .ok_or_else(|| no("insheet needs using and a file"))?;
        if !opts.has("clear", 5) {
            if self.data.is_some() {
                return Err(no("you must start with an empty dataset (Stata: r(18))"));
            }
            if let Some(why) = &self.data_lost {
                return Err(no(format!(
                    "insheet without clear: whether data is in memory is not known here ({why})"
                )));
            }
        }
        let path = file_name(&file, "");
        let name = frame_name(&path);
        let mut args = vec![r_text(&path)];
        let delim = match (
            opts.has("tab", 3),
            opts.has("comma", 5),
            opts.arg("delimiter", 5),
        ) {
            (true, false, None) => Some("\t".to_string()),
            (false, true, None) => Some(",".to_string()),
            (false, false, Some(d)) => Some(d.trim().trim_matches('"').to_string()),
            (false, false, None) => None,
            _ => {
                return Err(no(
                    "insheet: tab, comma and delimiter() are one or the other",
                ))
            }
        };
        if let Some(d) = delim {
            if d.chars().count() != 1 {
                return Err(no(format!(
                    "insheet: delimiter(\"{d}\") is one character (Stata: r(198))"
                )));
            }
            args.push(format!("delim = {}", r_text(&d)));
        }
        match (opts.has("names", 5), opts.has("nonames", 7)) {
            (true, true) => return Err(no("insheet: names and nonames together")),
            (true, false) => args.push("names = TRUE".into()),
            (false, true) => args.push("names = FALSE".into()),
            (false, false) => {}
        }
        if opts.has("case", 4) {
            args.push("case = TRUE".into());
        }
        if opts.has("double", 6) {
            args.push("double = TRUE".into());
        }
        if self.quiet(c) {
            args.push("show = FALSE".into());
        }
        out.package("readr");
        self.helpers.insert("stata_float");
        self.helpers.insert("stata_insheet");
        out.line(format!("{name} <- stata_insheet({})", args.join(", ")));
        self.data = Some(name);
        self.data_lost = None;
        self.file = None;
        self.kinds.clear();
        self.storage.clear();
        self.fv_base.clear();
        self.forget_ts();
        self.label_sets.clear();
        Ok(())
    }
}

/// `split`, as Stata 19 cuts a string (`plan/stata/split.md`): without parse()
/// into words; at the strings parse() gives (" " too) after trimming the ends
/// (unless notrim), at the earliest one (the later listed on a tie), empty
/// pieces kept and none after a last delimiter; at most limit() pieces.
pub(super) const SPLIT_HELPER: &str = r##"stata_split <- function(d, var, parse = NULL, gen = var, limit = Inf, trim = TRUE, rows = NULL, show = TRUE) {
  x <- d[[var]]
  if (is.null(x)) stop("variable ", var, " not found (Stata: r(111))", call. = FALSE)
  if (!is.character(x)) stop("numeric variables not allowed in varlist; ", var, " is a numeric variable (Stata: r(109))", call. = FALSE)
  if (!is.null(rows)) x[is.na(rows) | !rows] <- ""
  cut <- function(s) {
    if (is.null(parse)) {
      s <- trimws(s, whitespace = " ")
      return(if (nzchar(s)) strsplit(s, " +")[[1]] else character(0))
    }
    if (trim) s <- trimws(s, whitespace = " ")
    out <- character(0)
    while (nzchar(s)) {
      pos <- vapply(parse, function(p) as.integer(regexpr(p, s, fixed = TRUE)), 1L)
      hit <- which(pos > 0)
      if (!length(hit)) {
        out <- c(out, s)
        break
      }
      first <- min(pos[hit])
      k <- max(hit[pos[hit] == first])
      out <- c(out, substr(s, 1, first - 1))
      s <- substring(s, first + nchar(parse[k]))
    }
    out
  }
  pieces <- lapply(x, function(s) utils::head(cut(s), limit))
  k <- max(c(0L, lengths(pieces)))
  nm <- paste0(gen, seq_len(k))
  if (any(nm %in% names(d))) stop("cannot generate new variables using stub ", gen, " (Stata: r(110))", call. = FALSE)
  for (j in seq_len(k)) d[[nm[j]]] <- vapply(pieces, function(p) if (length(p) >= j) p[j] else "", "")
  if (show) cat("variables created as string: \n", paste(nm, collapse = "  "), "\n", sep = "")
  list(d = d, r = list(k_new = k, varlist = paste0(nm, " ", collapse = "")))
}"##;

/// The strings in `parse()`: quoted ones as written, others word by word.
fn parse_strings(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('"') {
            let (s, tail) = after.split_once('"').unwrap_or((after, ""));
            out.push(s.to_string());
            rest = tail.trim_start();
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push(rest.get(..end).unwrap_or(rest).to_string());
            rest = rest.get(end..).unwrap_or("").trim_start();
        }
    }
    out
}

impl Session {
    /// `split strvar [if] [in] [, generate() parse() limit() notrim]`.
    pub(super) fn split(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        let d = self.data()?;
        if let Some(o) = opts.other(&[("generate", 3), ("parse", 1), ("limit", 1), ("notrim", 6)]) {
            return Err(no(format!(
                "split with the option {o} is not translated yet"
            )));
        }
        if c.prefixes.iter().any(|p| matches!(p, Prefix::By { .. })) {
            return Err(no("by with split is not translated yet"));
        }
        let vars = names(&c.parts.main)?;
        let [var] = vars.as_slice() else {
            return Err(no(
                "split takes one string variable (Stata: too many variables specified)",
            ));
        };
        // the text as written: parse(" ") is a space, which the argument trims
        let raw = opts
            .0
            .iter()
            .find(|o| o.is("parse", 1))
            .map(|o| o.raw.clone().or_else(|| o.arg.clone()).unwrap_or_default());
        // none: words (runs of spaces as one); parse(" ") cuts at each space
        let words = raw.is_none();
        let parse = match raw {
            Some(p) => parse_strings(&p),
            None => vec![" ".to_string()],
        };
        if parse.is_empty() || parse.iter().any(|p| p.is_empty()) {
            return Err(no("split, parse() needs the strings to cut at"));
        }
        if words && opts.has("notrim", 6) {
            return Err(no(
                "parsing on spaces incompatible with notrim (Stata: r(198))",
            ));
        }
        // strings that could overlap: which Stata takes there is not read off
        let shared = parse.iter().enumerate().any(|(i, a)| {
            parse
                .iter()
                .skip(i + 1)
                .any(|b| a.chars().any(|ch| b.contains(ch)))
        });
        if shared {
            return Err(no(
                "split on parse strings that share characters is not translated yet",
            ));
        }
        let gen = match opts.arg("generate", 3) {
            Some(g) if is_name(g.trim()) => g.trim().to_string(),
            Some(g) => return Err(no(format!("generate({g}) is not a valid stub"))),
            None => var.clone(),
        };
        let limit = match opts.arg("limit", 1) {
            Some(l) => match l.trim().parse::<u32>() {
                Ok(k) if k >= 1 => k.to_string(),
                _ => {
                    return Err(no(format!(
                        "limit({l}) is a whole number of 1 or more (Stata: r(198))"
                    )))
                }
            },
            None => "Inf".to_string(),
        };
        let rows = match self.condition(c, out)? {
            Some(cond) => {
                out.package("dplyr");
                format!(", rows = dplyr::transmute({d}, stata_v = {cond})$stata_v")
            }
            None => String::new(),
        };
        let list = parse
            .iter()
            .map(|p| r_text(p))
            .collect::<Vec<_>>()
            .join(", ");
        self.helpers.insert("stata_split");
        out.line(format!(
            "stata_v <- stata_split({d}, {}, parse = {}, gen = {}, limit = {limit}, trim = {}{rows}, show = {})",
            r_text(var),
            if words { "NULL".to_string() } else { format!("c({list})") },
            r_text(&gen),
            if opts.has("notrim", 6) { "FALSE" } else { "TRUE" },
            if self.quiet(c) { "FALSE" } else { "TRUE" },
        ));
        out.line(format!("{d} <- stata_v$d"));
        out.line("stata_r <- stata_v$r");
        self.r_new = Some((returned::R_SPLIT, "split".into()));
        Ok(())
    }
}

/// Stata's storage type words before a name in `input`.
fn input_type(w: &str) -> Option<&'static str> {
    match w {
        "byte" | "int" | "long" => Some("integer"),
        "float" => Some("float"),
        "double" => Some("double"),
        "strL" => Some("string"),
        s if s
            .strip_prefix("str")
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) =>
        {
            Some("string")
        }
        _ => None,
    }
}

/// The values on one `input` line, split as Stata splits words.
fn input_values(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line.trim();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('"') {
            let (s, tail) = after.split_once('"').unwrap_or((after, ""));
            out.push(format!("\"{s}\""));
            rest = tail.trim_start();
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push(rest.get(..end).unwrap_or(rest).to_string());
            rest = rest.get(end..).unwrap_or("").trim_start();
        }
    }
    out
}

impl Session {
    /// `input [type] var [type] var ...` and its data lines up to `end`, into
    /// empty data: floats unless a type says otherwise (StatCraft's notes on
    /// Stata 19, probe 229). What is not read off is declined.
    pub(super) fn input(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let lines = self
            .input_lines
            .take()
            .ok_or_else(|| no("input typed alone is not translated yet"))?;
        if self.data.is_some() || self.data_lost.is_some() {
            return Err(no(
                "input into data already in memory is not translated yet (into empty data it is)",
            ));
        }
        // the variables and their types; double(a b) types a group
        let mut vars: Vec<(String, &'static str)> = Vec::new();
        let header = c.parts.main.replace('(', " ( ").replace(')', " ) ");
        let mut words = header.split_whitespace().peekable();
        while let Some(w) = words.next() {
            if let Some(t) = input_type(w) {
                if words.peek() == Some(&"(") {
                    words.next();
                    for n in words.by_ref() {
                        if n == ")" {
                            break;
                        }
                        vars.push((n.to_string(), t));
                    }
                } else {
                    let n = words
                        .next()
                        .ok_or_else(|| no("input: a type without a name"))?;
                    vars.push((n.to_string(), t));
                }
            } else if w == "(" || w == ")" {
                return Err(no("input: this list of names is not translated yet"));
            } else {
                vars.push((w.to_string(), "float"));
            }
        }
        if vars.is_empty() {
            return Err(no(
                "input with no names (more rows for the data in memory) is not translated yet",
            ));
        }
        for (n, _) in &vars {
            if !is_name(n) || n.len() > 32 {
                return Err(no(format!(
                    "{n} is not a valid variable name (Stata: r(198))"
                )));
            }
        }
        let mut cols: Vec<Vec<String>> = vec![Vec::new(); vars.len()];
        for line in &lines {
            if line.contains('`') || line.contains('$') {
                return Err(no("input lines with macros in them are not translated yet"));
            }
            let vals = input_values(line);
            if vals.len() != vars.len() {
                return Err(no(format!(
                    "input: a line with {} values for {} variables is not translated yet",
                    vals.len(),
                    vars.len()
                )));
            }
            for ((v, (name, t)), col) in vals.iter().zip(&vars).zip(cols.iter_mut()) {
                let code = if *t == "string" {
                    r_text(v.trim_matches('"'))
                } else if v == "." {
                    "NA".to_string()
                } else if v.len() == 2
                    && v.starts_with('.')
                    && v.chars().nth(1).is_some_and(|ch| ch.is_ascii_lowercase())
                {
                    // kept as haven's tagged NA, as Stata keeps .a to .z
                    self.extended_missing.set(true);
                    self.tags.set(true);
                    out.package("haven");
                    format!("haven::tagged_na({})", r_text(v.get(1..).unwrap_or("")))
                } else {
                    let x: f64 = v.trim_matches('"').parse().map_err(|_| {
                        no(format!("'{v}' cannot be read as a number for {name} (Stata skips the line); not translated yet"))
                    })?;
                    if *t == "integer" && x.fract() != 0.0 {
                        return Err(no(format!(
                            "{v} into the integer variable {name} is not translated yet"
                        )));
                    }
                    r_number(x)
                };
                col.push(code);
            }
        }
        let mut parts = Vec::new();
        for ((name, t), col) in vars.iter().zip(&cols) {
            let v = format!("c({})", col.join(", "));
            let v = match *t {
                "float" => {
                    self.helpers.insert("stata_float");
                    format!("stata_float(as.numeric({v}))")
                }
                "string" => format!("as.character({v})"),
                _ => format!("as.numeric({v})"),
            };
            parts.push(format!("{} = {v}", r_name(name)));
        }
        let n = lines.len();
        let code = if n == 0 {
            format!(
                "data.frame({}, stringsAsFactors = FALSE)[0, , drop = FALSE]",
                parts.join(", ")
            )
        } else {
            format!(
                "data.frame({}, stringsAsFactors = FALSE, check.names = FALSE)",
                parts.join(", ")
            )
        };
        out.line(format!("data <- {code}"));
        self.data = Some("data".into());
        self.data_lost = None;
        self.file = None;
        self.kinds.clear();
        self.storage.clear();
        self.fv_base.clear();
        self.forget_ts();
        self.label_sets.clear();
        for (name, t) in &vars {
            self.kinds.insert(
                name.clone(),
                if *t == "string" {
                    Kind::Text
                } else {
                    Kind::Number
                },
            );
            match *t {
                "float" => {
                    self.storage.insert(name.clone(), Storage::Float);
                }
                "double" => {
                    self.storage.insert(name.clone(), Storage::Double);
                }
                _ => {}
            }
            self.lost_vars.remove(name);
        }
        out.caveat("the data typed in, as Stata stores it (float unless a type says otherwise); Stata's echo of the lines is not printed");
        Ok(())
    }
}

impl Session {
    /// `confirm`: `[numeric|string] variable`, `new variable`, `[integer]
    /// number`, `file` and `new file`, each stopping with Stata's message and
    /// return code when it fails (StatCraft's notes on Stata 19, probe 120),
    /// so `capture confirm ...` leaves Stata's `_rc`.
    pub(super) fn confirm(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let text = super::returned::after_name(c);
        let words: Vec<String> = input_values(text.trim());
        let w: Vec<&str> = words.iter().map(String::as_str).collect();
        let stop = |msg: String, rc: u32| {
            format!(
                "stop({}, call. = FALSE)",
                r_text(&format!("{msg} (Stata: r({rc}))"))
            )
        };
        let is_var = |s: &str| s.len() >= 3 && "variable".starts_with(s);
        // [numeric|string] variable varlist
        let typed = matches!(w.first().copied(), Some("numeric" | "string"));
        let var_at = usize::from(typed);
        if w.get(var_at).is_some_and(|s| is_var(s)) {
            let kind = if typed { w.first().copied() } else { None };
            let names = w.get(var_at + 1..).unwrap_or(&[]);
            {
                if names.is_empty() {
                    return Err(no("confirm variable needs the variables (Stata: r(100))"));
                }
                if names.iter().any(|n| !is_name(n)) {
                    return Err(no(
                        "confirm variable with patterns or ranges is not translated yet",
                    ));
                }
                let d = self.data()?;
                let list = names
                    .iter()
                    .map(|n| r_text(n))
                    .collect::<Vec<_>>()
                    .join(", ");
                let kind_r = match kind {
                    Some(k) => r_text(k),
                    None => "NULL".into(),
                };
                self.helpers.insert("stata_confirm");
                out.line(format!("stata_confirm_vars({d}, c({list}), {kind_r})"));
                return Ok(());
            }
        }
        match w.as_slice() {
            ["new", v, names @ ..] if is_var(v) => {
                if names.is_empty() {
                    return Err(no("confirm new variable needs the names (Stata: r(100))"));
                }
                for n in names {
                    if !is_name(n) || n.len() > 32 {
                        out.line(stop(format!("{n} invalid name"), 198));
                        return Ok(());
                    }
                }
                let d = self.data()?;
                let list = names.iter().map(|n| r_text(n)).collect::<Vec<_>>().join(", ");
                self.helpers.insert("stata_confirm");
                out.line(format!("stata_confirm_new({d}, c({list}))"));
                Ok(())
            }
            [rest @ .., "number", s] if rest.is_empty() || rest == ["integer"] => {
                let integer = rest == ["integer"];
                // the text is written out (macros replaced): decided here, stopped in R
                let x = s.trim_matches('"').parse::<f64>().ok().filter(|v| v.is_finite());
                match x {
                    None => out.line(stop(format!("'{s}' found where number expected"), 7)),
                    Some(v) if integer && v.fract() != 0.0 => {
                        out.line(stop(format!("'{s}' found where integer expected"), 7))
                    }
                    Some(_) => out.line("invisible(NULL)"),
                }
                Ok(())
            }
            ["file", f] | ["new", "file", f] if self.files_unknown.is_some() => Err(no(format!(
                "confirm file: which files are there is not known here: a line before it that may write one was not translated ({})",
                self.files_unknown.clone().unwrap_or_default()
            ))),
            ["file", f] => {
                let f = &unquote_compound(f);
                let path = super::file_name(f, "");
                out.line(format!(
                    "if (!file.exists({})) {}",
                    r_text(path.trim_end_matches('/')),
                    stop(format!("file {} not found", f.trim_matches('"')), 601)
                ));
                Ok(())
            }
            ["new", "file", f] => {
                let f = &unquote_compound(f);
                let path = super::file_name(f, "");
                out.line(format!(
                    "if (file.exists({})) {}",
                    r_text(&path),
                    stop(format!("file {} already exists", f.trim_matches('"')), 602)
                ));
                Ok(())
            }
            _ => Err(no(format!("confirm {} is not translated yet", w.first().unwrap_or(&"")))),
        }
    }
}

/// `confirm`'s R.
pub(super) const CONFIRM_HELPER: &str = r##"stata_confirm_vars <- function(d, vars, kind = NULL) {
  have <- if (is.null(d)) character(0) else names(d)
  for (v in vars) {
    # an abbreviation names the one variable it starts
    hit <- if (v %in% have) v else { m <- have[startsWith(have, v)]; if (length(m) == 1) m else NA }
    if (is.na(hit)) stop("variable ", v, " not found (Stata: r(111))", call. = FALSE)
    x <- d[[hit]]
    if (identical(kind, "numeric") && !is.numeric(x)) stop("'", v, "' found where numeric variable expected (Stata: r(7))", call. = FALSE)
    if (identical(kind, "string") && !is.character(x)) stop("'", v, "' found where string variable expected (Stata: r(7))", call. = FALSE)
  }
  invisible(NULL)
}
stata_confirm_new <- function(d, vars) {
  have <- if (is.null(d)) character(0) else names(d)
  for (v in vars) if (v %in% have) stop("variable ", v, " already defined (Stata: r(110))", call. = FALSE)
  invisible(NULL)
}"##;

/// A file name in compound quotes (`` `"a b.txt"' ``) without them.
fn unquote_compound(s: &str) -> String {
    s.strip_prefix("`\"")
        .and_then(|r| r.strip_suffix("\"'"))
        .map_or_else(|| s.to_string(), str::to_string)
}

impl Session {
    /// `format varlist %fmt` (or `format %fmt varlist`): how the variables are
    /// shown, kept as their `format.stata` attribute (which haven writes to a
    /// .dta file); the values do not change.
    pub(super) fn format(&mut self, c: &Command, out: &mut Out) -> Result<(), Untranslated> {
        let words: Vec<&str> = c.parts.main.split_whitespace().collect();
        let (fmts, vars): (Vec<&str>, Vec<&str>) = words.iter().partition(|w| w.starts_with('%'));
        let [fmt] = fmts.as_slice() else {
            return Err(no(
                "format with no %format, or more than one, is not translated yet",
            ));
        };
        if vars.is_empty() {
            return Err(no("format needs the variables (Stata: varlist required)"));
        }
        if !super::returned::valid_format(fmt) || *fmt == "%21x" {
            return Err(no(format!(
                "format {fmt} is not a format RCraft has yet (Stata: invalid %format)"
            )));
        }
        if vars.iter().any(|v| !is_name(v)) {
            return Err(no(
                "format with patterns or ranges of variables is not translated yet",
            ));
        }
        let d = self.data()?;
        for v in &vars {
            out.line(format!(
                "if (!{0} %in% names({d})) stop(\"variable {1} not found (Stata: r(111))\", call. = FALSE)",
                r_text(v),
                v
            ));
            out.line(format!(
                "attr({d}[[{}]], \"format.stata\") <- {}",
                r_text(v),
                r_text(fmt)
            ));
        }
        out.caveat("the display format is kept with the variable (and in a saved .dta); R's own printing does not use it");
        Ok(())
    }
}
