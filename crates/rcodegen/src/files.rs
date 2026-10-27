//! R code for "Use my file" with Excel, Stata, SPSS, SAS and R data files,
//! and one type, [`Import`], that stands for any of them.
//!
//! Excel files are read with `readxl`, Stata, SPSS and SAS files with `haven`,
//! and Parquet and Feather files with `arrow`. None of these packages comes
//! with R, so [`Import::package`] names what must be installed first.

use crate::import::{r_bool, r_name, r_string, CsvImport};

/// What kind of file a path names, from its ending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// csv, tsv, txt, tab, dat, or no ending.
    Delimited,
    Excel,
    Stata,
    Spss,
    SpssPortable,
    Sas,
    SasTransport,
    /// A saved R object (`readRDS`).
    Rds,
    /// Saved R objects (`load`).
    RData,
    /// Apache Parquet (`arrow::read_parquet`).
    Parquet,
    /// Feather and Arrow IPC files (`arrow::read_feather`).
    Feather,
    Unknown,
}

/// The endings of the files "Use my file" opens, for a file chooser's filter.
pub const DATA_EXTENSIONS: &[&str] = &[
    "csv", "tsv", "txt", "tab", "dat", "psv", "xlsx", "xlsm", "xls", "dta", "sav", "zsav", "por",
    "sas7bdat", "xpt", "rds", "rdata", "rda", "parquet", "feather", "arrow",
];

impl FileKind {
    pub fn from_path(path: &str) -> FileKind {
        let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
        let Some((_, ext)) = file.rsplit_once('.') else {
            return FileKind::Delimited;
        };
        match ext.to_ascii_lowercase().as_str() {
            "csv" | "tsv" | "txt" | "tab" | "dat" | "psv" => FileKind::Delimited,
            "xlsx" | "xlsm" | "xls" => FileKind::Excel,
            "dta" => FileKind::Stata,
            "sav" | "zsav" => FileKind::Spss,
            "por" => FileKind::SpssPortable,
            "sas7bdat" => FileKind::Sas,
            "xpt" => FileKind::SasTransport,
            "rds" => FileKind::Rds,
            "rdata" | "rda" => FileKind::RData,
            "parquet" => FileKind::Parquet,
            "feather" | "arrow" => FileKind::Feather,
            _ => FileKind::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FileKind::Delimited => "Delimited text",
            FileKind::Excel => "Excel workbook",
            FileKind::Stata => "Stata data",
            FileKind::Spss => "SPSS data",
            FileKind::SpssPortable => "SPSS portable",
            FileKind::Sas => "SAS data",
            FileKind::SasTransport => "SAS transport",
            FileKind::Rds => "R data (one object)",
            FileKind::RData => "R data (workspace)",
            FileKind::Parquet => "Parquet data",
            FileKind::Feather => "Feather data",
            FileKind::Unknown => "Unknown file type",
        }
    }
}

/// An Excel workbook, read with `readxl::read_excel`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcelImport {
    pub path: String,
    pub name: String,
    /// A sheet name; empty means the first sheet.
    pub sheet: String,
    /// A cell range such as `B2:F40`; empty means the whole sheet.
    pub range: String,
    /// The first row holds the variable names.
    pub header: bool,
    /// Rows to skip at the top (ignored when a range is given).
    pub skip: usize,
    pub na_strings: Vec<String>,
    pub trim_ws: bool,
    /// Rows looked at to decide each column's type.
    pub guess_max: usize,
}

impl ExcelImport {
    pub fn new(path: &str) -> ExcelImport {
        let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
        ExcelImport {
            path: path.to_string(),
            name: crate::import::name_from_file(file),
            sheet: String::new(),
            range: String::new(),
            header: true,
            skip: 0,
            na_strings: vec![String::new(), "NA".to_string()],
            trim_ws: true,
            guess_max: 1000,
        }
    }

    pub fn read_expr(&self, rows: Option<usize>) -> String {
        let mut args = vec![r_string(&self.path.replace('\\', "/"))];
        if !self.sheet.trim().is_empty() {
            args.push(format!("sheet = {}", r_string(self.sheet.trim())));
        }
        if !self.range.trim().is_empty() {
            args.push(format!("range = {}", r_string(self.range.trim())));
        } else {
            if self.skip > 0 {
                args.push(format!("skip = {}", self.skip));
            }
            if let Some(n) = rows {
                args.push(format!("n_max = {n}"));
            }
        }
        args.push(format!("col_names = {}", r_bool(self.header)));
        let na = self
            .na_strings
            .iter()
            .map(|s| r_string(s))
            .collect::<Vec<_>>()
            .join(", ");
        args.push(format!("na = c({na})"));
        args.push(format!("trim_ws = {}", r_bool(self.trim_ws)));
        args.push(format!("guess_max = {}", self.guess_max));
        format!("as.data.frame(readxl::read_excel({}))", args.join(", "))
    }

    pub fn problem(&self) -> Option<&'static str> {
        if self.path.trim().is_empty() {
            return Some("Choose a file.");
        }
        if !is_cell_range(self.range.trim()) {
            return Some("The range should look like B2:F40, or be empty for the whole sheet.");
        }
        name_problem(&self.name)
    }
}

/// `""`, `A1`, `B2:F40`, `A:C`, `2:10`, with an optional `Sheet!` in front.
pub fn is_cell_range(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let body = text.rsplit_once('!').map_or(text, |(_, r)| r);
    let part = |p: &str| {
        let letters = p.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        let digits = p.len() - letters;
        let (_, d) = p.split_at(letters);
        letters <= 3
            && d.len() == digits
            && d.chars().all(|c| c.is_ascii_digit())
            && (letters > 0 || digits > 0)
    };
    let mut parts = body.split(':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(a), None, None) => part(a) && a.chars().any(|c| c.is_ascii_digit()),
        (Some(a), Some(b), None) => part(a) && part(b),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HavenKind {
    Stata,
    Spss,
    SpssPortable,
    Sas,
    SasTransport,
}

impl HavenKind {
    fn function(self) -> &'static str {
        match self {
            HavenKind::Stata => "read_dta",
            HavenKind::Spss => "read_sav",
            HavenKind::SpssPortable => "read_por",
            HavenKind::Sas => "read_sas",
            HavenKind::SasTransport => "read_xpt",
        }
    }

    /// The reader takes an `encoding` argument.
    pub fn has_encoding(self) -> bool {
        matches!(self, HavenKind::Stata | HavenKind::Spss | HavenKind::Sas)
    }

    /// The reader takes a `user_na` argument.
    pub fn has_user_na(self) -> bool {
        matches!(self, HavenKind::Spss | HavenKind::SpssPortable)
    }
}

/// A Stata, SPSS or SAS file, read with `haven`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HavenImport {
    pub path: String,
    pub name: String,
    pub kind: HavenKind,
    /// Turn labelled variables into factors. Off keeps the value labels as
    /// attributes, which the Variables pane shows.
    pub labels_as_factors: bool,
    /// Text encoding of the file; empty lets haven decide.
    pub encoding: String,
    /// SPSS: keep user-defined missing values as ordinary values.
    pub user_na: bool,
}

impl HavenImport {
    pub fn new(path: &str, kind: HavenKind) -> HavenImport {
        let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
        HavenImport {
            path: path.to_string(),
            name: crate::import::name_from_file(file),
            kind,
            labels_as_factors: false,
            encoding: String::new(),
            user_na: false,
        }
    }

    pub fn read_expr(&self, rows: Option<usize>) -> String {
        let mut args = vec![r_string(&self.path.replace('\\', "/"))];
        if self.kind.has_encoding() && !self.encoding.trim().is_empty() {
            args.push(format!("encoding = {}", r_string(self.encoding.trim())));
        }
        if self.kind.has_user_na() && self.user_na {
            args.push("user_na = TRUE".to_string());
        }
        if let Some(n) = rows {
            args.push(format!("n_max = {n}"));
        }
        let read = format!("haven::{}({})", self.kind.function(), args.join(", "));
        if self.labels_as_factors {
            format!("as.data.frame(haven::as_factor({read}))")
        } else {
            format!("as.data.frame({read})")
        }
    }

    pub fn problem(&self) -> Option<&'static str> {
        if self.path.trim().is_empty() {
            return Some("Choose a file.");
        }
        name_problem(&self.name)
    }
}

/// A saved R object: `.rds` (one object, named here) or `.RData` (objects
/// come back under the names they were saved with).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RObjectImport {
    pub path: String,
    pub name: String,
    pub workspace: bool,
}

impl RObjectImport {
    pub fn new(path: &str, workspace: bool) -> RObjectImport {
        let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
        RObjectImport {
            path: path.to_string(),
            name: crate::import::name_from_file(file),
            workspace,
        }
    }

    pub fn problem(&self) -> Option<&'static str> {
        if self.path.trim().is_empty() {
            return Some("Choose a file.");
        }
        if self.workspace {
            None
        } else {
            name_problem(&self.name)
        }
    }
}

fn name_problem(name: &str) -> Option<&'static str> {
    if name.trim().is_empty() {
        Some("Give the data frame a name.")
    } else if !crate::import::is_syntactic(name) {
        Some("The name must start with a letter and use letters, digits, . and _.")
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowFormat {
    Parquet,
    Feather,
}

impl ArrowFormat {
    fn function(self) -> &'static str {
        match self {
            ArrowFormat::Parquet => "read_parquet",
            ArrowFormat::Feather => "read_feather",
        }
    }
}

/// A Parquet or Feather file, read with `arrow`. Both keep each column's type,
/// so there is nothing to choose: the file is read as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrowImport {
    pub path: String,
    pub name: String,
    pub format: ArrowFormat,
}

impl ArrowImport {
    pub fn new(path: &str, format: ArrowFormat) -> ArrowImport {
        let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
        ArrowImport {
            path: path.to_string(),
            name: crate::import::name_from_file(file),
            format,
        }
    }

    fn read(&self, as_data_frame: bool) -> String {
        let path = r_string(&self.path.replace('\\', "/"));
        let function = self.format.function();
        if as_data_frame {
            format!("as.data.frame(arrow::{function}({path}))")
        } else {
            format!("arrow::{function}({path}, as_data_frame = FALSE)")
        }
    }

    /// R code that prints the size of the file and its first rows. The file
    /// is read as an Arrow table (cheap for Feather, a full read for Parquet)
    /// and only the first rows are turned into a data frame.
    pub fn preview_code(&self, rows: usize) -> String {
        format!(
            "local({{ op <- options(width = 150); on.exit(options(op)); \
             t <- {}; n <- min({rows}L, as.integer(t$num_rows)); \
             cat(t$num_columns, \"columns;\", t$num_rows, \"rows\\n\"); \
             print(as.data.frame(t$Slice(0L, n))) }})",
            self.read(false)
        )
    }

    pub fn problem(&self) -> Option<&'static str> {
        if self.path.trim().is_empty() {
            return Some("Choose a file.");
        }
        name_problem(&self.name)
    }
}

/// Any file the dialog can import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Import {
    Csv(CsvImport),
    Excel(ExcelImport),
    Haven(HavenImport),
    Arrow(ArrowImport),
    RObject(RObjectImport),
}

impl Import {
    /// The data frame the import creates, when it is known beforehand.
    pub fn result_name(&self) -> Option<&str> {
        match self {
            Import::Csv(c) => Some(&c.name),
            Import::Excel(e) => Some(&e.name),
            Import::Haven(h) => Some(&h.name),
            Import::Arrow(a) => Some(&a.name),
            Import::RObject(r) if !r.workspace => Some(&r.name),
            Import::RObject(_) => None,
        }
    }

    pub fn name_mut(&mut self) -> Option<&mut String> {
        match self {
            Import::Csv(c) => Some(&mut c.name),
            Import::Excel(e) => Some(&mut e.name),
            Import::Haven(h) => Some(&mut h.name),
            Import::Arrow(a) => Some(&mut a.name),
            Import::RObject(r) if !r.workspace => Some(&mut r.name),
            Import::RObject(_) => None,
        }
    }

    /// The package that must be installed, if the format needs one.
    pub fn package(&self) -> Option<&'static str> {
        match self {
            Import::Excel(_) => Some("readxl"),
            Import::Haven(_) => Some("haven"),
            Import::Arrow(_) => Some("arrow"),
            Import::Csv(_) | Import::RObject(_) => None,
        }
    }

    pub fn problem(&self) -> Option<&'static str> {
        match self {
            Import::Csv(c) => c.problem(),
            Import::Excel(e) => e.problem(),
            Import::Haven(h) => h.problem(),
            Import::Arrow(a) => a.problem(),
            Import::RObject(r) => r.problem(),
        }
    }

    /// The R code that imports the file.
    pub fn code(&self) -> String {
        match self {
            Import::Csv(c) => c.code(),
            Import::Excel(e) => format!("{} <- {}", r_name(&e.name), e.read_expr(None)),
            Import::Haven(h) => format!("{} <- {}", r_name(&h.name), h.read_expr(None)),
            Import::Arrow(a) => format!("{} <- {}", r_name(&a.name), a.read(true)),
            Import::RObject(r) => {
                let path = r_string(&r.path.replace('\\', "/"));
                if r.workspace {
                    format!("load({path})")
                } else {
                    format!("{} <- readRDS({path})", r_name(&r.name))
                }
            }
        }
    }

    /// R code that prints the first rows of what the import would create,
    /// without creating anything. Output is for the dialog's preview.
    pub fn preview_code(&self, rows: usize) -> String {
        let show = |expr: &str| {
            format!(
                "local({{ op <- options(width = 150); on.exit(options(op)); \
                 x <- {expr}; cat(ncol(x), \"columns; first\", nrow(x), \"rows\\n\"); \
                 print(utils::head(x, {rows})) }})"
            )
        };
        match self {
            Import::Csv(c) => show(&c.read_expr(Some(rows))),
            Import::Excel(e) => show(&e.read_expr(Some(rows))),
            Import::Haven(h) => show(&h.read_expr(Some(rows))),
            Import::Arrow(a) => a.preview_code(rows),
            Import::RObject(r) => {
                let path = r_string(&r.path.replace('\\', "/"));
                if r.workspace {
                    format!(
                        "local({{ e <- new.env(); n <- load({path}, envir = e); \
                         cat(\"objects:\", paste(n, collapse = \", \"), \"\\n\") }})"
                    )
                } else {
                    format!(
                        "local({{ x <- readRDS({path}); cat(\"an object of class\", \
                         paste(class(x), collapse = \" \"), \"\\n\"); \
                         if (is.data.frame(x)) print(utils::head(x, {rows})) else utils::str(x, max.level = 1) }})"
                    )
                }
            }
        }
    }
}

/// R code that lists the sheets of a workbook, one per line, or prints
/// `ERROR` and a message.
pub fn sheets_code(path: &str) -> String {
    format!(
        "tryCatch(cat(readxl::excel_sheets({}), sep = \"\\n\"), \
         error = function(e) cat(\"ERROR\\n\", conditionMessage(e), sep = \"\"))",
        r_string(&path.replace('\\', "/"))
    )
}

/// R code that prints `yes` or `no` for whether `package` can be loaded.
pub fn package_check_code(package: &str) -> String {
    format!(
        "cat(if (requireNamespace({}, quietly = TRUE)) \"yes\" else \"no\")",
        r_string(package)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parquet_and_feather_are_read_with_arrow() {
        let p = Import::Arrow(ArrowImport::new(
            "C:\\data\\Big file.parquet",
            ArrowFormat::Parquet,
        ));
        assert_eq!(p.package(), Some("arrow"));
        assert_eq!(p.result_name(), Some("Big_file"));
        assert_eq!(
            p.code(),
            "Big_file <- as.data.frame(arrow::read_parquet(\"C:/data/Big file.parquet\"))"
        );
        let f = Import::Arrow(ArrowImport::new("/d/x.feather", ArrowFormat::Feather));
        assert_eq!(
            f.code(),
            "x <- as.data.frame(arrow::read_feather(\"/d/x.feather\"))"
        );
        let preview = f.preview_code(8);
        assert!(preview.contains("arrow::read_feather(\"/d/x.feather\", as_data_frame = FALSE)"));
        assert!(preview.contains("min(8L, as.integer(t$num_rows))"));
        let mut empty = ArrowImport::new("", ArrowFormat::Parquet);
        assert_eq!(empty.problem(), Some("Choose a file."));
        empty.path = "a.parquet".into();
        empty.name = "1bad".into();
        assert!(empty.problem().is_some());
    }

    #[test]
    fn every_extension_the_chooser_offers_is_a_known_kind() {
        for ext in DATA_EXTENSIONS {
            let kind = FileKind::from_path(&format!("file.{ext}"));
            assert_ne!(kind, FileKind::Unknown, "{ext}");
        }
        // and no ending the dialog reads is left off the list
        for ext in ["xlsx", "dta", "rds", "parquet"] {
            assert!(DATA_EXTENSIONS.contains(&ext), "{ext}");
        }
    }

    #[test]
    fn files_are_told_apart_by_their_ending() {
        let k = FileKind::from_path;
        assert_eq!(k("C:\\d\\a.CSV"), FileKind::Delimited);
        assert_eq!(k("a.tsv"), FileKind::Delimited);
        assert_eq!(k("README"), FileKind::Delimited);
        assert_eq!(k("a.xlsx"), FileKind::Excel);
        assert_eq!(k("a.xls"), FileKind::Excel);
        assert_eq!(k("a.dta"), FileKind::Stata);
        assert_eq!(k("a.zsav"), FileKind::Spss);
        assert_eq!(k("a.por"), FileKind::SpssPortable);
        assert_eq!(k("a.sas7bdat"), FileKind::Sas);
        assert_eq!(k("a.xpt"), FileKind::SasTransport);
        assert_eq!(k("a.RDS"), FileKind::Rds);
        assert_eq!(k("a.RData"), FileKind::RData);
        assert_eq!(k("a.parquet"), FileKind::Parquet);
        assert_eq!(k("a.Feather"), FileKind::Feather);
        assert_eq!(k("a.arrow"), FileKind::Feather);
        assert_eq!(k("a.bin"), FileKind::Unknown);
        assert_eq!(k("C:\\dir.v2\\noext"), FileKind::Delimited);
    }

    #[test]
    fn excel_code_names_the_sheet_and_range() {
        let mut e = ExcelImport::new("C:\\d\\Budget 2024.xlsx");
        assert_eq!(e.name, "Budget_2024");
        assert_eq!(
            Import::Excel(e.clone()).code(),
            "Budget_2024 <- as.data.frame(readxl::read_excel(\"C:/d/Budget 2024.xlsx\", \
col_names = TRUE, na = c(\"\", \"NA\"), trim_ws = TRUE, guess_max = 1000))"
        );
        e.sheet = "Q1 totals".into();
        e.range = "B2:F40".into();
        e.skip = 3;
        e.header = false;
        let code = e.read_expr(None);
        assert!(code.contains("sheet = \"Q1 totals\", range = \"B2:F40\", col_names = FALSE"));
        assert!(!code.contains("skip"), "a range replaces skip: {code}");
        e.range.clear();
        assert!(e.read_expr(Some(10)).contains("skip = 3, n_max = 10"));
    }

    #[test]
    fn ranges_are_checked() {
        for ok in [
            "",
            "A1",
            "B2:F40",
            "A:C",
            "2:10",
            "Sheet1!A1:B2",
            "aa10:ZZ99",
        ] {
            assert!(is_cell_range(ok), "{ok}");
        }
        for bad in [
            "A", "1A", "A1:", ":B2", "A1:B2:C3", "AAAA1", "B 2", "A1;B2", "!",
        ] {
            assert!(!is_cell_range(bad), "{bad}");
        }
        let mut e = ExcelImport::new("x.xlsx");
        e.range = "nonsense".into();
        assert!(e.problem().is_some());
    }

    #[test]
    fn haven_code_uses_the_right_reader() {
        let mut h = HavenImport::new("d/survey.dta", HavenKind::Stata);
        assert_eq!(
            Import::Haven(h.clone()).code(),
            "survey <- as.data.frame(haven::read_dta(\"d/survey.dta\"))"
        );
        h.labels_as_factors = true;
        h.encoding = "latin1".into();
        assert_eq!(
            h.read_expr(None),
            "as.data.frame(haven::as_factor(haven::read_dta(\"d/survey.dta\", encoding = \"latin1\")))"
        );
        let mut s = HavenImport::new("a.sav", HavenKind::Spss);
        s.user_na = true;
        assert!(s.read_expr(Some(5)).contains("user_na = TRUE, n_max = 5"));
        let x = HavenImport::new("a.xpt", HavenKind::SasTransport);
        assert!(x.read_expr(None).contains("read_xpt"));
        let mut x2 = x.clone();
        x2.encoding = "latin1".into();
        assert!(
            !x2.read_expr(None).contains("encoding"),
            "xpt has no encoding"
        );
    }

    #[test]
    fn r_objects_and_packages() {
        let rds = Import::RObject(RObjectImport::new("a/b.rds", false));
        assert_eq!(rds.code(), "b <- readRDS(\"a/b.rds\")");
        let rdata = Import::RObject(RObjectImport::new("a/b.RData", true));
        assert_eq!(rdata.code(), "load(\"a/b.RData\")");
        assert_eq!(rdata.result_name(), None);
        assert_eq!(rds.package(), None);
        assert_eq!(
            Import::Excel(ExcelImport::new("a.xlsx")).package(),
            Some("readxl")
        );
        assert_eq!(
            Import::Haven(HavenImport::new("a.dta", HavenKind::Stata)).package(),
            Some("haven")
        );
    }

    #[test]
    fn previews_do_not_create_anything() {
        let p = Import::Haven(HavenImport::new("a.dta", HavenKind::Stata)).preview_code(8);
        assert!(p.starts_with("local({"));
        // it assigns only to a variable local to the preview, not to the data frame name
        assert!(!p.contains("a <- "), "{p}");
        assert!(p.contains("x <- as.data.frame(haven::read_dta"), "{p}");
        assert!(p.contains("n_max = 8"));
        assert!(sheets_code("C:\\a b.xlsx").contains("excel_sheets(\"C:/a b.xlsx\")"));
        assert!(package_check_code("haven").contains("requireNamespace(\"haven\""));
    }

    #[test]
    fn names_are_checked_for_every_kind() {
        let mut h = HavenImport::new("a.dta", HavenKind::Stata);
        assert!(h.problem().is_none());
        h.name = "bad name".into();
        assert!(h.problem().is_some());
        let r = RObjectImport::new("a.RData", true);
        assert!(r.problem().is_none(), "a workspace file needs no name");
    }
}
