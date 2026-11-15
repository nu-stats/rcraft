//! Reports (Phase 7): a table of models side by side (P7.1), in base R so it
//! needs no package. Each coefficient with its stars, and below it what the
//! user chose in parentheses; then the rows that describe each fit. The value
//! is a data frame, so Results shows it as a grid that copies into Word or
//! Excel, and it can be kept under a name.

use rcraft_rcodegen::dialog::{Args, Opt, Problem, Spec, TextDefault, TextRule, Widget};
use rcraft_rcodegen::r_name;

use crate::data::BASE;

/// The names typed in the Other models box, separated by commas.
fn other_models<'a>(a: &'a Args) -> Vec<&'a str> {
    a.values
        .text("others")
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .collect()
}

const PARENS: &[Opt] = &[
    Opt {
        value: "se",
        label: "Standard errors",
    },
    Opt {
        value: "statistic",
        label: "t or z statistics",
    },
    Opt {
        value: "p",
        label: "p-values",
    },
];

/// Most decimal places offered.
const MAX_DIGITS: u32 = 10;

fn table_check(a: &Args) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut seen = vec![a.ctx.dataset.as_str()];
    for name in other_models(a) {
        if seen.contains(&name) {
            out.push(Problem::new(
                "others",
                format!("{name} is listed more than once, or is this model."),
            ));
            break;
        }
        seen.push(name);
    }
    if !a
        .number("digits")
        .parse::<u32>()
        .is_ok_and(|d| d <= MAX_DIGITS)
    {
        out.push(Problem::new(
            "digits",
            format!("Type a whole number of decimal places, 0 to {MAX_DIGITS}."),
        ));
    }
    out
}

fn table_code(a: &Args, _: rcraft_rcodegen::dialog::Style) -> String {
    let mut names = vec![a.ctx.dataset.as_str()];
    names.extend(other_models(a));
    let list: Vec<String> = names
        .iter()
        .map(|n| format!("{} = {}", r_name(n), r_name(n)))
        .collect();
    let (column, words) = match a.choice("paren") {
        "statistic" => (3, "t or z statistics"),
        "p" => (4, "p-values"),
        _ => (2, "standard errors"),
    };
    let stars = a.flag("stars");
    let mut note = format!("{} in parentheses", capitalised(words));
    if stars {
        note.push_str("; * p < 0.05, ** p < 0.01, *** p < 0.001");
    }
    let star = if stars {
        "ifelse(p < 0.001, \"***\", ifelse(p < 0.01, \"**\", ifelse(p < 0.05, \"*\", \"\")))"
    } else {
        "rep(\"\", length(p))"
    };
    let mut stats: Vec<&str> = Vec::new();
    if a.flag("n") {
        stats.push(
            "    Observations = vapply(models, function(m) as.character(stats::nobs(m)), \"\")",
        );
    }
    if a.flag("r2") {
        stats.push(
            "    `R-squared` = vapply(models, function(m) \
             if (inherits(m, \"glm\")) \"\" else fmt(summary(m)$r.squared), \"\")",
        );
        stats.push(
            "    `Adjusted R-squared` = vapply(models, function(m) \
             if (inherits(m, \"glm\")) \"\" else fmt(summary(m)$adj.r.squared), \"\")",
        );
    }
    if a.flag("ic") {
        stats.push("    AIC = vapply(models, function(m) fmt(stats::AIC(m)), \"\")");
        stats.push("    BIC = vapply(models, function(m) fmt(stats::BIC(m)), \"\")");
    }
    if a.flag("ll") {
        stats.push(
            "    `Log-likelihood` = vapply(models, function(m) fmt(as.numeric(stats::logLik(m))), \"\")",
        );
    }
    let foot = if stats.is_empty() {
        String::new()
    } else {
        format!(
            "  extra <- list(\n{}\n  )\n  \
             fit <- data.frame(term = names(extra), do.call(rbind, extra), check.names = FALSE, \
             row.names = NULL)\n  \
             names(fit) <- names(out)\n  \
             out <- rbind(out, fit)\n",
            stats.join(",\n")
        )
    };
    let call = format!(
        "(function(models, digits) {{\n  \
         ok <- vapply(models, inherits, logical(1), \"lm\")\n  \
         if (!all(ok)) stop(\"the table takes linear and generalised linear models; not: \", \
         paste(names(models)[!ok], collapse = \", \"))\n  \
         fmt <- function(x) formatC(x, format = \"f\", digits = digits)\n  \
         terms <- unique(unlist(lapply(models, function(m) names(stats::coef(m)))))\n  \
         cells <- vapply(models, function(m) {{\n    \
         ct <- stats::coef(summary(m))\n    \
         p <- ct[, 4]\n    \
         out <- rep(\"\", 2 * length(terms))\n    \
         i <- match(rownames(ct), terms)\n    \
         out[2 * i - 1] <- paste0(fmt(ct[, 1]), {star})\n    \
         out[2 * i] <- paste0(\"(\", fmt(ct[, {column}]), \")\")\n    \
         out\n  \
         }}, character(2 * length(terms)))\n  \
         out <- data.frame(term = as.vector(rbind(terms, \"\")), \
         matrix(cells, ncol = length(models), dimnames = list(NULL, names(models))), \
         check.names = FALSE, row.names = NULL)\n\
         {foot}  \
         cat(\"{note}\\n\")\n  \
         attr(out, \"note\") <- \"{note}\"\n  \
         out\n}})(list({}), {})",
        list.join(", "),
        a.number("digits"),
    );
    if a.name_text("save").is_empty() {
        call
    } else {
        let save = a.name("save");
        format!("{save} <- {call}\n{save}")
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

pub const MODEL_TABLE: Spec = Spec {
    entry: "Model table",
    menu: "Objects > Model > Model table",
    about: "Put this model and others side by side in one table: each coefficient with its \
            stars and, below it, its standard error (or statistic or p-value), then the number \
            of rows, R-squared, AIC and BIC. The table copies into Word or Excel from Results. \
            For linear and generalised linear models.",
    widgets: &[
        Widget::Text {
            id: "others",
            label: "Other models",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: names of models in the workspace, separated by commas. This model \
                   is the first column.",
        },
        Widget::Choice {
            id: "paren",
            label: "In parentheses",
            options: PARENS,
            default: "se",
            hint: "",
        },
        Widget::Check {
            id: "stars",
            label: "Stars for p < 0.05, 0.01 and 0.001",
            default: true,
            hint: "",
        },
        Widget::Text {
            id: "digits",
            label: "Decimal places",
            default: TextDefault::Fixed("3"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Check {
            id: "n",
            label: "Number of observations",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "r2",
            label: "R-squared and adjusted R-squared (linear models)",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "ic",
            label: "AIC and BIC",
            default: true,
            hint: "",
        },
        Widget::Check {
            id: "ll",
            label: "Log-likelihood",
            default: false,
            hint: "",
        },
        Widget::Text {
            id: "save",
            label: "Also save the table as",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: a name for the table, so later commands can use it.",
        },
    ],
    needs: &[],
    styles: BASE,
    check: table_check,
    code: table_code,
};

/// The Reports dialogs that work on a fitted model; they are offered in the
/// Objects pane's Model menu with the post-estimation ones.
pub const MODEL_REPORTS: &[&Spec] = &[&MODEL_TABLE];

// ---- Export table (P7.2) ----

/// The kinds of file a table is written as, by the file name's ending.
const TABLE_FILES: &[(&str, &str)] = &[
    (".html", "a web page"),
    (".htm", "a web page"),
    (".docx", "a Word document"),
    (".rtf", "a Word document (RTF, which Word opens)"),
    (".tex", "LaTeX"),
    (".xlsx", "an Excel workbook"),
    (".md", "Markdown"),
];

fn table_file_kind(path: &str) -> Option<&'static str> {
    let lower = path.to_ascii_lowercase();
    TABLE_FILES
        .iter()
        .find(|(end, _)| lower.ends_with(end))
        .map(|(end, _)| *end)
}

fn export_check(a: &Args) -> Vec<Problem> {
    let file = a.name_text("file");
    if file.is_empty() {
        vec![Problem::new("file", "Type where to write the table.")]
    } else if table_file_kind(file).is_none() {
        vec![Problem::new(
            "file",
            "End the file name in .docx (Word), .html, .rtf, .tex, .xlsx or .md.",
        )]
    } else {
        Vec::new()
    }
}

fn export_code(a: &Args, _: rcraft_rcodegen::dialog::Style) -> String {
    let path_text = a.name_text("file").replace('\\', "/");
    let path = rcraft_rcodegen::r_string(&path_text);
    let title = a.values.text("title").trim().to_string();
    let guard = if a.flag("replace") {
        String::new()
    } else {
        format!(
            "  if (file.exists(path)) stop({})\n",
            rcraft_rcodegen::r_string("the file already exists; tick Replace to write over it")
        )
    };
    let write = match table_file_kind(&path_text).unwrap_or(".html") {
        ".xlsx" => {
            let head = if title.is_empty() {
                String::new()
            } else {
                format!(
                    "  wb$add_data(x = {}, start_row = 1, col_names = FALSE)\n",
                    rcraft_rcodegen::r_string(&title)
                )
            };
            let first = if title.is_empty() { 1 } else { 3 };
            format!(
                "  wb <- openxlsx2::wb_workbook()\n  wb$add_worksheet(\"Table\")\n{head}  \
                 wb$add_data(x = tab, start_row = {first})\n  \
                 if (!is.null(note)) wb$add_data(x = note, start_row = {first} + nrow(tab) + 2, \
                 col_names = FALSE)\n  \
                 wb$save(path)\n"
            )
        }
        ".docx" => {
            let head = if title.is_empty() {
                String::new()
            } else {
                format!(
                    "  ft <- flextable::add_header_lines(ft, {})
",
                    rcraft_rcodegen::r_string(&title)
                )
            };
            format!(
                "  ft <- flextable::flextable(tab)
{head}                   if (!is.null(note)) ft <- flextable::add_footer_lines(ft, note)
                   ft <- flextable::autofit(ft)
                   flextable::save_as_docx(ft, path = path)
"
            )
        }
        ".md" => {
            let head = if title.is_empty() {
                String::new()
            } else {
                format!("{}, \"\", ", rcraft_rcodegen::r_string(&title))
            };
            format!(
                "  cell <- function(x) gsub(\"|\", \"\\\\|\", ifelse(is.na(x), \"\", as.character(x)), fixed = TRUE)\n  \
                 row <- function(x) paste0(\"| \", paste(cell(x), collapse = \" | \"), \" |\")\n  \
                 body <- vapply(seq_len(nrow(tab)), function(i) row(unlist(tab[i, ], use.names = FALSE)), \"\")\n  \
                 writeLines(c({head}row(names(tab)), paste0(\"|\", strrep(\"---|\", ncol(tab))), body, \
                 if (!is.null(note)) c(\"\", note)), path)\n"
            )
        }
        _ => {
            let head = if title.is_empty() {
                String::new()
            } else {
                format!(
                    "  g <- gt::tab_header(g, title = {})\n",
                    rcraft_rcodegen::r_string(&title)
                )
            };
            format!(
                "  g <- gt::gt(tab)\n{head}  \
                 if (!is.null(note)) g <- gt::tab_source_note(g, note)\n  \
                 gt::gtsave(g, path)\n"
            )
        }
    };
    format!(
        "local({{\n  tab <- {}\n  \
         stopifnot(\"only a data frame can be written as a table\" = is.data.frame(tab))\n  \
         path <- {path}\n{guard}  \
         note <- attr(tab, \"note\")\n{write}  \
         if (!isTRUE(file.size(path) > 0)) stop(\"R wrote no file; check that the folder exists and can be written to\")\n  \
         cat(\"Wrote \", path, \"\\n\", sep = \"\")\n}})",
        a.data()
    )
}

pub const EXPORT_TABLE: Spec = Spec {
    entry: "Export table",
    menu: "Objects > Export table",
    about: "Write this data frame (a Model table, say) to a file as a formatted table: a web page \
            (.html), a Word document (.docx, or .rtf), LaTeX (.tex), an Excel workbook (.xlsx) \
            or Markdown (.md), chosen by the file name's ending. A Model table's note goes \
            below it.",
    widgets: &[
        Widget::Text {
            id: "file",
            label: "File",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Its ending chooses the kind: .docx, .html, .rtf, .tex, .xlsx or .md.",
        },
        Widget::Text {
            id: "title",
            label: "Title",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: a line above the table.",
        },
        Widget::Check {
            id: "replace",
            label: "Replace the file if it is already there",
            default: false,
            hint: "Left unticked, nothing is written when the file exists.",
        },
    ],
    needs: &["gt", "openxlsx2", "flextable", "officer"],
    styles: BASE,
    check: export_check,
    code: export_code,
};

/// The Reports dialogs that work on a data frame in the Objects pane.
pub const TABLE_REPORTS: &[&Spec] = &[&EXPORT_TABLE, &ADD_TABLE];

// ---- The session report (P7.2): tables, plots and text gathered into one document ----

/// R that gives the report so far (an empty list if there is none yet).
const REPORT_SO_FAR: &str =
    "if (exists(\"rcraft_report\", envir = globalenv(), inherits = FALSE)) \
     get(\"rcraft_report\", envir = globalenv()) else list()";

/// R that adds `part` (an R expression for a list) to the report and says how
/// many parts it has.
fn add_part(part: &str) -> String {
    format!(
        "assign(\"rcraft_report\", c({REPORT_SO_FAR}, list({part})), envir = globalenv())\n\
         cat(\"The report has \", length(rcraft_report), \" part\", \
         if (length(rcraft_report) == 1) \"\" else \"s\", \". Reports > Save report writes it.\\n\", sep = \"\")"
    )
}

fn part_title(a: &Args) -> String {
    rcraft_rcodegen::r_string(a.values.text("title").trim())
}

const PART_TITLE: Widget = Widget::Text {
    id: "title",
    label: "Heading",
    default: TextDefault::Empty,
    rule: TextRule::Any,
    hint: "Optional: a heading above it in the report.",
};

pub const ADD_TABLE: Spec = Spec {
    entry: "Add table to report",
    menu: "Objects > Add table to report",
    about: "Add this data frame (a Model table, say) to the session's report, as it is now; \
            Reports > Save report writes the report as a Word document or a web page.",
    widgets: &[PART_TITLE],
    needs: &[],
    styles: BASE,
    check: |_| Vec::new(),
    code: |a, _| {
        format!(
            "stopifnot(\"only a data frame can go in the report as a table\" = is.data.frame({d}))\n{}",
            add_part(&format!(
                "list(kind = \"table\", title = {}, value = {d})",
                part_title(a),
                d = a.data()
            )),
            d = a.data()
        )
    },
};

pub const ADD_PLOT: Spec = Spec {
    entry: "Add plot to report",
    menu: "Reports > Add plot to report",
    about: "Add a plot to the session's report: the newest one, or one named in the Graph tab. \
            It is drawn again at the size given when the report is saved.",
    widgets: &[
        Widget::Text {
            id: "plot",
            label: "Plot",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Empty for the newest plot, or its name in the Graph tab.",
        },
        PART_TITLE,
        Widget::Text {
            id: "width",
            label: "Width (inches)",
            default: TextDefault::Fixed("6"),
            rule: TextRule::Number,
            hint: "",
        },
        Widget::Text {
            id: "height",
            label: "Height (inches)",
            default: TextDefault::Fixed("4"),
            rule: TextRule::Number,
            hint: "",
        },
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        let ok = |id: &str| {
            a.number(id)
                .parse::<f64>()
                .is_ok_and(|v| (0.5..=20.0).contains(&v))
        };
        let mut out = Vec::new();
        for id in ["width", "height"] {
            if !ok(id) {
                out.push(Problem::new(id, "Type a size from 0.5 to 20 inches."));
            }
        }
        out
    },
    code: |a, _| {
        let lookup = if a.name_text("plot").is_empty() {
            "if (length(plots)) plots[[length(plots)]]".to_string()
        } else {
            format!(
                "plots[[{}]]",
                rcraft_rcodegen::r_string(a.name_text("plot"))
            )
        };
        format!(
            "local({{\n  plots <- getOption(\"rcraft.plots\")\n  rec <- {lookup}\n  \
             if (is.null(rec)) stop(\"that plot is no longer kept; draw it again, then add it\")\n  \
             {}\n}})",
            add_part(&format!(
                "list(kind = \"plot\", title = {}, value = rec, width = {}, height = {})",
                part_title(a),
                a.number("width"),
                a.number("height")
            ))
            .replace('\n', "\n  ")
        )
    },
};

pub const ADD_TEXT: Spec = Spec {
    entry: "Add text to report",
    menu: "Reports > Add text to report",
    about: "Add a paragraph of text, under an optional heading, to the session's report.",
    widgets: &[
        PART_TITLE,
        Widget::Text {
            id: "text",
            label: "Text",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "",
        },
    ],
    needs: &[],
    styles: BASE,
    check: |a| {
        if a.values.text("text").trim().is_empty() && a.values.text("title").trim().is_empty() {
            vec![Problem::new(
                "text",
                "Type the text, or at least a heading.",
            )]
        } else {
            Vec::new()
        }
    },
    code: |a, _| {
        add_part(&format!(
            "list(kind = \"text\", title = {}, value = {})",
            part_title(a),
            rcraft_rcodegen::r_string(a.values.text("text").trim())
        ))
    },
};

pub const REPORT_CONTENTS: Spec = Spec {
    entry: "Report contents",
    menu: "Reports > Report contents",
    about: "List what is in the session's report so far, in order.",
    widgets: &[],
    needs: &[],
    styles: BASE,
    check: |_| Vec::new(),
    code: |_, _| {
        format!(
            "local({{\n  rep <- {REPORT_SO_FAR}\n  \
             if (!length(rep)) cat(\"The report is empty.\\n\") else \
             print(data.frame(part = seq_along(rep), kind = vapply(rep, `[[`, \"\", \"kind\"), \
             heading = vapply(rep, `[[`, \"\", \"title\")), row.names = FALSE)\n}})"
        )
    },
};

pub const CLEAR_REPORT: Spec = Spec {
    entry: "Clear report",
    menu: "Reports > Clear report",
    about: "Start the session's report again, empty. Files already saved are not touched.",
    widgets: &[],
    needs: &[],
    styles: BASE,
    check: |_| Vec::new(),
    code: |_, _| {
        "if (exists(\"rcraft_report\", envir = globalenv(), inherits = FALSE)) \
         rm(\"rcraft_report\", envir = globalenv())\ncat(\"The report is empty.\\n\")"
            .to_string()
    },
};

fn save_report_check(a: &Args) -> Vec<Problem> {
    let file = a.name_text("file").to_ascii_lowercase();
    if file.is_empty() {
        vec![Problem::new("file", "Type where to write the report.")]
    } else if !(file.ends_with(".docx") || file.ends_with(".html") || file.ends_with(".htm")) {
        vec![Problem::new(
            "file",
            "End the file name in .docx (Word) or .html.",
        )]
    } else {
        Vec::new()
    }
}

fn save_report_code(a: &Args, _: rcraft_rcodegen::dialog::Style) -> String {
    let path_text = a.name_text("file").replace('\\', "/");
    let path = rcraft_rcodegen::r_string(&path_text);
    let title = a.values.text("title").trim().to_string();
    let guard = if a.flag("replace") {
        String::new()
    } else {
        format!(
            "  if (file.exists(path)) stop({})\n",
            rcraft_rcodegen::r_string("the file already exists; tick Replace to write over it")
        )
    };
    let draw = "    png_of <- function(p, f) {\n      before <- grDevices::dev.cur()\n      \
                grDevices::png(f, width = p$width, height = p$height, units = \"in\", res = 150)\n      \
                opened <- grDevices::dev.cur()\n      \
                if (opened == before) stop(\"R could not open a file to draw the plot in\")\n      \
                on.exit(grDevices::dev.off(opened))\n      grDevices::replayPlot(p$value)\n    }\n";
    let body = if path_text.to_ascii_lowercase().ends_with(".docx") {
        let head = if title.is_empty() {
            String::new()
        } else {
            format!(
                "  doc <- officer::body_add_par(doc, {}, style = \"heading 1\")\n",
                rcraft_rcodegen::r_string(&title)
            )
        };
        format!(
            "  doc <- officer::read_docx()\n{head}{draw}  \
             for (p in rep) {{\n    \
             if (nzchar(p$title)) doc <- officer::body_add_par(doc, p$title, style = \"heading 2\")\n    \
             if (p$kind == \"text\" && nzchar(p$value)) doc <- officer::body_add_par(doc, p$value, style = \"Normal\")\n    \
             if (p$kind == \"table\") {{\n      ft <- flextable::autofit(flextable::flextable(p$value))\n      \
             if (!is.null(attr(p$value, \"note\"))) ft <- flextable::add_footer_lines(ft, attr(p$value, \"note\"))\n      \
             doc <- flextable::body_add_flextable(doc, ft)\n    }}\n    \
             if (p$kind == \"plot\") {{\n      f <- tempfile(fileext = \".png\")\n      png_of(p, f)\n      \
             doc <- officer::body_add_img(doc, f, width = p$width, height = p$height)\n    }}\n  }}\n  \
             print(doc, target = path)\n"
        )
    } else {
        let head = if title.is_empty() {
            String::new()
        } else {
            format!(
                "  out <- c(out, paste0(\"<h1>\", esc({}), \"</h1>\"))\n",
                rcraft_rcodegen::r_string(&title)
            )
        };
        format!(
            "  esc <- function(x) {{ x <- gsub(\"&\", \"&amp;\", x, fixed = TRUE); \
             x <- gsub(\"<\", \"&lt;\", x, fixed = TRUE); gsub(\">\", \"&gt;\", x, fixed = TRUE) }}\n  \
             images <- paste0(tools::file_path_sans_ext(path), \"_files\")\n  \
             out <- c(\"<!DOCTYPE html>\", \"<html><head><meta charset=\\\"utf-8\\\"><title>\", \
             esc({}), \"</title></head><body>\")\n{head}{draw}  \
             n <- 0\n  \
             for (p in rep) {{\n    \
             if (nzchar(p$title)) out <- c(out, paste0(\"<h2>\", esc(p$title), \"</h2>\"))\n    \
             if (p$kind == \"text\" && nzchar(p$value)) out <- c(out, paste0(\"<p>\", esc(p$value), \"</p>\"))\n    \
             if (p$kind == \"table\") {{\n      g <- gt::gt(p$value)\n      \
             if (!is.null(attr(p$value, \"note\"))) g <- gt::tab_source_note(g, attr(p$value, \"note\"))\n      \
             out <- c(out, as.character(gt::as_raw_html(g)))\n    }}\n    \
             if (p$kind == \"plot\") {{\n      n <- n + 1\n      dir.create(images, showWarnings = FALSE)\n      \
             f <- file.path(images, sprintf(\"plot%d.png\", n))\n      png_of(p, f)\n      \
             out <- c(out, sprintf(\"<p><img src=\\\"%s/%s\\\" width=\\\"%d\\\"></p>\", \
             basename(images), basename(f), round(p$width * 96)))\n    }}\n  }}\n  \
             writeLines(c(out, \"</body></html>\"), path, useBytes = TRUE)\n",
            rcraft_rcodegen::r_string(if title.is_empty() { "Report" } else { &title })
        )
    };
    format!(
        "local({{\n  rep <- {REPORT_SO_FAR}\n  \
         if (!length(rep)) stop(\"the report is empty; add a table, a plot or text first\")\n  \
         path <- {path}\n{guard}{body}  \
         if (!isTRUE(file.size(path) > 0)) stop(\"R wrote no file; check that the folder exists and can be written to\")\n  \
         cat(\"Wrote the report (\", length(rep), \" part\", if (length(rep) == 1) \"\" else \"s\", \") to \", path, \"\\n\", sep = \"\")\n}})"
    )
}

pub const SAVE_REPORT: Spec = Spec {
    entry: "Save report",
    menu: "Reports > Save report",
    about: "Write the session's report, its tables, plots and text in the order they were \
            added, as a Word document (.docx) or a web page (.html, its plots in a folder beside \
            it).",
    widgets: &[
        Widget::Text {
            id: "file",
            label: "File",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "End it in .docx or .html.",
        },
        Widget::Text {
            id: "title",
            label: "Title",
            default: TextDefault::Empty,
            rule: TextRule::Any,
            hint: "Optional: a title at the top.",
        },
        Widget::Check {
            id: "replace",
            label: "Replace the file if it is already there",
            default: false,
            hint: "",
        },
    ],
    needs: &["gt", "flextable", "officer"],
    styles: BASE,
    check: save_report_check,
    code: save_report_code,
};

/// The Reports menu's dialogs, in menu order. They need no data frame.
pub const REPORT_SPECS: &[&Spec] = &[
    &ADD_TEXT,
    &ADD_PLOT,
    &REPORT_CONTENTS,
    &SAVE_REPORT,
    &CLEAR_REPORT,
];

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rcodegen::dialog::{Context, Style, Values};

    fn make(
        model: &str,
        set: &[(&str, &str)],
        flags: &[(&str, bool)],
    ) -> Result<String, Vec<Problem>> {
        let c = Context {
            dataset: model.into(),
            frames: Vec::new(),
            columns: Vec::new(),
        };
        let mut v: Values = MODEL_TABLE.defaults(&c);
        for (id, text) in set {
            v.set_text(id, *text);
        }
        for (id, on) in flags {
            v.set_flag(id, *on);
        }
        MODEL_TABLE.generate(&c, &v, Style::Base)
    }

    #[test]
    fn one_model_or_several_with_the_choices_made() {
        let one = make("fit", &[], &[]).unwrap_or_default();
        assert!(one.ends_with("})(list(fit = fit), 3)"), "{one}");
        assert!(one.contains("fmt(ct[, 2])"), "{one}");
        assert!(
            one.contains("Standard errors in parentheses; * p < 0.05"),
            "{one}"
        );
        assert!(
            one.contains("Observations = ") && one.contains("AIC = "),
            "{one}"
        );
        assert!(!one.contains("Log-likelihood"), "{one}");

        let two = make(
            "my fit",
            &[
                ("others", "m2, m3"),
                ("paren", "p"),
                ("digits", "2"),
                ("save", "tab"),
            ],
            &[
                ("stars", false),
                ("n", false),
                ("r2", false),
                ("ic", false),
                ("ll", true),
            ],
        )
        .unwrap_or_default();
        assert!(two.starts_with("tab <- (function(models, digits)"), "{two}");
        assert!(
            two.contains("})(list(`my fit` = `my fit`, m2 = m2, m3 = m3), 2)\ntab"),
            "{two}"
        );
        assert!(
            two.contains("fmt(ct[, 4])") && two.contains("P-values in parentheses\\n"),
            "{two}"
        );
        assert!(!two.contains("***") && !two.contains("AIC ="), "{two}");
        assert!(two.contains("`Log-likelihood` = "), "{two}");

        // nothing below the coefficients: no rows to add
        let bare =
            make("fit", &[], &[("n", false), ("r2", false), ("ic", false)]).unwrap_or_default();
        assert!(!bare.contains("extra <- list("), "{bare}");
    }

    #[test]
    fn a_table_is_written_as_its_file_ending_says() {
        let c = Context {
            dataset: "tab".into(),
            frames: Vec::new(),
            columns: Vec::new(),
        };
        let code = |file: &str, title: &str| {
            let mut v: Values = EXPORT_TABLE.defaults(&c);
            v.set_text("file", file);
            v.set_text("title", title);
            EXPORT_TABLE.generate(&c, &v, Style::Base)
        };
        let html = code(r"C:\out\t.HTML", "").unwrap_or_default();
        assert!(html.contains("path <- \"C:/out/t.HTML\""), "{html}");
        assert!(
            html.contains("gt::gtsave(g, path)") && html.contains("if (file.exists(path)) stop("),
            "{html}"
        );
        assert!(code("t.rtf", "Table 1")
            .unwrap_or_default()
            .contains("gt::tab_header(g, title = \"Table 1\")"));
        assert!(code("t.tex", "").unwrap_or_default().contains("gt::gtsave"));
        let xlsx = code("t.xlsx", "Table 1").unwrap_or_default();
        assert!(
            xlsx.contains("openxlsx2::wb_workbook()") && xlsx.contains("start_row = 3"),
            "{xlsx}"
        );
        let md = code("t.md", "").unwrap_or_default();
        assert!(md.contains("writeLines(c(row(names(tab))"), "{md}");
        let docx = code("t.docx", "Table 1").unwrap_or_default();
        assert!(
            docx.contains("flextable::save_as_docx(ft, path = path)")
                && docx.contains("flextable::add_header_lines(ft, \"Table 1\")"),
            "{docx}"
        );
        for bad in ["", "t.doc", "t.csv"] {
            assert!(code(bad, "").is_err(), "{bad}");
        }
    }

    #[test]
    fn names_listed_twice_and_odd_decimal_places_are_refused() {
        let problems = |set: &[(&str, &str)]| match make("fit", set, &[]) {
            Ok(_) => Vec::new(),
            Err(p) => p.into_iter().map(|p| p.message).collect(),
        };
        assert!(problems(&[("others", "fit")])[0].contains("listed more than once"));
        assert!(problems(&[("others", "a, a")])[0].contains("listed more than once"));
        assert!(problems(&[("digits", "11")])[0].contains("decimal places"));
        assert!(problems(&[("digits", "1.5")])[0].contains("decimal places"));
        assert!(problems(&[("others", "a, b"), ("digits", "0")]).is_empty());
    }
}
