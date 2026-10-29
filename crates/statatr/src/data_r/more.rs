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

/// `destring`: Stata's rules for which text is a number (`""` and `.` are
/// missing; "1,000", "5%" and "NA" are not numbers); without `force` a
/// variable with any other text is left alone, with Stata's message.
pub(super) const DESTRING_HELPER: &str = "stata_destring <- function(x, name, ignore = \"\", force = FALSE, generate = FALSE) {\n  if (!is.character(x)) {\n    message(name, \" already numeric; no \", if (generate) \"generate\" else \"replace\")\n    return(if (generate) NULL else x)\n  }\n  y <- x\n  for (ch in strsplit(ignore, \"\")[[1]]) y <- gsub(ch, \"\", y, fixed = TRUE)\n  y <- trimws(y)\n  number <- grepl(\"^[+-]?([0-9]+[.]?[0-9]*|[.][0-9]+)([eEdD][+-]?[0-9]+)?$\", y)\n  missing <- is.na(y) | y == \"\" | grepl(\"^[.][a-z]?$\", y)\n  if (!force && !all(number | missing)) {\n    message(name, \": contains nonnumeric characters; no \", if (generate) \"generate\" else \"replace\")\n    return(if (generate) NULL else x)\n  }\n  out <- suppressWarnings(as.numeric(sub(\"[dD]\", \"e\", y)))\n  out[!number] <- NA\n  structure(out, label = attr(x, \"label\"))\n}";

/// `tostring` of whole numbers, written as Stata's `%12.0g` writes them;
/// missing becomes ".". Other numbers stop with a message (not translated).
pub(super) const TOSTRING_HELPER: &str = "stata_tostring <- function(x, name, generate = FALSE) {\n  if (is.character(x)) {\n    message(name, \" already string; no \", if (generate) \"generate\" else \"replace\")\n    return(if (generate) NULL else x)\n  }\n  v <- as.numeric(x)\n  s <- ifelse(is.na(v), \".\", formatC(v, format = \"f\", digits = 0))\n  if (any(!is.na(v) & (v != trunc(v) | nchar(s) > 12))) stop(name, \": tostring of numbers that are not whole (or have more than 12 digits) is not translated yet\")\n  structure(s, label = attr(x, \"label\"))\n}";

/// A `recode` rule's left side as an R condition on `v`.
fn rule_condition(values: &str, v: &str) -> Result<String, Untranslated> {
    let mut conds = Vec::new();
    let mut numbers = Vec::new();
    for tok in values.split_whitespace() {
        match tok {
            "else" | "*" => conds.push("TRUE".to_string()),
            "missing" | "miss" => conds.push(format!("is.na({v})")),
            "nonmissing" | "nonmiss" => conds.push(format!("!is.na({v})")),
            t if t.contains('/') => {
                let (a, b) = t.split_once('/').unwrap_or((t, t));
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
                // a range never matches a missing value
                conds.push(format!(
                    "(!is.na({v}) & {v} >= {} & {v} <= {})",
                    end(a)?,
                    end(b)?
                ));
            }
            "min" => conds.push(format!("({v} %in% min({v}, na.rm = TRUE))")),
            "max" => conds.push(format!("({v} %in% max({v}, na.rm = TRUE))")),
            t => numbers
                .push(number_literal(t).ok_or_else(|| no(format!("recode: {t} is not a number")))?),
        }
    }
    if !numbers.is_empty() {
        conds.push(format!("{v} %in% c({})", numbers.join(", ")));
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
        if let Some(o) = opts.other(&[("generate", 1), ("prefix", 3), ("copyrest", 4)]) {
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
        for raw in rules_text.split(')') {
            let r = raw.trim().trim_start_matches('(').trim();
            if r.is_empty() {
                continue;
            }
            let (left, right) = r
                .split_once('=')
                .ok_or_else(|| no(format!("recode: ({r}) needs `=`")))?;
            let right = right.trim();
            if right.contains('"') {
                return Err(no(
                    "recode with value labels in its rules is not translated yet",
                ));
            }
            let target = number_literal(right)
                .ok_or_else(|| no(format!("recode: {right} is not a number")))?;
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
        let cond = self.condition(c, out)?;
        let mut assigns = Vec::new();
        for (i, v) in vars.iter().enumerate() {
            let rv = r_name(v);
            let mut cases = Vec::new();
            for (left, target) in &rules {
                // the first rule that matches wins
                cases.push(format!("{} ~ {target}", rule_condition(left, &rv)?));
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
                    assigns.push(format!(
                        "{} = structure(as.numeric({value}), label = \"RECODE of {v}\")",
                        r_name(n)
                    ));
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
                    format!("dplyr::pick({keys})")
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
                        "TRUE"
                    } else {
                        "stats::complete.cases(k)"
                    };
                    format!("(\\(k, s) {{ s <- rep_len(s, nrow(k)); t <- rep(0, nrow(k)); t[s] <- as.numeric(!duplicated(k[s, , drop = FALSE]) & ({complete})[s]); t }})({k}, {sel})")
                }
            }
            "rowtotal" | "rowmean" | "rowmax" | "rowmin" | "rownonmiss" | "rowmiss" => {
                let cols = format!("dplyr::pick({})", select(args)?);
                let v = match fname {
                    "rowtotal" if opts.has("missing", 7) => {
                        format!("(\\(m) ifelse(rowSums(!is.na(m)) == 0, NA_real_, rowSums(m, na.rm = TRUE)))({cols})")
                    }
                    "rowtotal" => format!("rowSums({cols}, na.rm = TRUE)"),
                    "rowmean" => format!("(\\(m) ifelse(rowSums(!is.na(m)) == 0, NA_real_, rowMeans(m, na.rm = TRUE)))({cols})"),
                    "rowmax" => format!("do.call(pmax, c(as.list({cols}), na.rm = TRUE))"),
                    "rowmin" => format!("do.call(pmin, c(as.list({cols}), na.rm = TRUE))"),
                    "rownonmiss" => format!("as.numeric(rowSums(!is.na({cols})))"),
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
                out.line(format!(
                    "{d} <- {d} |> dplyr::group_by(dplyr::pick({})) |> {step} |> dplyr::ungroup()",
                    select(&b)?
                ));
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
        let keys = names(keys)?;
        let (master_unique, using_unique) = match kind {
            "1:1" => (true, true),
            "m:1" => (false, true),
            "1:m" => (true, false),
            "m:m" => {
                return Err(no(
                    "merge m:m pairs the k-th rows of each key in Stata (not every combination); not translated",
                ))
            }
            other => return Err(no(format!("merge {other} is not translated (expected 1:1, m:1 or 1:m)"))),
        };
        let key_vec = format!(
            "c({})",
            keys.iter()
                .map(|k| r_text(k))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let key_list = keys.join(" ");
        let gen = opts.arg("generate", 3).unwrap_or_else(|| "_merge".into());
        out.package("haven");
        out.package("dplyr");
        out.line(format!("merge_using <- haven::read_dta({})", r_text(&path)));
        if let Some(k) = opts.arg("keepusing", 5) {
            out.line(format!(
                "merge_using <- dplyr::select(merge_using, dplyr::all_of({key_vec}), {})",
                select(&k)?
            ));
        }
        if master_unique {
            out.line(format!("if (anyDuplicated({d}[{key_vec}])) stop(\"variable {key_list} does not uniquely identify observations in the master data\")"));
        }
        if using_unique {
            out.line(format!("if (anyDuplicated(merge_using[{key_vec}])) stop(\"variable {key_list} does not uniquely identify observations in the using data\")"));
        }
        out.line(format!(
            "if ({} %in% names({d})) stop(\"variable {gen} already defined\")",
            r_text(&gen)
        ));
        out.line(format!("{d} <- {d} |> dplyr::mutate(merge_in_master = TRUE) |> dplyr::full_join(merge_using |> dplyr::mutate(merge_in_using = TRUE), by = {key_vec}, suffix = c(\"\", \".merge_using\"), na_matches = \"na\")"));
        // for a variable in both files the master's value stays, except in
        // rows only the using file has
        out.line(format!(
            "for (v in setdiff(intersect(names(merge_using), names({d})), {key_vec})) {{ u <- paste0(v, \".merge_using\"); if (u %in% names({d})) {{ {d}[[v]] <- dplyr::if_else(is.na({d}$merge_in_master), {d}[[u]], {d}[[v]]); {d}[[u]] <- NULL }} }}"
        ));
        let mut steps = vec![
            format!("dplyr::mutate({} = haven::labelled(dplyr::case_when(!is.na(merge_in_master) & !is.na(merge_in_using) ~ 3, !is.na(merge_in_master) ~ 1, .default = 2), c(\"Master only (1)\" = 1, \"Using only (2)\" = 2, \"Matched (3)\" = 3), label = \"Matching result from merge\"))", r_name(&gen)),
            // master rows sorted by key (stable), then the using-only rows sorted by key
            format!("dplyr::arrange({} == 2, dplyr::pick({}))", r_name(&gen), select(&key_list)?),
            "dplyr::select(-merge_in_master, -merge_in_using)".to_string(),
        ];
        if let Some(k) = opts.arg("keep", 4) {
            let mut codes = Vec::new();
            for w in k.split_whitespace() {
                codes.push(match w {
                    "master" | "1" => "1",
                    "using" | "2" => "2",
                    "match" | "matched" | "3" => "3",
                    other => return Err(no(format!("merge keep({other}) is not translated"))),
                });
            }
            steps.push(format!(
                "dplyr::filter({} %in% c({}))",
                r_name(&gen),
                codes.join(", ")
            ));
        }
        if opts.has("nogenerate", 5) {
            steps.push(format!("dplyr::select(-{})", r_name(&gen)));
        }
        out.line(format!("{d} <- {d} |> {}", steps.join(" |> ")));
        out.line("rm(merge_using)");
        Ok(())
    }

    pub(super) fn append(
        &mut self,
        c: &Command,
        opts: &Opts,
        out: &mut Out,
    ) -> Result<(), Untranslated> {
        if let Some(o) = opts.other(&[("generate", 3), ("nolabel", 5), ("nonotes", 5)]) {
            return Err(no(format!(
                "append with the option {o} is not translated yet"
            )));
        }
        let d = self.data()?;
        let files = c
            .using
            .clone()
            .ok_or_else(|| no("append needs `using file`"))?;
        let reads: Vec<String> = files
            .split_whitespace()
            .map(|f| format!("haven::read_dta({})", r_text(&file_name(f, "dta"))))
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
        out.caveat("a string variable in one file and a number in the other stops with an error, as in Stata");
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
        let j_text = opts.arg("j", 1).ok_or_else(|| no("reshape needs j()"))?;
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
                out.line(format!(
                    "{d} <- {d} |> tidyr::pivot_wider(names_from = {jn}, values_from = c({values}), names_glue = {glue}, names_vary = \"slowest\", names_sort = TRUE)",
                    jn = r_name(&j),
                    glue = r_text(&format!("{{.value}}{{{j}}}"))
                ));
                // every other variable must be the same within i
                out.line(format!("if (anyDuplicated({d}[{i_vec}])) stop(\"a variable other than the stubs is not constant within {}\")", i.join(" ")));
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
            let (label, expr) = match stat.as_str() {
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
        if let Some(b) = opts.arg("by", 2) {
            steps.push(format!("dplyr::group_by(dplyr::pick({}))", select(&b)?));
        }
        let assigns: Vec<String> = targets
            .iter()
            .map(|(n, e)| format!("{} = {e}", r_name(n)))
            .collect();
        steps.push(format!(
            "dplyr::summarise({}, .groups = \"drop\")",
            assigns.join(", ")
        ));
        steps.push("as.data.frame()".into());
        out.line(format!("{d} <- {d} |> {}", steps.join(" |> ")));
        for p in &percents {
            out.line(format!("rm(collapse_total_{p})"));
        }
        out.caveat(
            "Stata stores means and the like of float variables as floats; R keeps full precision",
        );
        self.kinds.clear();
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
        match sub {
            "drop" => {
                if c.parts.if_exp.is_some() || c.parts.in_range.is_some() {
                    return Err(no("duplicates drop with if/in is not translated yet"));
                }
                if !vars.trim().is_empty() && !opts.has("force", 5) {
                    out.line("stop(\"duplicates drop with a varlist needs the force option\")");
                    return Ok(());
                }
                // the first row of each group stays, in the data's order
                out.line(format!(
                    "{d} <- {d} |> dplyr::distinct(dplyr::pick({sel}), .keep_all = TRUE)"
                ));
            }
            "tag" => {
                let g = opts
                    .arg("generate", 3)
                    .ok_or_else(|| no("duplicates tag needs generate()"))?;
                let g = g.trim().to_string();
                let cond = self.condition(c, out)?.unwrap_or_else(|| "TRUE".into());
                out.line(format!(
                    "{d} <- {d} |> dplyr::mutate(dup_rows = {cond}) |> dplyr::group_by(dplyr::pick({sel}), dup_rows) |> dplyr::mutate({gn} = dplyr::if_else(dup_rows, as.numeric(dplyr::n() - 1), NA_real_)) |> dplyr::ungroup() |> dplyr::select(-dup_rows)",
                    gn = r_name(&g)
                ));
                self.kinds.insert(g, Kind::Number);
            }
            "report" => {
                if Self::quiet(c) {
                    return Ok(());
                }
                out.line(format!(
                    "local({{ k <- dplyr::count({d}, dplyr::pick({sel}), name = \"copies\"); r <- dplyr::count(k, copies, name = \"groups\"); r$observations <- r$copies * r$groups; r$surplus <- r$observations - r$groups; print(as.data.frame(r[c(\"copies\", \"observations\", \"surplus\")]), row.names = FALSE) }})"
                ));
                out.caveat("printed in R's layout, not Stata's table");
            }
            other => return Err(no(format!("duplicates {other} is not translated yet"))),
        }
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
            "{d} <- {d} |> dplyr::mutate({gn} = (\\(v, src) {{ lv <- sort(unique(v[!is.na(v) & v != \"\"]), method = \"radix\"); haven::labelled(as.numeric(match(v, lv)), labels = stats::setNames(as.numeric(seq_along(lv)), lv), label = attr(src, \"label\")) }})({v}, {src}))",
            gn = r_name(&g),
            src = r_name(var)
        ));
        self.kinds.insert(g, Kind::Number);
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
        let vars = if c.parts.main.trim().is_empty() {
            "_all".to_string()
        } else {
            c.parts.main.clone()
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
