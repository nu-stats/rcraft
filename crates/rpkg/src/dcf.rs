//! Reader for `PACKAGES` files: records of `Field: value` lines separated by
//! blank lines, a continuation line starting with a space or tab (the
//! Debian control format R uses).

use std::collections::HashMap;

use crate::table::Table;
use crate::unpack::{unpack, MAX_UNPACKED_BYTES};
use crate::{Error, Result};

/// Most records accepted (CRAN has about 22,000).
pub const MAX_RECORDS: usize = 1_000_000;

/// Reads `PACKAGES.gz` (or the plain `PACKAGES`), keeping the fields in `keep`.
/// Every record is one row; a field a record lacks is missing.
pub fn read(bytes: &[u8], keep: &[&str]) -> Result<Table> {
    let raw = unpack(bytes, MAX_UNPACKED_BYTES)?;
    table(&String::from_utf8_lossy(&raw), keep)
}

/// The same from text already unpacked.
pub fn table(text: &str, keep: &[&str]) -> Result<Table> {
    let wanted: HashMap<&str, usize> = keep.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let mut columns: Vec<Vec<Option<String>>> = keep.iter().map(|_| Vec::new()).collect();
    let mut rows = 0usize;
    let mut current: Vec<Option<String>> = vec![None; keep.len()];
    let mut open: Option<usize> = None; // the kept column a continuation line extends
    let mut any = false;

    let mut flush = |current: &mut Vec<Option<String>>, any: &mut bool| -> Result<()> {
        if !*any {
            return Ok(());
        }
        if rows >= MAX_RECORDS {
            return Err(Error::TooBig(format!("more than {MAX_RECORDS} records")));
        }
        for (col, cell) in columns.iter_mut().zip(current.iter_mut()) {
            col.push(cell.take());
        }
        rows += 1;
        *any = false;
        Ok(())
    };

    for line in text.lines() {
        if line.trim().is_empty() {
            flush(&mut current, &mut any)?;
            open = None;
            continue;
        }
        if line.starts_with([' ', '\t']) {
            if let Some(i) = open {
                if let Some(Some(v)) = current.get_mut(i) {
                    v.push('\n');
                    v.push_str(line.trim());
                }
            }
            continue;
        }
        let Some((field, value)) = line.split_once(':') else {
            return Err(Error::Format(format!(
                "a line with no field name: {}",
                line.chars().take(40).collect::<String>()
            )));
        };
        any = true;
        open = None;
        if let Some(&i) = wanted.get(field.trim()) {
            if let Some(cell) = current.get_mut(i) {
                *cell = Some(value.trim().to_string());
                open = Some(i);
            }
        }
    }
    flush(&mut current, &mut any)?;
    Ok(Table {
        names: keep.iter().map(|k| (*k).to_string()).collect(),
        columns,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Package: alpha\nVersion: 1.0.2\nDepends: R (>= 3.5.0)\nImports: beta,\n        gamma (>= 1.0)\nNeedsCompilation: no\n\nPackage: beta\r\nVersion: 0.3\r\nLicense: GPL-2 | GPL-3\r\n\r\n\n\nPackage: gamma\nVersion: 2.0\n";

    #[test]
    fn records_continuations_and_blank_lines() {
        let t = table(
            SAMPLE,
            &["Package", "Version", "Imports", "Depends", "Title"],
        );
        let t = t.unwrap_or_default();
        assert_eq!(t.rows, 3);
        assert_eq!(t.get("Package", 2), Some("gamma"));
        assert_eq!(t.get("Version", 0), Some("1.0.2"));
        assert_eq!(t.get("Imports", 0), Some("beta,\ngamma (>= 1.0)"));
        assert_eq!(t.get("Imports", 1), None);
        assert_eq!(t.get("Depends", 0), Some("R (>= 3.5.0)"));
        // a field that is asked for but absent everywhere is a column of missing values
        assert_eq!(t.get("Title", 0), None);
        // CRLF line ends are tolerated
        assert_eq!(t.get("Package", 1), Some("beta"));
    }

    #[test]
    fn a_line_without_a_field_name_is_an_error() {
        assert!(matches!(
            table("Package: a\nno colon here\n", &["Package"]),
            Err(Error::Format(_))
        ));
    }

    #[test]
    fn an_empty_file_is_an_empty_table() {
        assert_eq!(table("", &["Package"]).map(|t| t.rows), Ok(0));
        assert_eq!(table("\n\n\n", &["Package"]).map(|t| t.rows), Ok(0));
    }

    #[test]
    fn gzip_is_unpacked() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        assert!(e.write_all(SAMPLE.as_bytes()).is_ok());
        let gz = e.finish().unwrap_or_default();
        assert_eq!(read(&gz, &["Package"]).map(|t| t.rows), Ok(3));
    }
}
