//! File menu dialogs for saving data (P3.7): the data frame shown in
//! Variables as text, an Excel workbook, a Stata, SPSS or SAS transport file,
//! an R data file or a Parquet or Feather file, and the whole workspace.
//! Text and R data need no package; the others name the one they need in
//! `Spec::needs`, which the package panel reads. Nothing is ever overwritten
//! unless the box is ticked: the code stops with a message first.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, Style, TextDefault, TextRule, Widget};
use rcraft_rcodegen::r_string;

const BASE: &[Style] = &[Style::Base];

const FILE: Widget = Widget::Text {
    id: "file",
    label: "Save to",
    default: TextDefault::Empty,
    rule: TextRule::SavePath,
    hint: "The full path of the file, with its extension.",
};

const REPLACE: Widget = Widget::Check {
    id: "replace",
    label: "Replace the file if it is already there",
    default: false,
    hint: "Left unticked, nothing is written when the file exists.",
};

/// The file's path as an R string, with forward slashes (AGENTS.md section 4).
fn path(a: &Args) -> String {
    r_string(&a.name_text("file").replace('\\', "/"))
}

/// Stops R before it writes over a file, unless the box is ticked.
fn guard(a: &Args) -> String {
    if a.flag("replace") {
        String::new()
    } else {
        format!(
            "if (file.exists({})) stop({})\n",
            path(a),
            r_string("the file already exists; tick Replace to write over it")
        )
    }
}

/// `.csv`, `.tsv` or `.txt`-style endings, as a message.
fn ending_text(endings: &[&str]) -> String {
    match endings.split_last() {
        Some((last, [])) => format!(".{last}"),
        Some((last, rest)) => format!(
            "{} or .{last}",
            rest.iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        None => String::new(),
    }
}

/// The file name must end in one of `endings` (without the dot, any case).
fn ending_check(a: &Args, endings: &[&str]) -> Vec<Problem> {
    let file = a.name_text("file").to_ascii_lowercase();
    if endings.iter().any(|e| file.ends_with(&format!(".{e}"))) {
        Vec::new()
    } else {
        vec![Problem::new(
            "file",
            format!("The file name should end in {}.", ending_text(endings)),
        )]
    }
}

// ---- Text ----

pub const SAVE_TEXT: Spec = Spec {
    entry: "Text file",
    menu: "File > Save data as > Text file",
    about: "Save the data frame as a text file that a spreadsheet or any other program can \
            read: one line for each row, a header line of variable names, UTF-8.",
    widgets: &[
        FILE,
        Widget::Choice {
            id: "sep",
            label: "Columns are separated by",
            options: &[
                Opt {
                    value: "comma",
                    label: "Commas (.csv)",
                },
                Opt {
                    value: "tab",
                    label: "Tabs (.tsv or .txt)",
                },
                Opt {
                    value: "semicolon",
                    label: "Semicolons (.csv, as some countries' spreadsheets use)",
                },
            ],
            default: "comma",
            hint: "",
        },
        Widget::Choice {
            id: "na",
            label: "Missing values are written as",
            options: &[
                Opt {
                    value: "NA",
                    label: "NA",
                },
                Opt {
                    value: "empty",
                    label: "Nothing (an empty cell)",
                },
            ],
            default: "NA",
            hint: "",
        },
        Widget::Check {
            id: "rownames",
            label: "Also write the row names",
            default: false,
            hint: "",
        },
        REPLACE,
    ],
    needs: &[],
    styles: BASE,
    check: |a| ending_check(a, &["csv", "tsv", "txt"]),
    code: text_code,
};

fn text_code(a: &Args, _: Style) -> String {
    // the tab is written as R's own escape, not as a backslash and a t
    let sep = match a.choice("sep") {
        "tab" => "\"\\t\"".to_string(),
        "semicolon" => r_string(";"),
        _ => r_string(","),
    };
    let na = if a.choice("na") == "empty" { "" } else { "NA" };
    // a header with a blank first cell keeps the names over their columns
    let names = if a.flag("rownames") {
        "row.names = TRUE, col.names = NA"
    } else {
        "row.names = FALSE"
    };
    format!(
        "{}write.table({}, file = {}, sep = {}, {names}, na = {}, qmethod = \"double\", \
         fileEncoding = \"UTF-8\")",
        guard(a),
        a.data(),
        path(a),
        sep,
        r_string(na),
    )
}

// ---- Excel ----

pub const SAVE_EXCEL: Spec = Spec {
    entry: "Excel workbook",
    menu: "File > Save data as > Excel workbook",
    about: "Save the data frame as an .xlsx workbook with one sheet. Dates and numbers stay \
            dates and numbers. Needs the writexl package.",
    widgets: &[
        FILE,
        Widget::Text {
            id: "sheet",
            label: "Sheet name",
            default: TextDefault::Fixed("Sheet1"),
            rule: TextRule::Name,
            hint: "Up to 31 characters, none of  [ ] : * ? / \\",
        },
        REPLACE,
    ],
    needs: &["writexl"],
    styles: BASE,
    check: excel_check,
    code: excel_code,
};

fn excel_check(a: &Args) -> Vec<Problem> {
    let mut out = ending_check(a, &["xlsx"]);
    let sheet = a.name_text("sheet");
    if sheet.chars().count() > 31 || sheet.contains(['[', ']', ':', '*', '?', '/', '\\']) {
        out.push(Problem::new(
            "sheet",
            "A sheet name has at most 31 characters and none of [ ] : * ? / \\",
        ));
    }
    out
}

fn excel_code(a: &Args, _: Style) -> String {
    format!(
        "{}writexl::write_xlsx(list({} = {}), path = {})",
        guard(a),
        a.name_string("sheet"),
        a.data(),
        path(a),
    )
}

// ---- Stata, SPSS, SAS ----

pub const SAVE_STATISTICS: Spec = Spec {
    entry: "Stata, SPSS or SAS",
    menu: "File > Save data as > Stata, SPSS or SAS",
    about: "Save the data frame for another statistics program, keeping variable labels and \
            value labels. These formats have no missing text, so a missing text value is \
            saved as empty text. Stata, for one, needs variable names of letters, digits \
            and underscores only; R tells you if one is not. Needs the haven package.",
    widgets: &[
        FILE,
        Widget::Choice {
            id: "format",
            label: "Format",
            options: &[
                Opt {
                    value: "dta",
                    label: "Stata (.dta)",
                },
                Opt {
                    value: "sav",
                    label: "SPSS (.sav)",
                },
                Opt {
                    value: "zsav",
                    label: "SPSS, compressed (.zsav)",
                },
                Opt {
                    value: "xpt",
                    label: "SAS transport (.xpt)",
                },
            ],
            default: "dta",
            hint: "",
        },
        REPLACE,
    ],
    needs: &["haven"],
    styles: BASE,
    check: statistics_check,
    code: statistics_code,
};

fn statistics_check(a: &Args) -> Vec<Problem> {
    ending_check(a, &[a.choice("format")])
}

fn statistics_code(a: &Args, _: Style) -> String {
    let call = match a.choice("format") {
        "sav" => "haven::write_sav",
        "zsav" => "haven::write_sav",
        "xpt" => "haven::write_xpt",
        _ => "haven::write_dta",
    };
    let extra = if a.choice("format") == "zsav" {
        ", compress = \"zsav\""
    } else {
        ""
    };
    format!(
        "{}{call}({}, path = {}{extra})",
        guard(a),
        a.data(),
        path(a)
    )
}

// ---- R data ----

pub const SAVE_R: Spec = Spec {
    entry: "R data",
    menu: "File > Save data as > R data",
    about: "Save the data frame exactly as it is in R (types, labels, everything). An .rds \
            file holds the one data frame; an .RData file holds it under its name. Open \
            either again with Use my file.",
    widgets: &[
        FILE,
        Widget::Choice {
            id: "format",
            label: "Format",
            options: &[
                Opt {
                    value: "rds",
                    label: "One object (.rds)",
                },
                Opt {
                    value: "rdata",
                    label: "Named object (.RData or .rda)",
                },
            ],
            default: "rds",
            hint: "",
        },
        REPLACE,
    ],
    needs: &[],
    styles: BASE,
    check: r_check,
    code: r_code,
};

fn r_check(a: &Args) -> Vec<Problem> {
    if a.choice("format") == "rds" {
        ending_check(a, &["rds"])
    } else {
        ending_check(a, &["rdata", "rda"])
    }
}

fn r_code(a: &Args, _: Style) -> String {
    if a.choice("format") == "rds" {
        format!("{}saveRDS({}, file = {})", guard(a), a.data(), path(a))
    } else {
        format!(
            "{}save(list = {}, file = {})",
            guard(a),
            r_string(&a.ctx.dataset),
            path(a)
        )
    }
}

// ---- Parquet and Feather ----

pub const SAVE_ARROW: Spec = Spec {
    entry: "Parquet or Feather",
    menu: "File > Save data as > Parquet or Feather",
    about: "Save the data frame in a column-oriented file that is fast to read and small on \
            disk. Needs the arrow package.",
    widgets: &[
        FILE,
        Widget::Choice {
            id: "format",
            label: "Format",
            options: &[
                Opt {
                    value: "parquet",
                    label: "Parquet (.parquet)",
                },
                Opt {
                    value: "feather",
                    label: "Feather (.feather)",
                },
            ],
            default: "parquet",
            hint: "",
        },
        REPLACE,
    ],
    needs: &["arrow"],
    styles: BASE,
    check: arrow_check,
    code: arrow_code,
};

fn arrow_check(a: &Args) -> Vec<Problem> {
    ending_check(a, &[a.choice("format")])
}

fn arrow_code(a: &Args, _: Style) -> String {
    let call = if a.choice("format") == "feather" {
        "arrow::write_feather"
    } else {
        "arrow::write_parquet"
    };
    format!("{}{call}({}, sink = {})", guard(a), a.data(), path(a))
}

// ---- Workspace ----

pub const SAVE_WORKSPACE: Spec = Spec {
    entry: "Save workspace",
    menu: "File > Save workspace",
    about: "Save every object in R's workspace (data frames, models and the rest) in one \
            .RData file. Open it again with Use my file, or load() in the Command window.",
    widgets: &[FILE, REPLACE],
    needs: &[],
    styles: BASE,
    check: |a| ending_check(a, &["rdata", "rda"]),
    code: |a, _| format!("{}save.image(file = {})", guard(a), path(a)),
};

/// The File menu's save dialogs, in menu order.
pub const SPECS: &[&Spec] = &[
    &SAVE_TEXT,
    &SAVE_EXCEL,
    &SAVE_STATISTICS,
    &SAVE_R,
    &SAVE_ARROW,
    &SAVE_WORKSPACE,
];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{ColumnInfo, Context, VarKind};

    fn ctx() -> Context {
        Context {
            dataset: "my data".into(),
            frames: Vec::new(),
            columns: vec![ColumnInfo {
                name: "x".into(),
                kind: VarKind::Number,
            }],
        }
    }

    fn code(spec: &Spec, set: &[(&str, &str)], flags: &[(&str, bool)]) -> Result<String, String> {
        let c = ctx();
        let mut v = spec.defaults(&c);
        for (id, t) in set {
            v.set_text(id, *t);
        }
        for (id, on) in flags {
            v.set_flag(id, *on);
        }
        spec.generate(&c, &v, Style::Base).map_err(|p| {
            p.iter()
                .map(|p| p.message.clone())
                .collect::<Vec<_>>()
                .join(" / ")
        })
    }

    #[test]
    fn text_code_guards_the_file_and_names_the_separator() {
        let c = code(&SAVE_TEXT, &[("file", "C:\\out\\my.csv")], &[]);
        assert_eq!(
            c.as_deref(),
            Ok(
                "if (file.exists(\"C:/out/my.csv\")) stop(\"the file already exists; tick \
                Replace to write over it\")\nwrite.table(`my data`, file = \"C:/out/my.csv\", \
                sep = \",\", row.names = FALSE, na = \"NA\", qmethod = \"double\", \
                fileEncoding = \"UTF-8\")"
            )
        );
        let tab = code(
            &SAVE_TEXT,
            &[("file", "o.tsv"), ("sep", "tab"), ("na", "empty")],
            &[("replace", true), ("rownames", true)],
        )
        .unwrap_or_default();
        assert!(!tab.contains("file.exists"), "{tab}");
        assert!(
            tab.contains("sep = \"\\t\", row.names = TRUE, col.names = NA, na = \"\""),
            "{tab}"
        );
    }

    #[test]
    fn every_dialog_needs_a_path_with_the_right_ending() {
        for spec in SPECS {
            assert!(code(spec, &[], &[]).is_err(), "{}", spec.entry);
        }
        assert!(code(&SAVE_TEXT, &[("file", "a.xlsx")], &[]).is_err());
        assert!(code(&SAVE_EXCEL, &[("file", "a.CSV")], &[]).is_err());
        assert!(code(&SAVE_EXCEL, &[("file", "a.XLSX")], &[]).is_ok());
        assert!(code(&SAVE_WORKSPACE, &[("file", "w.rds")], &[]).is_err());
        assert!(code(&SAVE_WORKSPACE, &[("file", "w.RData")], &[]).is_ok());
        // the ending follows the chosen format
        assert!(code(&SAVE_STATISTICS, &[("file", "a.dta")], &[]).is_ok());
        assert!(code(
            &SAVE_STATISTICS,
            &[("file", "a.dta"), ("format", "sav")],
            &[]
        )
        .is_err());
        assert!(code(
            &SAVE_ARROW,
            &[("file", "a.feather"), ("format", "feather")],
            &[]
        )
        .is_ok());
        assert!(code(&SAVE_R, &[("file", "a.rds"), ("format", "rdata")], &[]).is_err());
    }

    #[test]
    fn each_format_calls_its_own_writer() {
        let one = |spec: &Spec, file: &str, format: &str| {
            code(
                spec,
                &[("file", file), ("format", format)],
                &[("replace", true)],
            )
            .unwrap_or_default()
        };
        assert_eq!(
            one(&SAVE_STATISTICS, "a.dta", "dta"),
            "haven::write_dta(`my data`, path = \"a.dta\")"
        );
        assert_eq!(
            one(&SAVE_STATISTICS, "a.zsav", "zsav"),
            "haven::write_sav(`my data`, path = \"a.zsav\", compress = \"zsav\")"
        );
        assert_eq!(
            one(&SAVE_STATISTICS, "a.xpt", "xpt"),
            "haven::write_xpt(`my data`, path = \"a.xpt\")"
        );
        assert_eq!(
            one(&SAVE_ARROW, "a.parquet", "parquet"),
            "arrow::write_parquet(`my data`, sink = \"a.parquet\")"
        );
        assert_eq!(
            one(&SAVE_R, "a.rds", "rds"),
            "saveRDS(`my data`, file = \"a.rds\")"
        );
        assert_eq!(
            one(&SAVE_R, "a.RData", "rdata"),
            "save(list = \"my data\", file = \"a.RData\")"
        );
        assert_eq!(
            code(
                &SAVE_WORKSPACE,
                &[("file", "w.RData")],
                &[("replace", true)]
            )
            .as_deref(),
            Ok("save.image(file = \"w.RData\")")
        );
    }

    #[test]
    fn excel_checks_the_sheet_name_and_names_its_package() {
        assert_eq!(SAVE_EXCEL.needs, &["writexl"]);
        assert_eq!(SAVE_STATISTICS.needs, &["haven"]);
        assert_eq!(SAVE_ARROW.needs, &["arrow"]);
        let ok = code(
            &SAVE_EXCEL,
            &[("file", "a.xlsx"), ("sheet", "Q1 data")],
            &[("replace", true)],
        );
        assert_eq!(
            ok.as_deref(),
            Ok("writexl::write_xlsx(list(\"Q1 data\" = `my data`), path = \"a.xlsx\")")
        );
        assert!(code(&SAVE_EXCEL, &[("file", "a.xlsx"), ("sheet", "a/b")], &[]).is_err());
        let long = "x".repeat(32);
        assert!(code(&SAVE_EXCEL, &[("file", "a.xlsx"), ("sheet", &long)], &[]).is_err());
    }

    #[test]
    fn a_path_with_a_quote_stays_inside_its_string() {
        let c = code(&SAVE_R, &[("file", "a\".rds")], &[("replace", true)]);
        assert_eq!(c.as_deref(), Ok("saveRDS(`my data`, file = \"a\\\".rds\")"));
        let c = code(
            &SAVE_TEXT,
            &[("file", "it's \"x\".csv")],
            &[("replace", true)],
        )
        .unwrap_or_default();
        assert!(c.contains("file = \"it's \\\"x\\\".csv\""), "{c}");
    }
}
