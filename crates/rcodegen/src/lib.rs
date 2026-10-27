//! Turns dialog specs into R code and parses R output.

pub mod import;

pub use import::{
    is_syntactic, name_from_file, parse_na_strings, r_name, r_string, CsvImport, Quote,
};
