//! R code for "Use my file" with a delimited text file: base R's
//! `read.table`, so it works with any R and no package.

use rcraft_rdata::sniff::{Encoding, Sniffed};

/// A name R can use as it is; anything else needs backticks.
pub fn is_syntactic(name: &str) -> bool {
    let mut chars = name.chars();
    let starts_ok = chars.next().is_some_and(|c| c.is_alphabetic() || c == '.');
    let dot_digit = name
        .strip_prefix('.')
        .and_then(|rest| rest.chars().next())
        .is_some_and(|c| c.is_ascii_digit());
    let reserved = matches!(
        name,
        "if" | "else"
            | "repeat"
            | "while"
            | "function"
            | "for"
            | "next"
            | "break"
            | "TRUE"
            | "FALSE"
            | "NULL"
            | "Inf"
            | "NaN"
            | "NA"
            | "NA_integer_"
            | "NA_real_"
            | "NA_character_"
            | "in"
            | "..."
    );
    starts_ok
        && !dot_digit
        && !reserved
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
}

/// A name as R code: as it is when syntactic, else in backticks.
pub fn r_name(name: &str) -> String {
    if is_syntactic(name) {
        name.to_string()
    } else {
        format!("`{}`", name.replace('\\', "\\\\").replace('`', "\\`"))
    }
}

/// A data frame name made from a file name: `my data-2024.csv` becomes
/// `my_data_2024`; falls back to `data`.
pub fn name_from_file(file_name: &str) -> String {
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    let mut name: String = stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    name = name.trim_matches('_').to_string();
    if name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c == '_')
    {
        name.insert(0, 'd');
    }
    if is_syntactic(&name) {
        name
    } else {
        "data".to_string()
    }
}

/// A string as an R literal.
pub fn r_string(s: &str) -> String {
    rcraft_rdata::r_string(s)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quote {
    Double,
    Single,
    Both,
    None,
}

impl Quote {
    pub const ALL: [Quote; 4] = [Quote::Double, Quote::Single, Quote::Both, Quote::None];

    pub fn label(self) -> &'static str {
        match self {
            Quote::Double => "double quotes (\")",
            Quote::Single => "single quotes (')",
            Quote::Both => "either kind",
            Quote::None => "none",
        }
    }

    fn r_text(self) -> &'static str {
        match self {
            Quote::Double => "\"",
            Quote::Single => "'",
            Quote::Both => "\"'",
            Quote::None => "",
        }
    }
}

/// Everything the dialog sets for a delimited text file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsvImport {
    pub path: String,
    /// The data frame to create.
    pub name: String,
    /// The first row holds the variable names.
    pub header: bool,
    /// `None` reads a single column.
    pub delimiter: Option<char>,
    pub decimal: char,
    pub quote: Quote,
    /// Lines to skip before the header or the first row.
    pub skip: usize,
    /// Text read as missing, separated by commas in the dialog.
    pub na_strings: Vec<String>,
    pub encoding: Encoding,
    /// Remove spaces around values.
    pub strip_white: bool,
    pub strings_as_factors: bool,
}

impl CsvImport {
    /// The choices a file's start suggests.
    pub fn from_sniffed(path: &str, sniffed: &Sniffed) -> CsvImport {
        let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        CsvImport {
            path: path.to_string(),
            name: name_from_file(file_name),
            header: sniffed.header,
            delimiter: sniffed.delimiter,
            decimal: sniffed.decimal,
            quote: Quote::Double,
            skip: 0,
            na_strings: vec![String::new(), "NA".to_string()],
            encoding: sniffed.encoding,
            strip_white: false,
            strings_as_factors: false,
        }
    }

    /// Why the choices cannot be used, if they cannot.
    pub fn problem(&self) -> Option<&'static str> {
        if self.path.trim().is_empty() {
            return Some("Choose a file.");
        }
        if self.name.trim().is_empty() {
            return Some("Give the data frame a name.");
        }
        if !is_syntactic(&self.name) {
            return Some("The name must start with a letter and use letters, digits, . and _.");
        }
        if self.delimiter == Some(self.decimal) {
            return Some("The delimiter and the decimal mark cannot be the same.");
        }
        None
    }

    /// The R code that reads the file into `name`.
    pub fn code(&self) -> String {
        format!("{} <- {}", r_name(&self.name), self.read_expr(None))
    }

    /// The `read.table(...)` call; `rows` limits it to the first rows, for a preview.
    pub fn read_expr(&self, rows: Option<usize>) -> String {
        let path = self.path.replace('\\', "/");
        let sep = self.delimiter.map_or(",".to_string(), |d| d.to_string());
        // a space means any run of white space, as in R's own convention
        let sep_arg = if sep == " " { String::new() } else { sep };
        let na = self
            .na_strings
            .iter()
            .map(|s| r_string(s))
            .collect::<Vec<_>>()
            .join(", ");
        let limit = rows.map_or(String::new(), |n| format!(", nrows = {n}"));
        format!(
            "read.table({path}, header = {header}, sep = {sep}, dec = {dec}, \
quote = {quote}, skip = {skip}, na.strings = c({na}), fileEncoding = {enc}, \
stringsAsFactors = {factors}, strip.white = {strip}, comment.char = \"\", \
fill = TRUE, check.names = TRUE{limit})",
            path = r_string(&path),
            header = r_bool(self.header),
            sep = r_string(&sep_arg),
            dec = r_string(&self.decimal.to_string()),
            quote = r_string(self.quote.r_text()),
            skip = self.skip,
            enc = r_string(self.encoding.r_name()),
            factors = r_bool(self.strings_as_factors),
            strip = r_bool(self.strip_white),
        )
    }
}

pub(crate) fn r_bool(b: bool) -> &'static str {
    if b {
        "TRUE"
    } else {
        "FALSE"
    }
}

/// Splits the dialog's text for missing values (`NA, -9, n/a`) into items;
/// an empty item means an empty field.
pub fn parse_na_strings(text: &str) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    text.split(',').map(|s| s.trim().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CsvImport {
        CsvImport {
            path: "C:\\data\\my file.csv".into(),
            name: "patients".into(),
            header: true,
            delimiter: Some(';'),
            decimal: ',',
            quote: Quote::Double,
            skip: 2,
            na_strings: vec![String::new(), "NA".into(), "-9".into()],
            encoding: Encoding::Latin1,
            strip_white: true,
            strings_as_factors: false,
        }
    }

    #[test]
    fn the_code_names_every_choice() {
        let code = sample().code();
        assert_eq!(
            code,
            "patients <- read.table(\"C:/data/my file.csv\", header = TRUE, sep = \";\", dec = \",\", \
quote = \"\\\"\", skip = 2, na.strings = c(\"\", \"NA\", \"-9\"), fileEncoding = \"latin1\", \
stringsAsFactors = FALSE, strip.white = TRUE, comment.char = \"\", fill = TRUE, check.names = TRUE)"
        );
    }

    #[test]
    fn a_tab_and_a_space_are_written_the_way_r_wants() {
        let mut s = sample();
        s.delimiter = Some('\t');
        assert!(s.code().contains("sep = \"\\t\""));
        s.delimiter = Some(' ');
        assert!(s.code().contains("sep = \"\""), "{}", s.code());
        s.delimiter = None;
        assert!(s.code().contains("sep = \",\""));
    }

    #[test]
    fn names_are_checked_and_quoted() {
        assert_eq!(r_name("age"), "age");
        assert_eq!(r_name("my.var_2"), "my.var_2");
        assert_eq!(r_name(".hidden"), ".hidden");
        assert_eq!(r_name("two words"), "`two words`");
        assert_eq!(r_name("2fast"), "`2fast`");
        assert_eq!(r_name(".5x"), "`.5x`");
        assert_eq!(r_name("if"), "`if`");
        assert_eq!(r_name("a`b"), "`a\\`b`");
        assert_eq!(r_name("é"), "é");
        let mut s = sample();
        s.name = "bad name".into();
        assert!(s.problem().is_some());
        s.name = "ok".into();
        assert!(s.problem().is_none());
        s.decimal = ';';
        assert!(s.problem().is_some());
        s.decimal = '.';
        s.path = "  ".into();
        assert!(s.problem().is_some());
    }

    #[test]
    fn data_frame_names_come_from_file_names() {
        assert_eq!(name_from_file("patients.csv"), "patients");
        assert_eq!(name_from_file("my data-2024.csv"), "my_data_2024");
        assert_eq!(name_from_file("2024 results.tsv"), "d2024_results");
        assert_eq!(name_from_file("....csv"), "data");
        assert_eq!(name_from_file(".csv"), "data");
        assert_eq!(name_from_file("if.csv"), "data");
        assert_eq!(name_from_file(""), "data");
    }

    #[test]
    fn missing_value_text_is_split() {
        assert_eq!(parse_na_strings("NA, -9 ,n/a"), vec!["NA", "-9", "n/a"]);
        assert_eq!(parse_na_strings(",NA"), vec!["", "NA"]);
        assert!(parse_na_strings("  ").is_empty());
    }
}
