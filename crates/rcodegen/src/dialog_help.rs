//! The help page of a dialog (P2A.5): what it does, each option it offers,
//! the packages it needs, an example of the R it writes (made with invented
//! variables), and the R functions that code calls, each a link to R's own
//! help page for it.
//!
//! Everything comes from the dialog's own [`Spec`], so a page can never
//! disagree with its dialog and a new dialog has a page the day it is added.
//! The words are RCraft's (the dialog's `about`, labels and hints); R's pages
//! are shown as R's, by the Help window.

use crate::dialog::{
    ColumnInfo, Context, Frame, Opt, Spec, Style, TextDefault, TextRule, Values, VarKind, Widget,
};

/// One option of a dialog, as the page lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionHelp {
    pub label: String,
    /// What it does: the dialog's hint, with the choices of a list added.
    pub text: String,
}

/// A dialog's help page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogHelp {
    /// The dialog's name (`"Sort rows"`).
    pub title: String,
    /// Where it is in the menus (`"Data > Sort rows"`).
    pub menu: String,
    /// What it does, in one line.
    pub about: String,
    pub options: Vec<OptionHelp>,
    /// Packages the dialog needs in its first (default) style; empty for base R.
    pub needs: Vec<String>,
    /// Packages a second style adds, with its name.
    pub other_style: Option<(String, Vec<String>)>,
    /// R it writes, for invented variables, in the default style. `None` when
    /// the example could not be made.
    pub example: Option<String>,
    /// R functions the code calls (`name` or `package::name`), each a help link.
    pub functions: Vec<String>,
    /// Longer words for the page, for the windows that are not catalog
    /// dialogs (Use my file, Packages). Empty for a catalog dialog.
    pub notes: Vec<String>,
}

/// Names that are R's grammar or too common to be worth a link.
const SKIPPED: &[&str] = &[
    "if",
    "for",
    "while",
    "repeat",
    "function",
    "return",
    "c",
    "list",
    "local",
    "invisible",
    "switch",
    "identical",
    "isTRUE",
    "is.null",
    "stop",
    "tryCatch",
    "vapply",
    "lapply",
    "sapply",
    "seq_len",
    "seq_along",
    "length",
    "nrow",
    "ncol",
    "names",
    "colnames",
    "rownames",
    "paste",
    "paste0",
    "sprintf",
    "cat",
    "print",
    "file.exists",
    "quote",
    "NULL",
];

/// The R functions called in `code`: `name(` or `pkg::name(`, outside strings,
/// comments and `` `names` ``, without the grammar words and the functions the
/// code defines itself; each once, in order of first use.
pub fn functions_in(code: &str) -> Vec<String> {
    let chars: Vec<char> = code.chars().collect();
    let mut defined: Vec<String> = Vec::new();
    let mut found: Vec<String> = Vec::new();
    let mut i = 0usize;
    let at = |k: usize| chars.get(k).copied();
    while let Some(c) = at(i) {
        match c {
            '"' | '\'' | '`' => {
                // skip the quoted text, honouring backslash escapes
                i += 1;
                while let Some(d) = at(i) {
                    if d == '\\' {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    if d == c {
                        break;
                    }
                }
            }
            '#' => {
                while at(i).is_some_and(|d| d != '\n') {
                    i += 1;
                }
            }
            c if c.is_alphabetic() || c == '.' => {
                let start = i;
                while at(i).is_some_and(|d| d.is_alphanumeric() || matches!(d, '.' | '_')) {
                    i += 1;
                }
                // `pkg::name`
                if at(i) == Some(':') && at(i + 1) == Some(':') && at(i + 2) != Some(':') {
                    i += 2;
                    while at(i).is_some_and(|d| d.is_alphanumeric() || matches!(d, '.' | '_')) {
                        i += 1;
                    }
                }
                let word: String = chars.get(start..i).unwrap_or(&[]).iter().collect();
                let after_dollar =
                    start > 0 && chars.get(start - 1).is_some_and(|p| matches!(p, '$' | '@'));
                if at(i) == Some('(') && !after_dollar {
                    let bare = word.rsplit("::").next().unwrap_or(&word).to_string();
                    if !SKIPPED.contains(&bare.as_str()) && !found.contains(&word) {
                        found.push(word);
                    }
                } else if !after_dollar && is_function_definition(&chars, i) {
                    defined.push(word);
                }
            }
            _ => i += 1,
        }
    }
    found.retain(|f| !defined.contains(f));
    found
}

/// Whether the text after a name is `<- function` (a function the code defines).
fn is_function_definition(chars: &[char], mut i: usize) -> bool {
    let skip_space = |i: &mut usize| {
        while chars.get(*i).is_some_and(|c| c.is_whitespace()) {
            *i += 1;
        }
    };
    skip_space(&mut i);
    if chars.get(i) != Some(&'<') || chars.get(i + 1) != Some(&'-') {
        return false;
    }
    i += 2;
    skip_space(&mut i);
    chars
        .get(i..i + 8)
        .is_some_and(|w| w.iter().collect::<String>() == "function")
}

/// Invented columns for an example: three numbers, two factors, text, a date
/// and a yes/no.
fn sample_columns() -> Vec<ColumnInfo> {
    let col = |name: &str, kind| ColumnInfo {
        name: name.to_string(),
        kind,
    };
    vec![
        col("x", VarKind::Number),
        col("y", VarKind::Number),
        col("z", VarKind::Number),
        col("group", VarKind::Factor),
        col("sex", VarKind::Factor),
        col("name", VarKind::Text),
        col("when", VarKind::Date),
        col("flag", VarKind::Logical),
    ]
}

fn sample_context() -> Context {
    Context {
        dataset: "data".into(),
        columns: sample_columns(),
        frames: vec![Frame {
            name: "other".into(),
            columns: sample_columns(),
        }],
    }
}

/// Text for a box whose rule cannot say what it wants: the boxes that expect
/// a particular little syntax, by their widget id.
fn sample_text(id: &str, rule: TextRule) -> Option<&'static str> {
    match id {
        "rules" => Some("1, 2 = Low"),
        "groups" => Some("Agree, Strongly agree = Agree"),
        "levels" => Some("low, medium, high"),
        "find" => Some("old"),
        "others" => Some("fit2"),
        _ => match rule {
            TextRule::Name => Some("new_name"),
            TextRule::Number => Some("1"),
            TextRule::Expression => Some("x"),
            TextRule::SavePath | TextRule::Any => None,
        },
    }
}

/// Values for the example: the dialog's defaults, a different fitting column
/// in every required variable box (starting `shift` columns along, so a
/// dialog that refuses one pairing gets another), and a plausible word in a
/// box that must not be empty. `file` goes in the save-path box.
fn sample_values(spec: &Spec, ctx: &Context, shift: usize, file: &str) -> Values {
    let mut values = spec.defaults(ctx);
    let mut order: Vec<&ColumnInfo> = ctx.columns.iter().collect();
    let turn = shift.checked_rem(order.len()).unwrap_or(0);
    order.rotate_left(turn);
    let mut used: Vec<String> = Vec::new();
    for w in spec.widgets {
        match *w {
            Widget::Variable {
                id,
                kinds,
                required,
                ..
            } => {
                if required {
                    if let Some(name) = pick(&order, kinds, &mut used) {
                        values.set_text(id, name);
                    }
                }
            }
            Widget::Variables { id, kinds, min, .. } => {
                let mut picked: Vec<String> = Vec::new();
                for _ in 0..min.max(1) {
                    if let Some(name) = pick(&order, kinds, &mut used) {
                        picked.push(name);
                    }
                }
                values.set_list(id, picked);
            }
            Widget::Frame { id, .. } => values.set_text(id, "other"),
            Widget::FrameVariables { id, min, .. } => {
                let picked: Vec<String> = ctx
                    .columns
                    .iter()
                    .take(min.max(1))
                    .map(|c| c.name.clone())
                    .collect();
                values.set_list(id, picked);
            }
            Widget::Text {
                id, rule, default, ..
            } => {
                if values.text(id).is_empty() && matches!(default, TextDefault::Empty) {
                    if rule == TextRule::SavePath || id == "file" {
                        values.set_text(id, file);
                    } else if let Some(s) = sample_text(id, rule) {
                        values.set_text(id, s);
                    }
                }
            }
            Widget::Choice { .. } | Widget::Check { .. } => {}
        }
    }
    values
}

/// A column that fits `kinds` and is not in `used` yet (or, failing that, one
/// that fits), recorded in `used`.
fn pick(order: &[&ColumnInfo], kinds: &[VarKind], used: &mut Vec<String>) -> Option<String> {
    let fitting: Vec<&&ColumnInfo> = order
        .iter()
        .filter(|c| kinds.is_empty() || kinds.contains(&c.kind))
        .collect();
    let chosen = fitting
        .iter()
        .find(|c| !used.contains(&c.name))
        .or_else(|| fitting.first())?;
    used.push(chosen.name.clone());
    Some(chosen.name.clone())
}

/// File names a save dialog might insist on, tried in turn.
const SAMPLE_FILES: &[&str] = &[
    "data.csv",
    "data.xlsx",
    "data.dta",
    "data.rds",
    "data.parquet",
    "data.RData",
    "data.png",
];

/// The first values for which `spec` writes code in `style`, with that code.
fn sample_code(spec: &Spec, ctx: &Context, style: Style) -> Option<String> {
    for shift in 0..ctx.columns.len().max(1) {
        for file in SAMPLE_FILES {
            let values = sample_values(spec, ctx, shift, file);
            if let Ok(code) = spec.generate(ctx, &values, style) {
                return Some(code);
            }
        }
    }
    None
}

fn option_text(hint: &str, options: &[Opt]) -> String {
    let mut text = hint.to_string();
    if !options.is_empty() {
        let labels: Vec<&str> = options.iter().map(|o| o.label).collect();
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str("Choices: ");
        text.push_str(&labels.join("; "));
        text.push('.');
    }
    text
}

/// The kinds a variable box accepts, in words (`number or factor`).
fn kinds_phrase(kinds: &[VarKind]) -> String {
    if kinds.is_empty() {
        return "any type".to_string();
    }
    kinds
        .iter()
        .map(|k| k.label())
        .collect::<Vec<_>>()
        .join(" or ")
}

/// `first` and then the hint, as one text.
fn with_hint(first: String, hint: &str) -> String {
    if hint.is_empty() {
        first
    } else {
        format!("{first} {hint}")
    }
}

fn options_of(spec: &Spec) -> Vec<OptionHelp> {
    spec.widgets
        .iter()
        .map(|w| match *w {
            Widget::Variable {
                label,
                hint,
                kinds,
                required,
                ..
            } => {
                let first = format!(
                    "One variable ({}){}",
                    kinds_phrase(kinds),
                    if required || hint.contains("ptional") {
                        "."
                    } else {
                        ". Optional."
                    }
                );
                OptionHelp {
                    label: label.to_string(),
                    text: with_hint(first, hint),
                }
            }
            Widget::Variables {
                label,
                hint,
                kinds,
                min,
                ..
            } => {
                let count = if min > 1 {
                    format!("at least {min}")
                } else if min == 1 {
                    "one or more".to_string()
                } else {
                    "any number, or none for all".to_string()
                };
                OptionHelp {
                    label: label.to_string(),
                    text: with_hint(
                        format!("Variables ({}): {count}.", kinds_phrase(kinds)),
                        hint,
                    ),
                }
            }
            Widget::Text { label, hint, .. }
            | Widget::Check { label, hint, .. }
            | Widget::Frame { label, hint, .. }
            | Widget::FrameVariables { label, hint, .. } => OptionHelp {
                label: label.to_string(),
                text: option_text(hint, &[]),
            },
            Widget::Choice {
                label,
                hint,
                options,
                ..
            } => OptionHelp {
                label: label.to_string(),
                text: option_text(hint, options),
            },
        })
        .collect()
}

/// The help page of a catalog dialog.
pub fn for_spec(spec: &Spec) -> DialogHelp {
    let ctx = sample_context();
    let default_style = spec.styles.first().copied().unwrap_or_default();
    let example = sample_code(spec, &ctx, default_style);
    // functions from every style's code, so a dialog that can write two ways
    // links the functions of both
    let mut functions: Vec<String> = Vec::new();
    for style in spec.styles {
        if let Some(code) = sample_code(spec, &ctx, *style) {
            for f in functions_in(&code) {
                if !functions.contains(&f) {
                    functions.push(f);
                }
            }
        }
    }
    let other_style = spec
        .styles
        .iter()
        .skip(1)
        .map(|s| {
            (
                s.label().to_string(),
                spec.packages(*s).iter().map(|p| p.to_string()).collect(),
            )
        })
        .next();
    DialogHelp {
        title: spec.entry.to_string(),
        menu: spec.menu.to_string(),
        about: spec.about.to_string(),
        options: options_of(spec),
        needs: spec
            .packages(default_style)
            .iter()
            .map(|p| p.to_string())
            .collect(),
        other_style,
        example,
        functions,
        notes: Vec::new(),
    }
}

/// The page of "Use my file", which is not a catalog dialog.
pub fn use_my_file() -> DialogHelp {
    DialogHelp {
        title: "Use my file".into(),
        menu: "File > Use my file".into(),
        about: "Reads a data file into a data frame in the R session.".into(),
        options: Vec::new(),
        needs: Vec::new(),
        other_style: None,
        example: Some("data <- read.csv(\"survey.csv\", stringsAsFactors = FALSE)".into()),
        functions: ["read.csv", "readxl::read_excel", "haven::read_dta", "haven::read_sav", "arrow::read_parquet", "readRDS"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        notes: vec![
            "Choose a file with Browse, or pick one of the ten most recent. RCraft looks at the file first and shows R's own preview, so you can set the separator, quotes, missing-value words, sheet and cell range before anything is read.".into(),
            "Text files (csv, tsv, txt) need no package. Excel (readxl), Stata, SPSS and SAS (haven), and Parquet or Feather (arrow) need an R package; the dialog offers to install it, and you say yes first.".into(),
            "The R it writes is shown before it runs, and it is added to the Results, the log and the script like any command.".into(),
        ],
    }
}

/// The page of the Packages window, which is not a catalog dialog.
pub fn packages_window() -> DialogHelp {
    DialogHelp {
        title: "Packages".into(),
        menu: "Packages > Find and install".into(),
        about: "Finds, describes, installs, updates and removes R packages, like ssc and net search for Stata.".into(),
        options: Vec::new(),
        needs: Vec::new(),
        other_style: None,
        example: Some("install.packages(\"fixest\")".into()),
        functions: [
            "install.packages",
            "remove.packages",
            "installed.packages",
            "available.packages",
            "update.packages",
            "BiocManager::install",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        notes: vec![
            "Find searches the CRAN package list kept on this computer; press Refresh to download a newer list (it says which server it asks). Nothing is sent but your search words and package names.".into(),
            "Every install, update and removal shows exactly what will happen, with the R call, and waits for your yes. Packages run their authors' code when installed, so RCraft never installs on its own.".into(),
            "Other sources: Bioconductor, r-universe, GitHub and local files are labelled as reviewed or not; the ones nobody reviews need a tick before the Install button turns on.".into(),
            "In the Command window, rc install, rc search, rc describe, rc installed, rc update and rc remove do the same things.".into(),
        ],
    }
}

/// Every style a spec can write, as a label (for tests and the page).
pub fn style_labels(spec: &Spec) -> Vec<&'static str> {
    spec.styles.iter().map(|s: &Style| s.label()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_are_found_outside_strings_comments_and_definitions() {
        let code = "res <- (function(x, y) {\n  for (n in setdiff(names(y), names(x))) x[[n]] <- NA\n  rbind(x, y[names(x)])\n})(a, b)\n# merge(a, b)\nz <- \"aggregate(x)\"\nstats::sd(c(1, 2)) ; df$mean(1) ; `weird name`(1)";
        assert_eq!(functions_in(code), vec!["setdiff", "rbind", "stats::sd"]);
    }

    #[test]
    fn a_function_the_code_defines_is_not_a_link() {
        let code = "fix <- function(v) v + 1\nfix(2)\nmerge(a, b)";
        assert_eq!(functions_in(code), vec!["merge"]);
    }

    #[test]
    fn escaped_quotes_do_not_end_a_string() {
        let code = "x <- \"a \\\" merge(\" ; sort(x)";
        assert_eq!(functions_in(code), vec!["sort"]);
        assert!(functions_in("").is_empty());
        assert!(functions_in("'unterminated merge(").is_empty());
    }

    #[test]
    fn choices_are_listed_with_the_hint() {
        let opts = [
            Opt {
                value: "a",
                label: "Alpha",
            },
            Opt {
                value: "b",
                label: "Beta",
            },
        ];
        assert_eq!(option_text("Pick.", &opts), "Pick. Choices: Alpha; Beta.");
        assert_eq!(option_text("", &opts), "Choices: Alpha; Beta.");
        assert_eq!(option_text("Just this.", &[]), "Just this.");
    }

    #[test]
    fn a_page_is_made_from_the_dialog_itself() {
        use crate::dialog::{no_check, Args};
        const SPEC: Spec = Spec {
            entry: "Mean of a variable",
            menu: "Data > Mean",
            about: "Averages one number variable.",
            widgets: &[
                Widget::Variable {
                    id: "v",
                    label: "Variable",
                    kinds: &[VarKind::Number],
                    required: true,
                    hint: "",
                },
                Widget::Choice {
                    id: "how",
                    label: "Kind",
                    options: &[
                        Opt {
                            value: "mean",
                            label: "Mean",
                        },
                        Opt {
                            value: "median",
                            label: "Median",
                        },
                    ],
                    default: "mean",
                    hint: "Which average.",
                },
            ],
            needs: &[],
            styles: &[Style::Base],
            check: no_check,
            code: |a: &Args, _| format!("stats::{}({})", a.choice("how"), a.var_text("v")),
        };
        let page = for_spec(&SPEC);
        assert_eq!(page.title, "Mean of a variable");
        assert_eq!(page.options.len(), 2);
        assert_eq!(page.options[0].text, "One variable (number).");
        assert_eq!(
            page.options[1].text,
            "Which average. Choices: Mean; Median."
        );
        assert_eq!(page.example.as_deref(), Some("stats::mean(x)"));
        assert_eq!(page.functions, vec!["stats::mean"]);
        assert!(page.needs.is_empty() && page.other_style.is_none());
    }

    #[test]
    fn the_two_hand_made_pages_have_what_a_page_needs() {
        for p in [use_my_file(), packages_window()] {
            assert!(!p.about.is_empty() && !p.notes.is_empty() && !p.functions.is_empty());
            assert!(p.example.is_some());
        }
    }
}
