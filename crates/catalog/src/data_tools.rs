//! Data menu dialogs for changing the values of one variable (P3.5): recode,
//! convert type, factor levels, text and dates. Base R only: no package is
//! needed, so each runs against any R. Each writes a small function applied to
//! the column, so its helper names (x, y, lab) cannot clash with the data
//! frame's, and the result goes into the variable itself or a new one.

use rcraft_rcodegen::dialog::{
    no_check, Args, Opt, Problem, Spec, TextDefault, TextRule, VarKind, Widget,
};
use rcraft_rcodegen::r_string;

use crate::data::{copy_line, BASE, RESULT};

const INTO: Widget = Widget::Text {
    id: "into",
    label: "Put it in a new variable",
    default: TextDefault::Empty,
    rule: TextRule::Any,
    hint: "Leave empty to change the variable itself.",
};

const INTO_NAMED: Widget = Widget::Text {
    id: "into",
    label: "New variable",
    default: TextDefault::Empty,
    rule: TextRule::Name,
    hint: "",
};

const fn var(kinds: &'static [VarKind], hint: &'static str) -> Widget {
    Widget::Variable {
        id: "var",
        label: "Variable",
        kinds,
        required: true,
        hint,
    }
}

const TEXTY: &[VarKind] = &[VarKind::Text, VarKind::Factor];

/// The column the new values go into: the variable itself, or the new one.
fn target(a: &Args) -> String {
    let into = a.name_text("into");
    if into.is_empty() {
        a.var_string("var").unwrap_or_default()
    } else {
        a.name_string("into")
    }
}

/// `res[["target"]] <- (function(x) { body })(res[["var"]])`, after the copy
/// line. The body leaves its answer in the last expression.
fn column_fn(a: &Args, body: &str) -> String {
    let res = a.name("result");
    format!(
        "{copy}{res}[[{target}]] <- (function(x) {{\n{body}\n}})({res}[[{col}]])",
        copy = copy_line(a),
        target = target(a),
        col = a.var_string("var").unwrap_or_default(),
    )
}

/// A body that builds `y` from `x` and hands the variable's label on to it.
fn labelled(body: &str) -> String {
    format!("  lab <- attr(x, \"label\")\n{body}\n  attr(y, \"label\") <- lab\n  y")
}

/// Splits a list on `sep` into trimmed, non-empty parts.
fn parts(text: &str, sep: char) -> Vec<&str> {
    text.split(sep)
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect()
}

/// A text value as typed: inside double quotes it is exactly what is between
/// them (so "NA" is the text NA); the flag says whether it was quoted.
fn unquote(s: &str) -> (&str, bool) {
    let s = s.trim();
    match s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        Some(inner) => (inner, true),
        None => (s, false),
    }
}

/// A number as typed, when R reads it the same way.
fn number(s: &str) -> Option<&str> {
    let s = s.trim();
    s.parse::<f64>().ok().filter(|n| n.is_finite())?;
    Some(s)
}

// ---- Recode ----

enum Old<'a> {
    Missing,
    Value(&'a str),
    Range(&'a str, &'a str),
}

enum New<'a> {
    Missing,
    Number(&'a str),
    Text(&'a str),
}

struct Rule<'a> {
    old: Vec<Old<'a>>,
    new: New<'a>,
}

fn parse_old(item: &str, numeric: bool) -> Result<Old<'_>, String> {
    let (inner, quoted) = unquote(item);
    if !quoted && inner == "NA" {
        return Ok(Old::Missing);
    }
    if !numeric {
        return Ok(Old::Value(inner));
    }
    // a "to" at either end still counts as a range with a side missing
    let trimmed = item.trim();
    let range = trimmed
        .split_once(" to ")
        .or_else(|| trimmed.strip_suffix(" to").map(|lo| (lo, "")))
        .or_else(|| trimmed.strip_prefix("to ").map(|hi| ("", hi)));
    if let Some((lo, hi)) = range {
        return match (number(lo), number(hi)) {
            (Some(l), Some(h)) if l.parse::<f64>().ok() <= h.parse::<f64>().ok() => {
                Ok(Old::Range(l, h))
            }
            (Some(_), Some(_)) => Err(format!("\"{item}\" has its ends the wrong way round.")),
            _ => Err(format!("\"{item}\" needs a number on each side of \"to\".")),
        };
    }
    number(item)
        .map(Old::Value)
        .ok_or_else(|| format!("{item} is not a number, and this variable holds numbers."))
}

fn parse_new(text: &str) -> New<'_> {
    let (inner, quoted) = unquote(text);
    if quoted {
        New::Text(inner)
    } else if inner == "NA" {
        New::Missing
    } else if let Some(n) = number(inner) {
        New::Number(n)
    } else {
        New::Text(inner)
    }
}

/// `old, old = new; ...` as typed in the Recode dialog.
fn parse_rules(text: &str, numeric: bool) -> Result<Vec<Rule<'_>>, String> {
    let mut out = Vec::new();
    for part in parts(text, ';') {
        let Some((left, right)) = part.split_once('=') else {
            return Err(format!("\"{part}\" needs an = (for example 1, 2 = Low)."));
        };
        if left.trim().is_empty() || right.trim().is_empty() {
            return Err(format!(
                "\"{part}\" needs a value before and a new value after the =."
            ));
        }
        let old = parts(left, ',')
            .into_iter()
            .map(|item| parse_old(item, numeric))
            .collect::<Result<Vec<_>, _>>()?;
        out.push(Rule {
            old,
            new: parse_new(right),
        });
    }
    if out.is_empty() {
        return Err("Type at least one rule, for example: 1, 2 = Low".into());
    }
    Ok(out)
}

fn is_number_column(a: &Args) -> bool {
    a.ctx
        .column(a.var_text("var"))
        .is_some_and(|c| c.kind == VarKind::Number)
}

fn recode_check(a: &Args) -> Vec<Problem> {
    match parse_rules(a.expr("rules"), is_number_column(a)) {
        Ok(_) => Vec::new(),
        Err(why) => vec![Problem::new("rules", why)],
    }
}

fn recode_code(a: &Args, _: rcraft_rcodegen::dialog::Style) -> String {
    let numeric_in = is_number_column(a);
    let keep = a.choice("others") == "keep";
    let rules = parse_rules(a.expr("rules"), numeric_in).unwrap_or_default();
    // numbers come out as numbers only when nothing else has to be kept next to them
    let numeric_out = rules
        .iter()
        .all(|r| matches!(r.new, New::Missing | New::Number(_)))
        && (numeric_in || !keep);
    let mut b = String::from("  lab <- attr(x, \"label\")\n");
    b.push_str(if numeric_in {
        "  x <- as.vector(x)\n"
    } else {
        "  x <- as.character(x)\n"
    });
    b.push_str(&match (keep, numeric_out, numeric_in) {
        (true, false, true) => "  y <- as.character(x)\n".to_string(),
        (true, _, _) => "  y <- x\n".to_string(),
        (false, true, _) => "  y <- rep(NA_real_, length(x))\n".to_string(),
        (false, false, _) => "  y <- rep(NA_character_, length(x))\n".to_string(),
    });
    b.push_str("  done <- rep(FALSE, length(x))\n");
    for rule in &rules {
        let mut values: Vec<String> = Vec::new();
        let mut tests: Vec<String> = Vec::new();
        for old in &rule.old {
            match old {
                Old::Missing => tests.push("is.na(x)".into()),
                Old::Value(v) if numeric_in => values.push((*v).to_string()),
                Old::Value(v) => values.push(r_string(v)),
                Old::Range(lo, hi) => tests.push(format!("(!is.na(x) & x >= {lo} & x <= {hi})")),
            }
        }
        if !values.is_empty() {
            tests.insert(0, format!("x %in% c({})", values.join(", ")));
        }
        let new = match rule.new {
            New::Missing => "NA".to_string(),
            New::Number(n) if numeric_out => n.to_string(),
            New::Number(n) => r_string(n),
            New::Text(t) => r_string(t),
        };
        b.push_str(&format!(
            "  hit <- {}\n  y[hit & !done] <- {new}\n  done <- done | hit\n",
            tests.join(" | ")
        ));
    }
    b.push_str("  attr(y, \"label\") <- lab\n  y");
    column_fn(a, &b)
}

pub const RECODE: Spec = Spec {
    entry: "Recode",
    menu: "Data > Recode variable",
    about: "Change some values of a variable into others. The first rule that fits a value is \
            used; the labels of the variable are kept.",
    widgets: &[
        var(&[VarKind::Number, VarKind::Text, VarKind::Factor], ""),
        Widget::Text {
            id: "rules",
            label: "Rules",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Old values = new value, separated by ; for example: 1, 2 = Low; 3 to 5 = High; \
                   NA = 0. Use \"quotes\" for text that looks like a number or NA. \
                   Ranges (3 to 5) are for numbers. Values cannot contain , or ;",
        },
        Widget::Choice {
            id: "others",
            label: "Other values",
            options: &[
                Opt {
                    value: "keep",
                    label: "Keep as they are",
                },
                Opt {
                    value: "missing",
                    label: "Make missing",
                },
            ],
            default: "keep",
            hint: "A variable that gets text for some values becomes text.",
        },
        INTO,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: recode_check,
    code: recode_code,
};

// ---- Convert type ----

const TYPES: &[Opt] = &[
    Opt {
        value: "number",
        label: "Number",
    },
    Opt {
        value: "text",
        label: "Text",
    },
    Opt {
        value: "factor",
        label: "Factor (categories)",
    },
];

fn convert_check(a: &Args) -> Vec<Problem> {
    let kind = a.ctx.column(a.var_text("var")).map(|c| c.kind);
    let same = matches!(
        (kind, a.choice("to")),
        (Some(VarKind::Number), "number")
            | (Some(VarKind::Text), "text")
            | (Some(VarKind::Factor), "factor")
    );
    if same && a.name_text("into").is_empty() {
        vec![Problem::new(
            "to",
            format!("{} already is that type.", a.var_text("var")),
        )]
    } else {
        Vec::new()
    }
}

fn convert_code(a: &Args, _: rcraft_rcodegen::dialog::Style) -> String {
    let y = match a.choice("to") {
        // a factor's codes are not its values: go through its labels
        "number" => "  y <- as.numeric(if (is.factor(x)) as.character(x) else x)",
        "text" => "  y <- as.character(x)",
        _ => "  y <- factor(x)",
    };
    column_fn(a, &labelled(y))
}

pub const CONVERT_TYPE: Spec = Spec {
    entry: "Convert type",
    menu: "Data > Convert type",
    about: "Change a variable into a number, text or a factor. Text that is not a number \
            becomes missing, and R says how many.",
    widgets: &[
        var(
            &[
                VarKind::Number,
                VarKind::Text,
                VarKind::Factor,
                VarKind::Logical,
                VarKind::Date,
            ],
            "",
        ),
        Widget::Choice {
            id: "to",
            label: "Change it into",
            options: TYPES,
            default: "number",
            hint: "A factor becomes the numbers its labels spell, not its internal codes.",
        },
        INTO,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: convert_check,
    code: convert_code,
};

// ---- Factor levels ----

/// Level names typed as a list separated by commas.
fn level_list(text: &str) -> Vec<&str> {
    parts(text, ',').into_iter().map(|l| unquote(l).0).collect()
}

fn r_vector(items: &[&str]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| r_string(s)).collect();
    format!("c({})", quoted.join(", "))
}

pub const FACTORS: &[VarKind] = &[VarKind::Factor];

pub const REFERENCE_LEVEL: Spec = Spec {
    entry: "Reference level",
    menu: "Data > Factor > Reference level",
    about: "Make one level of a factor the first, which models compare the others with.",
    widgets: &[
        var(FACTORS, ""),
        Widget::Text {
            id: "level",
            label: "Reference level",
            default: TextDefault::Empty,
            rule: TextRule::Name,
            hint: "One of the variable's levels, exactly as it is spelled.",
        },
        INTO,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let body = format!("  y <- stats::relevel(x, ref = {})", a.name_string("level"));
        column_fn(a, &labelled(&body))
    },
};

fn combine_groups(text: &str) -> Result<Vec<(Vec<&str>, &str)>, String> {
    let mut out: Vec<(Vec<&str>, &str)> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for part in parts(text, ';') {
        let Some((left, right)) = part.split_once('=') else {
            return Err(format!(
                "\"{part}\" needs an = (for example Agree, Strongly agree = Agree)."
            ));
        };
        let old = level_list(left);
        let new = unquote(right).0;
        if old.is_empty() || new.is_empty() {
            return Err(format!(
                "\"{part}\" needs levels before the = and a new level after it."
            ));
        }
        for level in &old {
            if seen.contains(level) {
                return Err(format!("{level} is in two groups."));
            }
            seen.push(level);
        }
        out.push((old, new));
    }
    if out.is_empty() {
        return Err("Type at least one group.".into());
    }
    Ok(out)
}

pub const COMBINE_LEVELS: Spec = Spec {
    entry: "Combine levels",
    menu: "Data > Factor > Combine levels",
    about: "Merge levels of a factor into new ones. Levels you do not mention stay as they are.",
    widgets: &[
        var(FACTORS, ""),
        Widget::Text {
            id: "groups",
            label: "Groups",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Old levels = new level, separated by ; for example: \
                   Agree, Strongly agree = Agree; Disagree, Strongly disagree = Disagree. \
                   Levels cannot contain , or ;",
        },
        INTO,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: |a| match combine_groups(a.expr("groups")) {
        Ok(_) => Vec::new(),
        Err(why) => vec![Problem::new("groups", why)],
    },
    code: |a, _| {
        let groups = combine_groups(a.expr("groups")).unwrap_or_default();
        let (mut from, mut to) = (Vec::new(), Vec::new());
        for (old, new) in &groups {
            for level in old {
                from.push(*level);
                to.push(*new);
            }
        }
        let body = format!(
            "  m <- stats::setNames({}, {})\n  lv <- levels(x)\n  bad <- setdiff(names(m), lv)\n  \
             if (length(bad) > 0) stop(\"not levels of the variable: \", paste(bad, collapse = \", \"))\n  \
             lv[match(names(m), lv)] <- m\n  levels(x) <- lv\n  x",
            r_vector(&to),
            r_vector(&from),
        );
        column_fn(a, &body)
    },
};

pub const REORDER_LEVELS: Spec = Spec {
    entry: "Reorder levels",
    menu: "Data > Factor > Reorder levels",
    about: "Put the levels of a factor in the order you want, which tables and graphs follow.",
    widgets: &[
        var(FACTORS, ""),
        Widget::Text {
            id: "levels",
            label: "Levels in order",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Every level exactly once, separated by commas, for example: low, medium, high",
        },
        INTO,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        let levels = level_list(a.expr("levels"));
        let mut seen: Vec<&str> = Vec::new();
        if levels.is_empty() {
            return vec![Problem::new(
                "levels",
                "Type the levels, separated by commas.",
            )];
        }
        for l in levels {
            if seen.contains(&l) {
                return vec![Problem::new("levels", format!("{l} is listed twice."))];
            }
            seen.push(l);
        }
        Vec::new()
    },
    code: |a, _| {
        let levels = level_list(a.expr("levels"));
        let body = format!(
            "  lv <- {}\n  if (!setequal(lv, levels(x))) stop(\"list every level of the variable \
             exactly once\")\n  y <- factor(x, levels = lv, ordered = is.ordered(x))",
            r_vector(&levels)
        );
        column_fn(a, &labelled(&body))
    },
};

// ---- Text ----

const CASES: &[Opt] = &[
    Opt {
        value: "trim",
        label: "Remove spaces at the start and end",
    },
    Opt {
        value: "squish",
        label: "Remove those and extra spaces inside",
    },
    Opt {
        value: "upper",
        label: "UPPER CASE",
    },
    Opt {
        value: "lower",
        label: "lower case",
    },
    Opt {
        value: "title",
        label: "Title Case",
    },
];

pub const TEXT_CASE: Spec = Spec {
    entry: "Text case and spaces",
    menu: "Data > Text > Case and spaces",
    about: "Tidy the text of a variable: spaces and capital letters. Missing values stay missing.",
    widgets: &[
        var(TEXTY, ""),
        Widget::Choice {
            id: "how",
            label: "Change",
            options: CASES,
            default: "trim",
            hint: "",
        },
        INTO,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let y = match a.choice("how") {
            "squish" => "  y <- trimws(gsub(\"[[:space:]]+\", \" \", as.character(x)))",
            "upper" => "  y <- toupper(as.character(x))",
            "lower" => "  y <- tolower(as.character(x))",
            "title" => {
                "  y <- as.character(x)\n  ok <- !is.na(y)\n  y[ok] <- tools::toTitleCase(y[ok])"
            }
            _ => "  y <- trimws(as.character(x))",
        };
        column_fn(a, &labelled(y))
    },
};

/// `text` with the characters a pattern treats as special made literal.
fn escape_pattern(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if "\\.|()[]{}^$*+?".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn find_check(a: &Args) -> Vec<Problem> {
    if a.values.text("find").is_empty() {
        vec![Problem::new("find", "Type the text to find.")]
    } else {
        Vec::new()
    }
}

pub const FIND_REPLACE: Spec = Spec {
    entry: "Find and replace text",
    menu: "Data > Text > Find and replace",
    about: "Replace every occurrence of some text inside the values of a variable.",
    widgets: &[
        var(TEXTY, ""),
        Widget::Text {
            id: "find",
            label: "Find",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "",
        },
        Widget::Text {
            id: "with",
            label: "Replace with",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Leave empty to delete the text.",
        },
        Widget::Check {
            id: "pattern",
            label: "Treat the text to find as a pattern (regular expression)",
            default: false,
            hint: "",
        },
        Widget::Check {
            id: "ignore",
            label: "Ignore capital letters",
            default: false,
            hint: "",
        },
        INTO,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: find_check,
    code: |a, _| {
        let (find, with) = (a.values.text("find"), a.values.text("with"));
        let (pattern, ignore) = (a.flag("pattern"), a.flag("ignore"));
        let call = if pattern {
            format!(
                "gsub({}, {}, as.character(x), ignore.case = {})",
                r_string(find),
                r_string(with),
                if ignore { "TRUE" } else { "FALSE" }
            )
        } else if ignore {
            // literal text, but R can only ignore case in a pattern
            format!(
                "gsub({}, {}, as.character(x), ignore.case = TRUE)",
                r_string(&escape_pattern(find)),
                r_string(&with.replace('\\', "\\\\")),
            )
        } else {
            format!(
                "gsub({}, {}, as.character(x), fixed = TRUE)",
                r_string(find),
                r_string(with)
            )
        };
        column_fn(a, &labelled(&format!("  y <- {call}")))
    },
};

fn extract_bounds(a: &Args) -> Result<(u32, Option<u32>), String> {
    let from = a.expr("from");
    let Some(from) = from.parse::<u32>().ok().filter(|n| *n >= 1) else {
        return Err("Type the first character's position, a whole number from 1.".into());
    };
    let to = a.expr("to");
    if to.is_empty() {
        return Ok((from, None));
    }
    match to.parse::<u32>() {
        Ok(n) if n >= from => Ok((from, Some(n))),
        _ => Err("Type a whole number that is not before the first position.".into()),
    }
}

pub const EXTRACT_TEXT: Spec = Spec {
    entry: "Extract part of text",
    menu: "Data > Text > Extract part",
    about: "Keep only some characters of each value, counted from the left, starting at 1.",
    widgets: &[
        var(TEXTY, ""),
        Widget::Text {
            id: "from",
            label: "From character",
            default: TextDefault::Fixed("1"),
            rule: TextRule::Any,
            hint: "",
        },
        Widget::Text {
            id: "to",
            label: "To character",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Leave empty to go to the end.",
        },
        INTO,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: |a| match extract_bounds(a) {
        Ok(_) => Vec::new(),
        Err(why) => vec![Problem::new(
            if a.expr("from").parse::<u32>().is_ok() {
                "to"
            } else {
                "from"
            },
            why,
        )],
    },
    code: |a, _| {
        let call = match extract_bounds(a) {
            Ok((from, Some(to))) => format!("substring(as.character(x), {from}, {to})"),
            Ok((from, None)) => format!("substring(as.character(x), {from})"),
            Err(_) => "as.character(x)".to_string(),
        };
        column_fn(a, &labelled(&format!("  y <- {call}")))
    },
};

// ---- Dates ----

const DATE_LAYOUTS: &[Opt] = &[
    Opt {
        value: "%Y-%m-%d",
        label: "Year-month-day (2026-03-27)",
    },
    Opt {
        value: "%d/%m/%Y",
        label: "Day/month/year (27/03/2026)",
    },
    Opt {
        value: "%m/%d/%Y",
        label: "Month/day/year (03/27/2026)",
    },
    Opt {
        value: "%Y%m%d",
        label: "Year month day run together (20260327)",
    },
];

pub const TEXT_TO_DATE: Spec = Spec {
    entry: "Text to date",
    menu: "Data > Date > Text to date",
    about: "Turn text that spells dates into real dates. Values that do not fit the layout \
            become missing, and R says how many.",
    widgets: &[
        var(TEXTY, ""),
        Widget::Choice {
            id: "layout",
            label: "The text is written as",
            options: DATE_LAYOUTS,
            default: "%Y-%m-%d",
            hint: "",
        },
        INTO,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let body = format!(
            "  y <- as.Date(as.character(x), format = {})\n  \
             bad <- sum(is.na(y) & !is.na(x) & nzchar(as.character(x)))\n  \
             if (bad > 0) warning(bad, \" values did not fit the date layout and are now missing\")",
            r_string(a.choice("layout"))
        );
        column_fn(a, &labelled(&body))
    },
};

const DATE_PARTS: &[Opt] = &[
    Opt {
        value: "year",
        label: "Year",
    },
    Opt {
        value: "month",
        label: "Month (1 to 12)",
    },
    Opt {
        value: "day",
        label: "Day of the month",
    },
    Opt {
        value: "weekday",
        label: "Day of the week (name)",
    },
    Opt {
        value: "quarter",
        label: "Quarter (1 to 4)",
    },
];

pub const DATE_PART: Spec = Spec {
    entry: "Date part",
    menu: "Data > Date > Date part",
    about: "Make a new variable holding the year, month, day or weekday of a date.",
    widgets: &[
        var(
            &[VarKind::Date],
            "Turn text into dates first (Text to date).",
        ),
        Widget::Choice {
            id: "part",
            label: "Part",
            options: DATE_PARTS,
            default: "year",
            hint: "",
        },
        INTO_NAMED,
        RESULT,
    ],
    needs: &[],
    styles: BASE,
    check: no_check,
    code: |a, _| {
        let y = match a.choice("part") {
            "month" => "  as.integer(format(x, \"%m\"))",
            "day" => "  as.integer(format(x, \"%d\"))",
            "weekday" => "  weekdays(x)",
            "quarter" => "  (as.integer(format(x, \"%m\")) - 1L) %/% 3L + 1L",
            _ => "  as.integer(format(x, \"%Y\"))",
        };
        column_fn(a, y)
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, Style, Values};

    fn ctx() -> Context {
        let col = |name: &str, kind| ColumnInfo {
            name: name.into(),
            kind,
        };
        Context {
            dataset: "people".into(),
            columns: vec![
                col("age", VarKind::Number),
                col("sex", VarKind::Factor),
                col("town", VarKind::Text),
                col("born", VarKind::Date),
            ],
        }
    }

    fn make(spec: &Spec, set: &[(&str, &str)]) -> Result<String, Vec<Problem>> {
        let c = ctx();
        let mut v: Values = spec.defaults(&c);
        for (id, text) in set {
            v.set_text(id, *text);
        }
        spec.generate(&c, &v, Style::Base)
    }

    fn code(spec: &Spec, set: &[(&str, &str)]) -> String {
        make(spec, set).unwrap_or_default()
    }

    /// The problems a dialog reports, or "no problem" and its code.
    fn message(spec: &Spec, set: &[(&str, &str)]) -> String {
        match make(spec, set) {
            Ok(code) => format!("no problem: {code}"),
            Err(p) => p
                .iter()
                .map(|p| p.message.clone())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    #[test]
    fn recode_numbers_to_numbers_in_place() {
        assert_eq!(
            code(
                &RECODE,
                &[
                    ("var", "age"),
                    ("rules", "99 = NA; 0 to 17 = 1"),
                    ("others", "missing")
                ]
            ),
            "people[[\"age\"]] <- (function(x) {\n  lab <- attr(x, \"label\")\n  x <- as.vector(x)\n  \
             y <- rep(NA_real_, length(x))\n  done <- rep(FALSE, length(x))\n  \
             hit <- x %in% c(99)\n  y[hit & !done] <- NA\n  done <- done | hit\n  \
             hit <- (!is.na(x) & x >= 0 & x <= 17)\n  y[hit & !done] <- 1\n  done <- done | hit\n  \
             attr(y, \"label\") <- lab\n  y\n})(people[[\"age\"]])"
        );
    }

    #[test]
    fn recode_text_news_make_text_and_a_copy_keeps_the_original() {
        let out = code(
            &RECODE,
            &[
                ("var", "age"),
                ("rules", "1, NA = low"),
                ("into", "band"),
                ("result", "p"),
            ],
        );
        assert!(
            out.starts_with("p <- people\np[[\"band\"]] <- (function(x) {"),
            "{out}"
        );
        assert!(out.contains("y <- as.character(x)"), "{out}");
        assert!(out.contains("hit <- x %in% c(1) | is.na(x)"), "{out}");
        assert!(out.contains("y[hit & !done] <- \"low\""), "{out}");
        assert!(out.ends_with("})(p[[\"age\"]])"), "{out}");
    }

    #[test]
    fn recode_refuses_what_it_cannot_read() {
        let age = |rules| message(&RECODE, &[("var", "age"), ("rules", rules)]);
        assert!(age("").contains("at least one rule"));
        assert!(age("1 Low").contains("needs an ="));
        assert!(age("1 = ").contains("needs a value before"));
        assert!(age("one = 1").contains("is not a number"));
        assert!(age("9 to 3 = 1").contains("wrong way round"));
        assert!(age("1 to = 1").contains("each side"));
        // text variables take any value
        assert!(
            message(&RECODE, &[("var", "town"), ("rules", "Oslo = NO")]).starts_with("no problem")
        );
        assert!(message(&RECODE, &[("var", "born"), ("rules", "1 = 2")]).contains("date"));
    }

    #[test]
    fn convert_refuses_the_same_type_only_in_place() {
        assert!(message(&CONVERT_TYPE, &[("var", "age"), ("to", "number")]).contains("already"));
        let copy = [("var", "age"), ("to", "number"), ("into", "age2")];
        assert!(message(&CONVERT_TYPE, &copy).starts_with("no problem"));
        let out = code(&CONVERT_TYPE, &[("var", "sex"), ("to", "number")]);
        assert!(
            out.contains("as.numeric(if (is.factor(x)) as.character(x) else x)"),
            "{out}"
        );
    }

    #[test]
    fn factor_dialogs_read_their_lists() {
        let combine = |groups| message(&COMBINE_LEVELS, &[("var", "sex"), ("groups", groups)]);
        assert!(combine("").contains("at least one"));
        assert!(combine("a b").contains("needs an ="));
        assert!(combine("a, b = x; b = y").contains("b is in two groups"));
        let out = code(
            &COMBINE_LEVELS,
            &[("var", "sex"), ("groups", "a, b = ab; c = cee")],
        );
        assert!(
            out.contains("stats::setNames(c(\"ab\", \"ab\", \"cee\"), c(\"a\", \"b\", \"c\"))"),
            "{out}"
        );
        let order = |levels| message(&REORDER_LEVELS, &[("var", "sex"), ("levels", levels)]);
        assert!(order("a, b, a").contains("twice"));
        assert!(order(" , ").contains("Type the levels"));
        assert!(message(&REFERENCE_LEVEL, &[("var", "age"), ("level", "a")]).contains("needs"));
        assert!(message(&REFERENCE_LEVEL, &[("var", "sex"), ("level", "")]).contains("Type"));
    }

    #[test]
    fn find_and_replace_is_literal_unless_asked() {
        let plain = code(
            &FIND_REPLACE,
            &[("var", "town"), ("find", "a.b"), ("with", "\\1")],
        );
        assert!(
            plain.contains("gsub(\"a.b\", \"\\\\1\", as.character(x), fixed = TRUE)"),
            "{plain}"
        );
        let c = ctx();
        let mut v = FIND_REPLACE.defaults(&c);
        v.set_text("var", "town");
        v.set_text("find", "a.b");
        v.set_text("with", "\\1");
        v.set_flag("ignore", true);
        let ignoring = FIND_REPLACE
            .generate(&c, &v, Style::Base)
            .unwrap_or_default();
        assert!(
            ignoring
                .contains("gsub(\"a\\\\.b\", \"\\\\\\\\1\", as.character(x), ignore.case = TRUE)"),
            "{ignoring}"
        );
        assert!(message(&FIND_REPLACE, &[("var", "town"), ("find", "")]).contains("Type the text"));
        assert!(message(&FIND_REPLACE, &[("var", "age"), ("find", "x")]).contains("age"));
    }

    #[test]
    fn extract_checks_its_positions() {
        let set = |from, to| {
            message(
                &EXTRACT_TEXT,
                &[("var", "town"), ("from", from), ("to", to)],
            )
        };
        assert!(set("0", "").contains("from 1"));
        assert!(set("x", "").contains("from 1"));
        assert!(set("5", "2").contains("not before"));
        assert!(set("2", "2").starts_with("no problem"));
        assert!(set("2", "").contains("substring(as.character(x), 2)"));
        assert!(set("2", "4").contains("substring(as.character(x), 2, 4)"));
    }

    #[test]
    fn dates_need_a_layout_and_a_date_variable() {
        let out = code(&TEXT_TO_DATE, &[("var", "town"), ("layout", "%d/%m/%Y")]);
        assert!(
            out.contains("as.Date(as.character(x), format = \"%d/%m/%Y\")"),
            "{out}"
        );
        assert!(out.contains("warning(bad, \" values did not fit"), "{out}");
        assert!(make(&TEXT_TO_DATE, &[("var", "town"), ("layout", "%Q")]).is_err());
        assert!(message(&DATE_PART, &[("var", "town"), ("into", "y")]).contains("needs"));
        assert!(message(&DATE_PART, &[("var", "born"), ("into", "")]).contains("Type"));
        let year = code(
            &DATE_PART,
            &[("var", "born"), ("part", "year"), ("into", "y")],
        );
        assert!(
            year.starts_with("people[[\"y\"]] <- (function(x) {\n  as.integer(format(x, \"%Y\"))"),
            "{year}"
        );
    }

    #[test]
    fn every_dialog_here_is_in_the_data_menu() {
        for spec in [
            &RECODE,
            &CONVERT_TYPE,
            &REFERENCE_LEVEL,
            &COMBINE_LEVELS,
            &REORDER_LEVELS,
            &TEXT_CASE,
            &FIND_REPLACE,
            &EXTRACT_TEXT,
            &TEXT_TO_DATE,
            &DATE_PART,
        ] {
            assert!(
                crate::data::SPECS.iter().any(|s| s.menu == spec.menu),
                "{} is not in the Data menu",
                spec.entry
            );
            assert!(spec.menu.starts_with("Data > "));
        }
    }
}
