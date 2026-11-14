//! A table R printed: a command's visible value that is a data frame, matrix
//! or table (or a named list of them), sent so Results can show it as a grid.
//! R writes it to a file in the session's plot folder and sends
//! `<token>\tTABLE\t<id>\t<file name>`. The file holds R's own printed text
//! too, so nothing printed is lost when the grid is shown instead.
//!
//! ```text
//! RCT1
//! <title>                    (a list element's name; empty otherwise)
//! <corner>                   (the row dimension's name, if any)
//! <column>\t<column>...      (escaped)
//! <1|0>\t<1|0>...            (1: the column holds numbers)
//! <row name>\t<cell>\t<cell>...   (one line per row, escaped)
//! TEXT
//! <what R printed>           (escaped, one line)
//! ```
//!
//! Every field is escaped with [`crate::escape`]; cells are R's own formatted
//! values (`format()`), so numbers read as R prints them.

use crate::unescape;

/// Largest table file RCraft reads.
pub const MAX_TABLE_FILE_BYTES: u64 = 8 * 1024 * 1024;
/// Most rows and columns a table may have (R sends no larger ones).
pub const MAX_TABLE_ROWS: usize = 500;
pub const MAX_TABLE_COLUMNS: usize = 50;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResultTable {
    /// A list element's name, or empty.
    pub title: String,
    /// The name of what the rows are (a table's row variable), or empty.
    pub corner: String,
    pub columns: Vec<String>,
    /// Whether each column holds numbers (they are aligned to the right).
    pub numeric: Vec<bool>,
    /// Each row's name and its cells, one per column.
    pub rows: Vec<(String, Vec<String>)>,
}

impl ResultTable {
    /// Tab-separated text with a header line: pastes as cells into a
    /// spreadsheet or a word processor's table.
    pub fn tsv(&self) -> String {
        let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
        let mut out = clean(&self.corner);
        for c in &self.columns {
            out.push('\t');
            out.push_str(&clean(c));
        }
        out.push('\n');
        for (name, cells) in &self.rows {
            out.push_str(&clean(name));
            for c in cells {
                out.push('\t');
                out.push_str(&clean(c));
            }
            out.push('\n');
        }
        out
    }

    /// Comma-separated text, quoted where needed (RFC 4180), with a header line.
    pub fn csv(&self) -> String {
        let quote = |s: &str| {
            if s.contains([',', '"', '\n', '\r']) || s.starts_with(' ') || s.ends_with(' ') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        let line = |first: &str, rest: &[String]| {
            let mut fields = vec![quote(first)];
            fields.extend(rest.iter().map(|c| quote(c)));
            fields.join(",") + "\r\n"
        };
        let mut out = line(&self.corner, &self.columns);
        for (name, cells) in &self.rows {
            out.push_str(&line(name, cells));
        }
        out
    }
}

/// Reads a table file's text: the table and what R printed.
pub fn parse_table(text: &str) -> Result<(ResultTable, String), String> {
    let mut lines = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l));
    let mut next = |what: &str| {
        lines
            .next()
            .ok_or_else(|| format!("the table ended before {what}"))
    };
    if next("its start")? != "RCT1" {
        return Err("not a table file".into());
    }
    let title = unescape(next("its title")?);
    let corner = unescape(next("its corner")?);
    let columns: Vec<String> = next("its columns")?.split('\t').map(unescape).collect();
    if columns.is_empty() || columns.len() > MAX_TABLE_COLUMNS {
        return Err(format!("a table of {} columns", columns.len()));
    }
    let numeric: Vec<bool> = next("its column kinds")?
        .split('\t')
        .map(|f| f == "1")
        .collect();
    if numeric.len() != columns.len() {
        return Err("the column kinds do not match the columns".into());
    }
    let mut rows = Vec::new();
    loop {
        let line = next("its text")?;
        if line == "TEXT" {
            break;
        }
        if rows.len() >= MAX_TABLE_ROWS {
            return Err(format!("more than {MAX_TABLE_ROWS} rows"));
        }
        let mut fields = line.split('\t').map(unescape);
        let name = fields.next().unwrap_or_default();
        let cells: Vec<String> = fields.collect();
        if cells.len() != columns.len() {
            return Err(format!(
                "row {} has {} cells for {} columns",
                rows.len() + 1,
                cells.len(),
                columns.len()
            ));
        }
        rows.push((name, cells));
    }
    let printed = unescape(next("its text")?);
    Ok((
        ResultTable {
            title,
            corner,
            columns,
            numeric,
            rows,
        },
        printed,
    ))
}

/// Whether `name` is a table file R may name for run `run`: `<run>-t<n>.rct`,
/// a bare file name with nothing that could lead out of the folder.
pub fn is_table_file_name(name: &str, run: crate::RunId) -> bool {
    name.strip_prefix(&format!("{run}-t"))
        .and_then(|rest| rest.strip_suffix(".rct"))
        .is_some_and(|n| !n.is_empty() && n.len() <= 6 && n.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::escape;

    fn file(rows: &[&str]) -> String {
        let mut lines = vec![
            "RCT1".to_string(),
            escape("By group"),
            escape("arm"),
            format!("{}\t{}", escape("n"), escape("mean\tvalue")),
            "1\t1".to_string(),
        ];
        lines.extend(rows.iter().map(|r| r.to_string()));
        lines.push("TEXT".into());
        lines.push(escape("     n mean\na    3  1.5\n"));
        lines.join("\n") + "\n"
    }

    #[test]
    fn a_table_file_reads_back_with_its_text() {
        let parsed = parse_table(&file(&["a\t3\t1.5", "b\\tc\t10\tNA"]));
        assert!(parsed.is_ok(), "{parsed:?}");
        let Ok((t, text)) = parsed else { return };
        assert_eq!(t.title, "By group");
        assert_eq!(t.corner, "arm");
        assert_eq!(t.columns, vec!["n".to_string(), "mean\tvalue".to_string()]);
        assert_eq!(t.numeric, vec![true, true]);
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[1].0, "b\tc");
        assert_eq!(t.rows[1].1, vec!["10".to_string(), "NA".to_string()]);
        assert_eq!(text, "     n mean\na    3  1.5\n");
        assert_eq!(t.tsv(), "arm\tn\tmean value\na\t3\t1.5\nb c\t10\tNA\n");
        assert_eq!(
            t.csv(),
            "arm,n,mean\tvalue\r\na,3,1.5\r\nb\tc,10,NA\r\n",
            "tabs need no quotes in CSV"
        );
    }

    #[test]
    fn csv_quotes_what_needs_it() {
        let t = ResultTable {
            corner: String::new(),
            columns: vec!["a,b".into(), "say \"hi\"".into()],
            numeric: vec![false, false],
            rows: vec![(" x".into(), vec!["1".into(), "two\nlines".into()])],
            ..ResultTable::default()
        };
        assert_eq!(
            t.csv(),
            ",\"a,b\",\"say \"\"hi\"\"\"\r\n\" x\",1,\"two\nlines\"\r\n"
        );
    }

    #[test]
    fn damaged_or_odd_files_are_refused_not_panicked_on() {
        assert!(parse_table("").is_err());
        assert!(parse_table("RCT2\n").is_err());
        assert!(parse_table("RCT1\nt\nc\n").is_err(), "ends early");
        assert!(parse_table(&file(&["a\t3"])).is_err(), "a cell short");
        assert!(parse_table(&file(&["a\t3\t4\t5"])).is_err(), "a cell over");
        let no_text = file(&[]).replace("TEXT\n", "");
        assert!(parse_table(&no_text).is_err());
        let kinds = "RCT1\n\n\na\tb\n1\nTEXT\n\n";
        assert!(parse_table(kinds).is_err(), "kinds do not match");
        let wide = format!("RCT1\n\n\n{}\n", vec!["c"; 51].join("\t"));
        assert!(parse_table(&wide).is_err());
        let many: Vec<String> = (0..501).map(|i| format!("r{i}\t1\t2")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        assert!(parse_table(&file(&many)).is_err());
        // an empty table (no rows) is fine
        assert!(parse_table(&file(&[])).is_ok());
    }

    #[test]
    fn only_this_runs_own_table_files_are_taken() {
        assert!(is_table_file_name("7-t1.rct", 7));
        assert!(is_table_file_name("7-t012.rct", 7));
        assert!(!is_table_file_name("8-t1.rct", 7));
        assert!(!is_table_file_name("7-t.rct", 7));
        assert!(!is_table_file_name("7-t1.png", 7));
        assert!(!is_table_file_name("../7-t1.rct", 7));
        assert!(!is_table_file_name("7-t1/../x.rct", 7));
        assert!(!is_table_file_name("7-t1234567.rct", 7));
    }
}
