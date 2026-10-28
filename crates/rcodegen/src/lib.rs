//! Turns dialog specs into R code and parses R output.

pub mod dialog;
pub mod failure;
pub mod files;
pub mod import;
pub mod packages;
pub mod rc;

pub use files::{
    is_cell_range, missing_packages_code, package_check_code, sheets_code, ArrowFormat,
    ArrowImport, ExcelImport, FileKind, HavenImport, HavenKind, Import, RObjectImport,
    DATA_EXTENSIONS,
};
pub use import::{
    is_syntactic, name_from_file, parse_na_strings, r_name, r_string, CsvImport, Quote,
};
