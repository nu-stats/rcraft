//! Looks at the start of a text data file and guesses how to read it: the
//! text encoding, the delimiter, whether the first row is a header, and the
//! decimal mark. The guesses only fill in the dialog's choices; the user can
//! change every one, and R does the real reading.

use std::fmt;
use std::io::Read;
use std::path::Path;

/// How much of the file is looked at.
pub const HEAD_BYTES: usize = 256 * 1024;
/// Lines kept for the raw preview.
pub const PREVIEW_LINES: usize = 25;
/// Longest preview line, in characters.
pub const PREVIEW_WIDTH: usize = 400;
/// Records examined to judge the delimiter and header.
const SAMPLE_RECORDS: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    /// Anything else one byte per character (Windows-1252 and relatives).
    Latin1,
}

impl Encoding {
    /// The name R's `fileEncoding` argument takes.
    pub fn r_name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8-BOM",
            Encoding::Utf16Le => "UTF-16LE",
            Encoding::Utf16Be => "UTF-16BE",
            Encoding::Latin1 => "latin1",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8 with byte order mark",
            Encoding::Utf16Le => "UTF-16 (little-endian)",
            Encoding::Utf16Be => "UTF-16 (big-endian)",
            Encoding::Latin1 => "Latin-1 / Windows-1252",
        }
    }

    pub const ALL: [Encoding; 5] = [
        Encoding::Utf8,
        Encoding::Utf8Bom,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Latin1,
    ];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SniffError {
    Io(String),
    Empty,
    /// Not a text file; says what it looks like.
    NotText(&'static str),
}

impl fmt::Display for SniffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SniffError::Io(e) => write!(f, "could not read the file: {e}"),
            SniffError::Empty => write!(f, "the file is empty"),
            SniffError::NotText(what) => write!(f, "this is not a text file ({what})"),
        }
    }
}

impl std::error::Error for SniffError {}

/// What the start of a file looks like.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sniffed {
    pub encoding: Encoding,
    /// `None` when the file looks like a single column.
    pub delimiter: Option<char>,
    pub header: bool,
    pub decimal: char,
    /// Fields per row, as most rows have them.
    pub columns: usize,
    /// The first lines as text, for the raw preview.
    pub preview: Vec<String>,
    /// The file goes on past what was looked at.
    pub more: bool,
    pub bytes: u64,
}

/// Looks at the file at `path`.
pub fn sniff_file(path: &Path) -> Result<Sniffed, SniffError> {
    let io = |e: std::io::Error| SniffError::Io(e.to_string());
    let file = std::fs::File::open(path).map_err(io)?;
    let bytes = file.metadata().map_err(io)?.len();
    let mut head = Vec::new();
    file.take(HEAD_BYTES as u64 + 1)
        .read_to_end(&mut head)
        .map_err(io)?;
    sniff_bytes(&head, bytes)
}

/// Looks at the start of a file, given as bytes; `total` is the file's size.
pub fn sniff_bytes(head: &[u8], total: u64) -> Result<Sniffed, SniffError> {
    if head.is_empty() {
        return Err(SniffError::Empty);
    }
    let more = head.len() > HEAD_BYTES || total > head.len() as u64;
    let head = head.get(..HEAD_BYTES.min(head.len())).unwrap_or(head);
    if head.starts_with(b"PK\x03\x04") {
        return Err(SniffError::NotText("a zip archive, such as .xlsx"));
    }
    if head.starts_with(b"%PDF") {
        return Err(SniffError::NotText("a PDF"));
    }
    if head.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        return Err(SniffError::NotText("an old Excel file"));
    }
    let (encoding, mut text) = decode(head)?;
    if more {
        // the last line may have been cut in the middle
        if let Some(at) = text.rfind('\n') {
            text.truncate(at + 1);
        }
    }
    let text = text.trim_start_matches('\u{feff}');
    let preview = text
        .lines()
        .take(PREVIEW_LINES)
        .map(|l| l.chars().take(PREVIEW_WIDTH).collect::<String>())
        .collect();

    let delimiter = pick_delimiter(text);
    let records = records(text, delimiter.unwrap_or('\u{0}'), SAMPLE_RECORDS);
    let decimal = pick_decimal(&records, delimiter);
    let header = looks_like_header(&records, decimal);
    let columns = modal_count(&records).unwrap_or(1);
    Ok(Sniffed {
        encoding,
        delimiter,
        header,
        decimal,
        columns,
        preview,
        more,
        bytes: total,
    })
}

fn decode(head: &[u8]) -> Result<(Encoding, String), SniffError> {
    if let Some(rest) = head.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return Ok((
            Encoding::Utf8Bom,
            String::from_utf8_lossy(rest).into_owned(),
        ));
    }
    if let Some(rest) = head.strip_prefix(&[0xFF, 0xFE]) {
        return Ok((Encoding::Utf16Le, utf16(rest, u16::from_le_bytes)));
    }
    if let Some(rest) = head.strip_prefix(&[0xFE, 0xFF]) {
        return Ok((Encoding::Utf16Be, utf16(rest, u16::from_be_bytes)));
    }
    if head.contains(&0) {
        return Err(SniffError::NotText("it contains binary data"));
    }
    match std::str::from_utf8(head) {
        Ok(s) => Ok((Encoding::Utf8, s.to_string())),
        // cut in the middle of a character at the end of the sample
        Err(e) if e.error_len().is_none() => Ok((
            Encoding::Utf8,
            String::from_utf8_lossy(head.get(..e.valid_up_to()).unwrap_or(head)).into_owned(),
        )),
        Err(_) => Ok((
            Encoding::Latin1,
            head.iter().map(|&b| char::from(b)).collect(),
        )),
    }
}

fn utf16(bytes: &[u8], make: fn([u8; 2]) -> u16) -> String {
    let units = bytes
        .chunks_exact(2)
        .filter_map(|c| <[u8; 2]>::try_from(c).ok())
        .map(make);
    char::decode_utf16(units)
        .map(|r| r.unwrap_or('\u{fffd}'))
        .collect()
}

/// Splits `text` into records of fields. Quotes (`"`) protect delimiters and
/// line breaks; a doubled quote is a quote. Blank lines are skipped. Stops
/// after `limit` records.
pub fn records(text: &str, delimiter: char, limit: usize) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if out.len() >= limit {
            break;
        }
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' && field.is_empty() {
            quoted = true;
        } else if c == delimiter {
            row.push(std::mem::take(&mut field));
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            if !row.is_empty() || !field.is_empty() {
                row.push(std::mem::take(&mut field));
                out.push(std::mem::take(&mut row));
            }
        } else {
            field.push(c);
        }
    }
    if out.len() < limit && (!row.is_empty() || !field.is_empty()) {
        row.push(field);
        out.push(row);
    }
    out
}

fn modal_count(records: &[Vec<String>]) -> Option<usize> {
    let mut counts: Vec<(usize, usize)> = Vec::new();
    for r in records {
        match counts.iter_mut().find(|(n, _)| *n == r.len()) {
            Some((_, times)) => *times += 1,
            None => counts.push((r.len(), 1)),
        }
    }
    counts
        .into_iter()
        .max_by_key(|&(n, times)| (times, n))
        .map(|(n, _)| n)
}

/// The delimiter that splits most rows into the same number (at least two)
/// of fields, or `None` for a single column.
fn pick_delimiter(text: &str) -> Option<char> {
    // in order of preference when two are equally good
    const CANDIDATES: [char; 5] = ['\t', ',', ';', '|', ' '];
    let mut best: Option<(char, f64, usize)> = None;
    for d in CANDIDATES {
        let recs = records(text, d, SAMPLE_RECORDS);
        let Some(modal) = modal_count(&recs) else {
            continue;
        };
        if modal < 2 || recs.is_empty() {
            continue;
        }
        let consistency =
            recs.iter().filter(|r| r.len() == modal).count() as f64 / recs.len() as f64;
        if consistency < 0.8 {
            continue;
        }
        // spaces are common inside text: use them only when nothing else fits
        let weight = if d == ' ' {
            consistency * 0.5
        } else {
            consistency
        };
        let better = match best {
            None => true,
            Some((_, w, m)) => weight > w + 1e-9 || ((weight - w).abs() <= 1e-9 && modal > m),
        };
        if better {
            best = Some((d, weight, modal));
        }
    }
    best.map(|(d, _, _)| d)
}

fn is_number(s: &str, decimal: char) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    let t: String = if decimal == ',' {
        t.replace(',', ".")
    } else {
        t.to_string()
    };
    t.parse::<f64>().is_ok_and(|x| x.is_finite())
}

fn pick_decimal(records: &[Vec<String>], delimiter: Option<char>) -> char {
    if delimiter == Some(',') {
        return '.';
    }
    let (mut comma, mut dot) = (0usize, 0usize);
    for field in records.iter().flatten() {
        let t = field.trim();
        let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
        let t = t.strip_prefix('-').unwrap_or(t);
        if let Some((a, b)) = t.split_once(',') {
            if digits(a) && digits(b) {
                comma += 1;
            }
        } else if let Some((a, b)) = t.split_once('.') {
            if digits(a) && digits(b) {
                dot += 1;
            }
        }
    }
    if comma > dot {
        ','
    } else {
        '.'
    }
}

/// The first row is a header when none of its fields is a number.
fn looks_like_header(records: &[Vec<String>], decimal: char) -> bool {
    match records.first() {
        Some(first) => !first.iter().any(|f| is_number(f, decimal)),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sniff(text: &str) -> Sniffed {
        sniff_bytes(text.as_bytes(), text.len() as u64).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn comma_separated_with_header() {
        let s = sniff("id,name,score\n1,Ann,3.5\n2,Bo,4\n3,Cy,\n");
        assert_eq!(s.delimiter, Some(','));
        assert!(s.header);
        assert_eq!(s.decimal, '.');
        assert_eq!(s.columns, 3);
        assert_eq!(s.encoding, Encoding::Utf8);
        assert_eq!(s.preview.len(), 4);
        assert!(!s.more);
    }

    #[test]
    fn semicolons_and_decimal_commas() {
        let s = sniff("id;score;city\n1;3,5;Köln\n2;4,25;Bonn\n");
        assert_eq!(s.delimiter, Some(';'));
        assert_eq!(s.decimal, ',');
        assert!(s.header);
    }

    #[test]
    fn tabs_win_over_commas_inside_text() {
        let s = sniff("a\tb\tc\n1\tx, y\t3\n4\tz, w\t6\n");
        assert_eq!(s.delimiter, Some('\t'));
        assert_eq!(s.columns, 3);
    }

    #[test]
    fn pipes_and_spaces() {
        assert_eq!(sniff("a|b\n1|2\n3|4\n").delimiter, Some('|'));
        assert_eq!(sniff("a b c\n1 2 3\n4 5 6\n").delimiter, Some(' '));
    }

    #[test]
    fn quotes_protect_delimiters_and_line_breaks() {
        let s = sniff("id,note\n1,\"a, b\"\n2,\"line\nbreak\"\n3,\"say \"\"hi\"\"\"\n");
        assert_eq!(s.delimiter, Some(','));
        assert_eq!(s.columns, 2);
        let r = records("id,note\n3,\"say \"\"hi\"\"\"\n", ',', 10);
        assert_eq!(r[1], vec!["3", "say \"hi\""]);
    }

    #[test]
    fn a_file_of_numbers_has_no_header() {
        let s = sniff("1,2,3\n4,5,6\n");
        assert!(!s.header);
        let s = sniff("1;2,5;3\n4;5,5;6\n");
        assert!(!s.header, "decimal commas are numbers too");
    }

    #[test]
    fn a_single_column_has_no_delimiter() {
        let s = sniff("name\nAnn\nBo\n");
        assert_eq!(s.delimiter, None);
        assert_eq!(s.columns, 1);
        assert!(s.header);
    }

    #[test]
    fn ragged_files_are_not_split() {
        // commas that are not delimiters: prose
        let s = sniff("Hello, world\nNo commas here\nOne, two, three, four\nplain\n");
        assert_eq!(s.delimiter, None);
    }

    #[test]
    fn encodings_are_recognised() {
        let mut bom = vec![0xEF, 0xBB, 0xBF];
        bom.extend_from_slice(b"a,b\n1,2\n");
        let s = sniff_bytes(&bom, bom.len() as u64).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s.encoding, Encoding::Utf8Bom);
        assert_eq!(s.preview.first().map(String::as_str), Some("a,b"));

        let mut u16le = vec![0xFF, 0xFE];
        for u in "a,b\n1,2\n".encode_utf16() {
            u16le.extend_from_slice(&u.to_le_bytes());
        }
        let s = sniff_bytes(&u16le, u16le.len() as u64).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s.encoding, Encoding::Utf16Le);
        assert_eq!(s.delimiter, Some(','));

        let latin = b"name,city\nJos\xe9,K\xf6ln\nAna,Bonn\n";
        let s = sniff_bytes(latin, latin.len() as u64).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s.encoding, Encoding::Latin1);
        assert!(s.preview.iter().any(|l| l.contains("Jos\u{e9}")));
    }

    #[test]
    fn binary_and_empty_files_are_refused_with_a_reason() {
        assert_eq!(sniff_bytes(b"", 0), Err(SniffError::Empty));
        assert!(matches!(
            sniff_bytes(b"PK\x03\x04rest", 100),
            Err(SniffError::NotText(_))
        ));
        assert!(matches!(
            sniff_bytes(b"ab\0cd,ef\n", 9),
            Err(SniffError::NotText(_))
        ));
    }

    #[test]
    fn a_long_file_is_flagged_and_the_cut_line_dropped() {
        let line = "1,2,3\n";
        let mut data = String::new();
        while data.len() <= HEAD_BYTES + 100 {
            data.push_str(line);
        }
        let s = sniff_bytes(data.as_bytes(), data.len() as u64).unwrap_or_else(|e| panic!("{e}"));
        assert!(s.more);
        assert_eq!(s.columns, 3);
    }

    #[test]
    fn nothing_crashes_on_hostile_bytes() {
        let base = b"a,b,\"c\n1,2,3\n\xff\xfe\"\"\r\r\n,,,\n";
        for i in 0..base.len() {
            for v in [0u8, 0x22, 0x2c, 0xff, b'\n'] {
                let mut b = base.to_vec();
                if let Some(slot) = b.get_mut(i) {
                    *slot = v;
                }
                let _ = sniff_bytes(&b, b.len() as u64);
            }
        }
        for cut in 0..base.len() {
            let _ = sniff_bytes(base.get(..cut).unwrap_or_default(), base.len() as u64);
        }
        let _ = records("\"\"\"\"\"", ',', 5);
        let _ = records("", ',', 5);
    }

    #[test]
    fn files_come_from_disk() {
        let dir = std::env::temp_dir().join(format!("rcraft-sniff-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.csv");
        let _ = std::fs::write(&path, "a,b\n1,2\n");
        let s = sniff_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(s.is_ok_and(|s| s.delimiter == Some(',') && s.bytes == 8));
        assert!(matches!(
            sniff_file(Path::new("definitely/not/here.csv")),
            Err(SniffError::Io(_))
        ));
    }
}
