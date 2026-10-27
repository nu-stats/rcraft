//! Moves data frames between R and RCraft without copying whole tables.
//!
//! # RCraft columnar, version 1 (`RCB1`)
//!
//! R writes a data frame to a file with base R only (`rcb.R`, no package
//! needed); this crate reads it. All numbers are little-endian.
//!
//! ```text
//! "RCB1"  ncol:i32  nrow:i32
//! per column:
//!   name:str  class:str  kind:i32  attributes  payload
//! "RCBE"  length_before_trailer:f64
//! ```
//!
//! `str` is `len:i32` then that many UTF-8 bytes. A *text vector* of `n`
//! items is `n` lengths (`i32`, -1 for a missing item), `total:i32`, then the
//! bytes of all non-missing items joined together.
//!
//! Attributes are `count:i32 dropped:i32`, then per attribute `key:str`,
//! `kind:i32` (6 text, 2 number), `len:i32`, the items, `has_names:i32` and,
//! if set, the names as a text vector. `dropped` counts attributes that were
//! left out (not plain text or numbers, or too long), so a report can say so.
//!
//! Kinds and payloads: 1 integer (`i32`, missing is `i32::MIN`); 2 double
//! (`f64`, R's missing is a NaN with payload 1954); 3 logical (`i32`: 0, 1,
//! -1 missing); 4 factor (`ordered:i32`, `nlevels:i32`, levels as a text
//! vector, codes `i32`, -1 missing, from 0); 5 date (`f64` days); 6 text;
//! 7 date-time (`tz:str`, `f64` seconds); 8 duration (`units:str`, `f64`);
//! 9 other (list, matrix, complex...): shown as text, never guessed.
//!
//! The reader never trusts the file: every count is checked against the bytes
//! that are actually left, so a truncated or hostile file gives an error, not
//! a crash or a huge allocation.

use std::fmt;
use std::path::Path;

/// R's text for the R code that writes a table (a function expression).
pub const WRITER_R: &str = include_str!("rcb.R");

/// Most columns accepted.
pub const MAX_COLUMNS: usize = 100_000;
/// Most cells (rows times columns) R is asked to send.
pub const MAX_CELLS: f64 = 2e8;
/// Largest file read, in bytes.
pub const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// The value an integer column uses for missing.
pub const INT_NA: i32 = i32::MIN;

/// R code that writes the data frame in `variable` to `path`. `variable` must
/// be an R expression (a name); `path` is quoted for R.
pub fn write_code(variable: &str, path: &str) -> String {
    format!(
        "invisible(({})({}, {}, {:e}))",
        WRITER_R.trim_end(),
        variable,
        r_string(path),
        MAX_CELLS
    )
}

/// A string as an R literal, escaped.
pub fn r_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeError {
    /// The file ended before the table did.
    Truncated,
    /// Not an RCraft table, or the end marker is missing or wrong.
    NotATable(&'static str),
    /// A count or kind that cannot be right.
    Invalid(&'static str),
    TooLarge(&'static str),
    Io(String),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BridgeError::Truncated => write!(f, "the table file ended early"),
            BridgeError::NotATable(why) => write!(f, "not an RCraft table: {why}"),
            BridgeError::Invalid(why) => write!(f, "the table file is damaged: {why}"),
            BridgeError::TooLarge(what) => write!(f, "too large to open: {what}"),
            BridgeError::Io(e) => write!(f, "could not read the table file: {e}"),
        }
    }
}

impl std::error::Error for BridgeError {}

/// A table as R sent it.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub nrow: usize,
    pub columns: Vec<Column>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub name: String,
    /// R's class, joined by spaces (`"numeric"`, `"factor"`, `"haven_labelled vctrs_vctr double"`).
    pub class: String,
    pub attrs: Vec<Attr>,
    /// Attributes R left out because RCraft cannot carry them.
    pub dropped_attrs: u32,
    pub data: Data,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attr {
    pub key: String,
    pub value: AttrValue,
    pub names: Option<Vec<Option<String>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttrValue {
    Text(Vec<Option<String>>),
    Number(Vec<f64>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Data {
    /// Missing is [`INT_NA`].
    Int(Vec<i32>),
    /// Missing is R's NA (see [`is_na`]); NaN is a different thing.
    Double(Vec<f64>),
    /// 0, 1, or -1 for missing.
    Logical(Vec<i8>),
    /// Codes start at 0; -1 is missing.
    Factor {
        levels: Vec<Option<String>>,
        codes: Vec<i32>,
        ordered: bool,
    },
    /// Days since 1970-01-01.
    Date(Vec<f64>),
    /// Seconds since 1970-01-01 UTC.
    DateTime {
        secs: Vec<f64>,
        tz: String,
    },
    Duration {
        values: Vec<f64>,
        units: String,
    },
    Text(Vec<Option<String>>),
    /// A list, matrix or other column, shown as text.
    Other(Vec<Option<String>>),
}

/// Whether `x` is R's `NA_real_` (as opposed to an ordinary NaN).
pub fn is_na(x: f64) -> bool {
    x.is_nan() && (x.to_bits() & 0xFFFF_FFFF) == 1954
}

impl Data {
    pub fn len(&self) -> usize {
        match self {
            Data::Int(v) => v.len(),
            Data::Double(v) | Data::Date(v) => v.len(),
            Data::Logical(v) => v.len(),
            Data::Factor { codes, .. } => codes.len(),
            Data::DateTime { secs, .. } => secs.len(),
            Data::Duration { values, .. } => values.len(),
            Data::Text(v) | Data::Other(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// How many values are missing.
    pub fn missing(&self) -> usize {
        match self {
            Data::Int(v) => v.iter().filter(|&&x| x == INT_NA).count(),
            Data::Double(v) | Data::Date(v) => v.iter().filter(|&&x| x.is_nan()).count(),
            Data::Logical(v) => v.iter().filter(|&&x| x < 0).count(),
            Data::Factor { codes, .. } => codes.iter().filter(|&&x| x < 0).count(),
            Data::DateTime { secs, .. } => secs.iter().filter(|&&x| x.is_nan()).count(),
            Data::Duration { values, .. } => values.iter().filter(|&&x| x.is_nan()).count(),
            Data::Text(v) | Data::Other(v) => v.iter().filter(|x| x.is_none()).count(),
        }
    }

    /// A short word for the Variables pane.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Data::Int(_) => "integer",
            Data::Double(_) => "double",
            Data::Logical(_) => "logical",
            Data::Factor { ordered: true, .. } => "ordered factor",
            Data::Factor { .. } => "factor",
            Data::Date(_) => "date",
            Data::DateTime { .. } => "date-time",
            Data::Duration { .. } => "duration",
            Data::Text(_) => "text",
            Data::Other(_) => "other",
        }
    }
}

impl Column {
    /// The variable label (`attr(x, "label")`), if there is one.
    pub fn label(&self) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.key == "label")
            .and_then(|a| match &a.value {
                AttrValue::Text(v) => v.first().and_then(|s| s.as_deref()),
                AttrValue::Number(_) => None,
            })
    }

    /// Value labels as (value, label) pairs, from the `labels` attribute.
    pub fn value_labels(&self) -> Vec<(f64, String)> {
        let Some(a) = self.attrs.iter().find(|a| a.key == "labels") else {
            return Vec::new();
        };
        let (AttrValue::Number(values), Some(names)) = (&a.value, &a.names) else {
            return Vec::new();
        };
        values
            .iter()
            .zip(names.iter())
            .map(|(v, n)| (*v, n.clone().unwrap_or_default()))
            .collect()
    }
}

/// Reads a table file, deleting nothing.
pub fn read_file(path: &Path) -> Result<Table, BridgeError> {
    let meta = std::fs::metadata(path).map_err(|e| BridgeError::Io(e.to_string()))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(BridgeError::TooLarge("the table file"));
    }
    let bytes = std::fs::read(path).map_err(|e| BridgeError::Io(e.to_string()))?;
    decode(&bytes)
}

/// Decodes a table from its bytes.
pub fn decode(bytes: &[u8]) -> Result<Table, BridgeError> {
    const TRAILER: usize = 12;
    if bytes.get(..4) != Some(b"RCB1") {
        return Err(BridgeError::NotATable("wrong start"));
    }
    let body_len = bytes
        .len()
        .checked_sub(TRAILER)
        .ok_or(BridgeError::Truncated)?;
    let mut tail = Cursor::new(bytes.get(body_len..).ok_or(BridgeError::Truncated)?);
    if tail.take(4)? != b"RCBE" {
        return Err(BridgeError::NotATable("the end marker is missing"));
    }
    if tail.f64()? != body_len as f64 {
        return Err(BridgeError::NotATable("the length does not match"));
    }

    let mut r = Cursor::new(bytes.get(4..body_len).ok_or(BridgeError::Truncated)?);
    let ncol = r.count()?;
    let nrow = r.count()?;
    if ncol > MAX_COLUMNS {
        return Err(BridgeError::TooLarge("too many columns"));
    }
    let mut columns = Vec::new();
    for _ in 0..ncol {
        columns.push(read_column(&mut r, nrow)?);
    }
    if r.remaining() != 0 {
        return Err(BridgeError::Invalid(
            "bytes left over after the last column",
        ));
    }
    Ok(Table { nrow, columns })
}

fn read_column(r: &mut Cursor<'_>, nrow: usize) -> Result<Column, BridgeError> {
    let name = r.string()?;
    let class = r.string()?;
    let kind = r.i32()?;
    let (attrs, dropped_attrs) = read_attrs(r)?;
    let data = match kind {
        1 => Data::Int(r.i32s(nrow)?),
        2 => Data::Double(r.f64s(nrow)?),
        3 => Data::Logical(
            r.i32s(nrow)?
                .into_iter()
                .map(|x| if x < 0 { -1 } else { i8::from(x != 0) })
                .collect(),
        ),
        4 => {
            let ordered = r.i32()? != 0;
            let nlevels = r.count()?;
            let levels = r.text(nlevels)?;
            let codes = r.i32s(nrow)?;
            if codes.iter().any(|&c| c < -1 || c as i64 >= nlevels as i64) {
                return Err(BridgeError::Invalid("a factor code is out of range"));
            }
            Data::Factor {
                levels,
                codes,
                ordered,
            }
        }
        5 => Data::Date(r.f64s(nrow)?),
        6 => Data::Text(r.text(nrow)?),
        7 => {
            let tz = r.string()?;
            Data::DateTime {
                secs: r.f64s(nrow)?,
                tz,
            }
        }
        8 => {
            let units = r.string()?;
            Data::Duration {
                values: r.f64s(nrow)?,
                units,
            }
        }
        9 => Data::Other(r.text(nrow)?),
        _ => return Err(BridgeError::Invalid("an unknown column kind")),
    };
    Ok(Column {
        name,
        class,
        attrs,
        dropped_attrs,
        data,
    })
}

fn read_attrs(r: &mut Cursor<'_>) -> Result<(Vec<Attr>, u32), BridgeError> {
    let count = r.count()?;
    let dropped = u32::try_from(r.count()?).unwrap_or(u32::MAX);
    if count > r.remaining() {
        return Err(BridgeError::Truncated);
    }
    let mut attrs = Vec::new();
    for _ in 0..count {
        let key = r.string()?;
        let kind = r.i32()?;
        let len = r.count()?;
        let value = match kind {
            6 => AttrValue::Text(r.text(len)?),
            2 => AttrValue::Number(r.f64s(len)?),
            _ => return Err(BridgeError::Invalid("an unknown attribute kind")),
        };
        let names = if r.i32()? != 0 {
            Some(r.text(len)?)
        } else {
            None
        };
        attrs.push(Attr { key, value, names });
    }
    Ok((attrs, dropped))
}

/// A bounds-checked reader over a byte slice.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Cursor<'a> {
        Cursor { bytes, at: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.at)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], BridgeError> {
        let end = self.at.checked_add(n).ok_or(BridgeError::Truncated)?;
        let s = self.bytes.get(self.at..end).ok_or(BridgeError::Truncated)?;
        self.at = end;
        Ok(s)
    }

    fn i32(&mut self) -> Result<i32, BridgeError> {
        let b: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| BridgeError::Truncated)?;
        Ok(i32::from_le_bytes(b))
    }

    fn f64(&mut self) -> Result<f64, BridgeError> {
        let b: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| BridgeError::Truncated)?;
        Ok(f64::from_le_bytes(b))
    }

    /// A count: a non-negative `i32`.
    fn count(&mut self) -> Result<usize, BridgeError> {
        usize::try_from(self.i32()?).map_err(|_| BridgeError::Invalid("a negative count"))
    }

    fn i32s(&mut self, n: usize) -> Result<Vec<i32>, BridgeError> {
        let bytes = self.take(n.checked_mul(4).ok_or(BridgeError::Truncated)?)?;
        Ok(bytes
            .chunks_exact(4)
            .filter_map(|c| <[u8; 4]>::try_from(c).ok())
            .map(i32::from_le_bytes)
            .collect())
    }

    fn f64s(&mut self, n: usize) -> Result<Vec<f64>, BridgeError> {
        let bytes = self.take(n.checked_mul(8).ok_or(BridgeError::Truncated)?)?;
        Ok(bytes
            .chunks_exact(8)
            .filter_map(|c| <[u8; 8]>::try_from(c).ok())
            .map(f64::from_le_bytes)
            .collect())
    }

    fn string(&mut self) -> Result<String, BridgeError> {
        let n = self.count()?;
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }

    /// A text vector of `n` items.
    fn text(&mut self, n: usize) -> Result<Vec<Option<String>>, BridgeError> {
        let lens = self.i32s(n)?;
        let total = self.count()?;
        let blob = self.take(total)?;
        let mut at = 0usize;
        let mut out = Vec::with_capacity(lens.len());
        for len in lens {
            if len < 0 {
                out.push(None);
                continue;
            }
            let end = at
                .checked_add(len as usize)
                .ok_or(BridgeError::Invalid("a text length"))?;
            let piece = blob
                .get(at..end)
                .ok_or(BridgeError::Invalid("text lengths exceed the text"))?;
            out.push(Some(String::from_utf8_lossy(piece).into_owned()));
            at = end;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i4(out: &mut Vec<u8>, v: i32) {
        out.extend_from_slice(&v.to_le_bytes());
    }
    fn s(out: &mut Vec<u8>, t: &str) {
        i4(out, t.len() as i32);
        out.extend_from_slice(t.as_bytes());
    }
    fn text(out: &mut Vec<u8>, items: &[Option<&str>]) {
        let mut blob = Vec::new();
        for it in items {
            i4(out, it.map_or(-1, |t| t.len() as i32));
            if let Some(t) = it {
                blob.extend_from_slice(t.as_bytes());
            }
        }
        i4(out, blob.len() as i32);
        out.extend_from_slice(&blob);
    }
    fn finish(mut body: Vec<u8>) -> Vec<u8> {
        let len = body.len() as f64;
        body.extend_from_slice(b"RCBE");
        body.extend_from_slice(&len.to_le_bytes());
        body
    }

    /// Two rows: an integer with a label, a text column with a missing item.
    fn sample() -> Vec<u8> {
        let mut b = b"RCB1".to_vec();
        i4(&mut b, 2);
        i4(&mut b, 2);
        s(&mut b, "n");
        s(&mut b, "integer");
        i4(&mut b, 1);
        i4(&mut b, 1); // one attribute
        i4(&mut b, 0);
        s(&mut b, "label");
        i4(&mut b, 6);
        i4(&mut b, 1);
        text(&mut b, &[Some("Count")]);
        i4(&mut b, 0);
        i4(&mut b, 7);
        i4(&mut b, INT_NA);
        s(&mut b, "t");
        s(&mut b, "character");
        i4(&mut b, 6);
        i4(&mut b, 0);
        i4(&mut b, 0);
        text(&mut b, &[Some("é,\"x\"\n"), None]);
        finish(b)
    }

    #[test]
    fn a_small_table_reads_back() {
        let t = decode(&sample()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(t.nrow, 2);
        assert_eq!(t.columns.len(), 2);
        let n = &t.columns[0];
        assert_eq!(n.label(), Some("Count"));
        assert_eq!(n.data, Data::Int(vec![7, INT_NA]));
        assert_eq!(n.data.missing(), 1);
        assert_eq!(
            t.columns[1].data,
            Data::Text(vec![Some("é,\"x\"\n".to_string()), None])
        );
    }

    #[test]
    fn every_shorter_prefix_is_an_error_not_a_crash() {
        let full = sample();
        for cut in 0..full.len() {
            assert!(decode(&full[..cut]).is_err(), "prefix of {cut} bytes");
        }
    }

    #[test]
    fn flipped_bytes_never_crash() {
        let full = sample();
        for i in 0..full.len() {
            for v in [0u8, 0x7f, 0xff] {
                let mut b = full.clone();
                b[i] = v;
                let _ = decode(&b);
            }
        }
    }

    #[test]
    fn hostile_counts_do_not_allocate() {
        let mut b = b"RCB1".to_vec();
        i4(&mut b, 1);
        i4(&mut b, i32::MAX); // two billion rows, no data
        s(&mut b, "x");
        s(&mut b, "numeric");
        i4(&mut b, 2);
        i4(&mut b, 0);
        i4(&mut b, 0);
        assert_eq!(decode(&finish(b)), Err(BridgeError::Truncated));
        let mut b = b"RCB1".to_vec();
        i4(&mut b, i32::MAX);
        i4(&mut b, 0);
        assert!(decode(&finish(b)).is_err());
        let mut b = b"RCB1".to_vec();
        i4(&mut b, -1);
        i4(&mut b, 0);
        assert!(decode(&finish(b)).is_err());
    }

    #[test]
    fn the_end_marker_and_length_are_checked() {
        let mut b = sample();
        let n = b.len();
        b[n - 12] = b'X';
        assert!(matches!(decode(&b), Err(BridgeError::NotATable(_))));
        let mut b = sample();
        b.insert(10, 0); // extra byte: the recorded length no longer matches
        assert!(matches!(decode(&b), Err(BridgeError::NotATable(_))));
        assert!(matches!(decode(b"nope"), Err(BridgeError::NotATable(_))));
    }

    #[test]
    fn factor_codes_are_range_checked() {
        let mut b = b"RCB1".to_vec();
        i4(&mut b, 1);
        i4(&mut b, 1);
        s(&mut b, "g");
        s(&mut b, "factor");
        i4(&mut b, 4);
        i4(&mut b, 0);
        i4(&mut b, 0);
        i4(&mut b, 0); // not ordered
        i4(&mut b, 1);
        text(&mut b, &[Some("a")]);
        i4(&mut b, 5); // code 5 with one level
        assert!(matches!(decode(&finish(b)), Err(BridgeError::Invalid(_))));
    }

    #[test]
    fn r_missing_is_not_nan() {
        let na = f64::from_bits(0x7FF0_0000_0000_07A2);
        assert!(is_na(na));
        assert!(!is_na(f64::NAN));
        assert!(!is_na(1.0));
    }

    #[test]
    fn r_strings_are_escaped() {
        assert_eq!(r_string("C:\\a\"b\n"), "\"C:\\\\a\\\"b\\n\"");
    }

    #[test]
    fn the_writer_code_is_one_expression() {
        let code = write_code("mtcars", "C:/t/x.rcb");
        assert!(code.starts_with("invisible((function(df, path"));
        assert!(code.contains("mtcars, \"C:/t/x.rcb\""));
    }
}
