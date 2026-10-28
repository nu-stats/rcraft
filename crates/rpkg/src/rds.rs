//! A reader for R's `saveRDS` files, limited to what the CRAN package
//! database needs: text, number and logical vectors, lists, pairlists (the
//! attributes) and symbols, in the XDR format, versions 2 and 3, plain or
//! gzip. Anything else (environments, functions, other compressions) is an
//! [`Error::Unsupported`], never a guess.
//!
//! The reader never trusts the file: every length is checked against the bytes
//! that are actually left before anything is allocated, nesting is bounded,
//! and the total number of elements is capped.

use crate::unpack::{unpack, MAX_UNPACKED_BYTES};
use crate::{Error, Result};

/// Deepest nesting of lists accepted.
pub const MAX_DEPTH: usize = 64;
/// Most elements (vector items and list entries) accepted in one file.
pub const MAX_ELEMENTS: usize = 200_000_000;

/// One R value with its attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub data: Data,
    pub attrs: Vec<(String, Node)>,
}

/// What a value holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    /// `NULL`.
    Null,
    /// A text vector; `None` is R's `NA_character_`.
    Strings(Vec<Option<String>>),
    /// Integers and logicals (`i32::MIN` is R's missing).
    Ints(Vec<i32>),
    /// Numbers.
    Reals(Vec<f64>),
    /// A list.
    List(Vec<Node>),
    /// A pairlist: entries with their tag (empty when untagged).
    Pairlist(Vec<(String, Node)>),
    /// A symbol (a name).
    Symbol(String),
    /// Something read past but not kept (raw, complex, unknown ALTREP).
    Other,
}

impl Node {
    fn new(data: Data) -> Node {
        Node {
            data,
            attrs: Vec::new(),
        }
    }

    /// The attribute called `name`.
    pub fn attr(&self, name: &str) -> Option<&Node> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }
}

/// Reads an R object from the bytes of an `.rds` file.
pub fn read(bytes: &[u8]) -> Result<Node> {
    let raw = unpack(bytes, MAX_UNPACKED_BYTES)?;
    let mut r = Reader {
        buf: &raw,
        pos: 0,
        symbols: Vec::new(),
        budget: MAX_ELEMENTS,
        latin1: false,
    };
    r.header()?;
    r.item(0)
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
    /// Symbols seen so far, for back-references (1-based in the file).
    symbols: Vec<String>,
    /// Elements still allowed.
    budget: usize,
    /// Whether text with no declared encoding is Latin-1 rather than UTF-8.
    latin1: bool,
}

// type codes (R's `Rinternals.h`)
const SYMSXP: u8 = 1;
const LISTSXP: u8 = 2;
const CHARSXP: u8 = 9;
const LGLSXP: u8 = 10;
const INTSXP: u8 = 13;
const REALSXP: u8 = 14;
const CPLXSXP: u8 = 15;
const STRSXP: u8 = 16;
const VECSXP: u8 = 19;
const EXPRSXP: u8 = 20;
const RAWSXP: u8 = 24;
const ALTREP_SXP: u8 = 238;
const NILVALUE_SXP: u8 = 254;
const REFSXP: u8 = 255;

// bits in a CHARSXP's flags (the `gp` field, shifted by 12 in the file)
const GP_BYTES: u32 = 1 << 1;
const GP_LATIN1: u32 = 1 << 2;
const GP_UTF8: u32 = 1 << 3;

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated)?;
        let bytes = self.buf.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(bytes)
    }

    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    fn int(&mut self) -> Result<i32> {
        let b: [u8; 4] = self.take(4)?.try_into().map_err(|_| Error::Truncated)?;
        Ok(i32::from_be_bytes(b))
    }

    fn real(&mut self) -> Result<f64> {
        let b: [u8; 8] = self.take(8)?.try_into().map_err(|_| Error::Truncated)?;
        Ok(f64::from_be_bytes(b))
    }

    /// A vector length, checked against the bytes left (`min_each` bytes at
    /// least per item) and the element budget.
    fn length(&mut self, min_each: usize) -> Result<usize> {
        let n = self.int()?;
        if n == -1 {
            return Err(Error::Unsupported(
                "a vector with more than 2 billion items".into(),
            ));
        }
        let n = usize::try_from(n).map_err(|_| Error::Format("a negative length".into()))?;
        let needed = n
            .checked_mul(min_each)
            .ok_or_else(|| Error::TooBig("a vector length".into()))?;
        if needed > self.remaining() {
            return Err(Error::Truncated);
        }
        self.budget = self
            .budget
            .checked_sub(n)
            .ok_or_else(|| Error::TooBig(format!("more than {MAX_ELEMENTS} elements")))?;
        Ok(n)
    }

    fn header(&mut self) -> Result<()> {
        let magic = self.take(2)?;
        match magic {
            b"X\n" => {}
            b"A\n" => return Err(Error::Unsupported("the text (ascii) format".into())),
            b"B\n" => return Err(Error::Unsupported("the native binary format".into())),
            _ => return Err(Error::Format("not an R serialization file".into())),
        }
        let version = self.int()?;
        let _writer = self.int()?;
        let _min_reader = self.int()?;
        match version {
            2 => {}
            3 => {
                let n = self.int()?;
                let n =
                    usize::try_from(n).map_err(|_| Error::Format("a negative length".into()))?;
                let enc = self.take(n)?;
                let enc = String::from_utf8_lossy(enc).to_ascii_lowercase();
                self.latin1 =
                    enc.contains("latin1") || enc.contains("8859-1") || enc.contains("cp1252");
            }
            v => return Err(Error::Unsupported(format!("serialization version {v}"))),
        }
        Ok(())
    }

    fn item(&mut self, depth: usize) -> Result<Node> {
        if depth > MAX_DEPTH {
            return Err(Error::TooBig(format!("nesting deeper than {MAX_DEPTH}")));
        }
        let flags = self.int()? as u32;
        self.item_with(flags, depth)
    }

    fn item_with(&mut self, flags: u32, depth: usize) -> Result<Node> {
        let ty = (flags & 0xff) as u8;
        let has_attr = flags & 0x200 != 0;
        match ty {
            NILVALUE_SXP => Ok(Node::new(Data::Null)),
            REFSXP => {
                let mut idx = (flags >> 8) as usize;
                if idx == 0 {
                    idx = usize::try_from(self.int()?)
                        .map_err(|_| Error::Format("a negative reference".into()))?;
                }
                let name = idx
                    .checked_sub(1)
                    .and_then(|i| self.symbols.get(i))
                    .ok_or_else(|| Error::Format("a reference to an unknown object".into()))?;
                Ok(Node::new(Data::Symbol(name.clone())))
            }
            SYMSXP => {
                let name = self.symbol_name()?;
                self.symbols.push(name.clone());
                Ok(Node::new(Data::Symbol(name)))
            }
            CHARSXP => {
                let s = self.charsxp(flags)?;
                Ok(Node::new(Data::Strings(vec![s])))
            }
            LGLSXP | INTSXP => {
                let n = self.length(4)?;
                let mut v = Vec::with_capacity(n);
                for _ in 0..n {
                    v.push(self.int()?);
                }
                self.finish(Data::Ints(v), has_attr, depth)
            }
            REALSXP => {
                let n = self.length(8)?;
                let mut v = Vec::with_capacity(n);
                for _ in 0..n {
                    v.push(self.real()?);
                }
                self.finish(Data::Reals(v), has_attr, depth)
            }
            CPLXSXP => {
                let n = self.length(16)?;
                self.take(n * 16)?;
                self.finish(Data::Other, has_attr, depth)
            }
            RAWSXP => {
                let n = self.length(1)?;
                self.take(n)?;
                self.finish(Data::Other, has_attr, depth)
            }
            STRSXP => {
                let n = self.length(4)?;
                let mut v = Vec::with_capacity(n);
                for _ in 0..n {
                    let f = self.int()? as u32;
                    if (f & 0xff) as u8 != CHARSXP {
                        return Err(Error::Format("a text vector holds something else".into()));
                    }
                    v.push(self.charsxp(f)?);
                }
                self.finish(Data::Strings(v), has_attr, depth)
            }
            VECSXP | EXPRSXP => {
                let n = self.length(4)?;
                let mut v = Vec::with_capacity(n);
                for _ in 0..n {
                    v.push(self.item(depth + 1)?);
                }
                self.finish(Data::List(v), has_attr, depth)
            }
            LISTSXP => self.pairlist(flags, depth),
            ALTREP_SXP => self.altrep(depth),
            other => Err(Error::Unsupported(format!("R object type {other}"))),
        }
    }

    /// The attributes after a vector's payload, when it has any.
    fn finish(&mut self, data: Data, has_attr: bool, depth: usize) -> Result<Node> {
        let mut node = Node::new(data);
        if has_attr {
            node.attrs = self.attributes(depth)?;
        }
        Ok(node)
    }

    fn attributes(&mut self, depth: usize) -> Result<Vec<(String, Node)>> {
        let list = self.item(depth + 1)?;
        match list.data {
            Data::Pairlist(v) => Ok(v),
            Data::Null => Ok(Vec::new()),
            _ => Err(Error::Format("attributes that are not a pairlist".into())),
        }
    }

    fn symbol_name(&mut self) -> Result<String> {
        let f = self.int()? as u32;
        if (f & 0xff) as u8 != CHARSXP {
            return Err(Error::Format("a symbol without a name".into()));
        }
        Ok(self.charsxp(f)?.unwrap_or_default())
    }

    /// A pairlist, followed along its chain with a loop, not recursion.
    fn pairlist(&mut self, first: u32, depth: usize) -> Result<Node> {
        let mut entries = Vec::new();
        let mut flags = first;
        loop {
            let ty = (flags & 0xff) as u8;
            if ty == NILVALUE_SXP {
                break;
            }
            if ty != LISTSXP {
                return Err(Error::Format("a pairlist that does not end".into()));
            }
            self.budget = self
                .budget
                .checked_sub(1)
                .ok_or_else(|| Error::TooBig(format!("more than {MAX_ELEMENTS} elements")))?;
            if flags & 0x200 != 0 {
                // attributes on a list cell: read past
                self.item(depth + 1)?;
            }
            let tag = if flags & 0x400 != 0 {
                match self.item(depth + 1)?.data {
                    Data::Symbol(s) => s,
                    _ => return Err(Error::Format("a tag that is not a name".into())),
                }
            } else {
                String::new()
            };
            let value = self.item(depth + 1)?;
            entries.push((tag, value));
            flags = self.int()? as u32;
        }
        Ok(Node::new(Data::Pairlist(entries)))
    }

    /// ALTREP objects (compact sequences and wrapped vectors are written this
    /// way by R 3.5 and later). The two sequence kinds and the wrappers are
    /// expanded; anything else is read past and reported as [`Data::Other`].
    fn altrep(&mut self, depth: usize) -> Result<Node> {
        let info = self.item(depth + 1)?;
        let state = self.item(depth + 1)?;
        let attr = self.item(depth + 1)?;
        let class = match &info.data {
            Data::Pairlist(v) => match v.first() {
                Some((
                    _,
                    Node {
                        data: Data::Symbol(s),
                        ..
                    },
                )) => s.clone(),
                _ => String::new(),
            },
            _ => String::new(),
        };
        let data = match (class.as_str(), state.data) {
            ("compact_intseq", Data::Reals(v)) => match v.as_slice() {
                [n, start, step] => Data::Ints(sequence(*n, *start, *step, &mut self.budget)?),
                _ => Data::Other,
            },
            ("compact_realseq", Data::Reals(v)) => match v.as_slice() {
                [n, start, step] => {
                    let ints = sequence(*n, *start, *step, &mut self.budget)?;
                    Data::Reals(ints.into_iter().map(f64::from).collect())
                }
                _ => Data::Other,
            },
            (c, Data::List(mut v)) if c.starts_with("wrap_") && !v.is_empty() => {
                // wrapper state is list(x, meta): the vector is x
                let x = v.remove(0);
                let mut node = Node::new(x.data);
                node.attrs = x.attrs;
                if let Data::Pairlist(a) = attr.data {
                    node.attrs.extend(a);
                }
                return Ok(node);
            }
            _ => Data::Other,
        };
        let mut node = Node::new(data);
        if let Data::Pairlist(a) = attr.data {
            node.attrs = a;
        }
        Ok(node)
    }

    fn charsxp(&mut self, flags: u32) -> Result<Option<String>> {
        let n = self.int()?;
        if n == -1 {
            return Ok(None);
        }
        let n = usize::try_from(n).map_err(|_| Error::Format("a negative length".into()))?;
        let bytes = self.take(n)?;
        let gp = flags >> 12;
        // text marked UTF-8 (or raw bytes) is read as UTF-8; text marked
        // Latin-1, or unmarked in a Latin-1 session, is read as Latin-1
        let utf8 = gp & (GP_BYTES | GP_UTF8) != 0;
        let text = if !utf8 && (gp & GP_LATIN1 != 0 || self.latin1) {
            bytes.iter().map(|&b| char::from(b)).collect()
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        };
        Ok(Some(text))
    }
}

/// The integers of a compact sequence: `n` items from `start` by `step`.
fn sequence(n: f64, start: f64, step: f64, budget: &mut usize) -> Result<Vec<i32>> {
    if !(n.is_finite() && (0.0..=2_147_483_647.0).contains(&n)) {
        return Err(Error::Format("a sequence length".into()));
    }
    let n = n as usize;
    *budget = budget
        .checked_sub(n)
        .ok_or_else(|| Error::TooBig(format!("more than {MAX_ELEMENTS} elements")))?;
    let mut out = Vec::with_capacity(n.min(1 << 20));
    let mut v = start;
    for _ in 0..n {
        if !(v.is_finite() && v.abs() <= f64::from(i32::MAX)) {
            return Err(Error::Format("a sequence value".into()));
        }
        out.push(v as i32);
        v += step;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Header of a version 2 file.
    fn head() -> Vec<u8> {
        let mut v = b"X\n".to_vec();
        v.extend(2i32.to_be_bytes());
        v.extend(0x00040403i32.to_be_bytes());
        v.extend(0x00020300i32.to_be_bytes());
        v
    }

    fn int(v: &mut Vec<u8>, n: i32) {
        v.extend(n.to_be_bytes());
    }

    fn chars(v: &mut Vec<u8>, s: &str) {
        int(v, 9 | (1 << 15)); // CHARSXP, UTF-8 flag in the gp bits
        int(v, s.len() as i32);
        v.extend(s.as_bytes());
    }

    #[test]
    fn a_text_vector_is_read_with_its_missing_values() {
        let mut v = head();
        int(&mut v, 16);
        int(&mut v, 3);
        chars(&mut v, "a");
        int(&mut v, 9);
        int(&mut v, -1);
        chars(&mut v, "é");
        let node = read(&v);
        assert_eq!(
            node.map(|n| n.data),
            Ok(Data::Strings(vec![
                Some("a".into()),
                None,
                Some("é".into())
            ]))
        );
    }

    #[test]
    fn attributes_follow_the_payload() {
        let mut v = head();
        int(&mut v, 13 | 0x200); // INTSXP with attributes
        int(&mut v, 2);
        int(&mut v, 7);
        int(&mut v, 8);
        // attribute pairlist: tag "dim" = c(1L)
        int(&mut v, 2 | 0x400);
        int(&mut v, 1); // SYMSXP
        chars(&mut v, "dim");
        int(&mut v, 13);
        int(&mut v, 1);
        int(&mut v, 1);
        int(&mut v, 254);
        let node = read(&v).ok();
        assert_eq!(
            node.as_ref().map(|n| n.data.clone()),
            Some(Data::Ints(vec![7, 8]))
        );
        assert_eq!(
            node.as_ref()
                .and_then(|n| n.attr("dim"))
                .map(|d| d.data.clone()),
            Some(Data::Ints(vec![1]))
        );
    }

    #[test]
    fn a_length_longer_than_the_file_is_an_error_not_an_allocation() {
        let mut v = head();
        int(&mut v, 13);
        int(&mut v, 2_000_000_000);
        assert_eq!(read(&v), Err(Error::Truncated));
        let mut v = head();
        int(&mut v, 16);
        int(&mut v, i32::MAX);
        assert_eq!(read(&v), Err(Error::Truncated));
    }

    #[test]
    fn other_formats_and_objects_are_refused_by_name() {
        assert!(matches!(read(b"A\n\0\0\0\x02"), Err(Error::Unsupported(_))));
        assert!(matches!(read(b"hello world"), Err(Error::Format(_))));
        let mut v = head();
        int(&mut v, 4); // ENVSXP
        assert!(matches!(read(&v), Err(Error::Unsupported(_))));
        let mut v = b"X\n".to_vec();
        int(&mut v, 4);
        int(&mut v, 0);
        int(&mut v, 0);
        assert!(matches!(read(&v), Err(Error::Unsupported(_))));
    }

    #[test]
    fn deep_nesting_is_refused() {
        let mut v = head();
        for _ in 0..(MAX_DEPTH + 5) {
            int(&mut v, 19); // a list...
            int(&mut v, 1); // ...of one list...
        }
        int(&mut v, 254);
        assert!(matches!(read(&v), Err(Error::TooBig(_))));
    }

    #[test]
    fn a_compact_sequence_is_expanded() {
        let mut v = head();
        int(&mut v, 238);
        // info pairlist: class symbol, package symbol, type integer
        int(&mut v, 2);
        int(&mut v, 1);
        chars(&mut v, "compact_intseq");
        int(&mut v, 2);
        int(&mut v, 1);
        chars(&mut v, "base");
        int(&mut v, 2);
        int(&mut v, 13);
        int(&mut v, 1);
        int(&mut v, 13);
        int(&mut v, 254);
        // state: c(n, start, step)
        int(&mut v, 14);
        int(&mut v, 3);
        for x in [4.0f64, 1.0, 1.0] {
            v.extend(x.to_be_bytes());
        }
        int(&mut v, 254); // no attributes
        assert_eq!(read(&v).map(|n| n.data), Ok(Data::Ints(vec![1, 2, 3, 4])));
    }

    #[test]
    fn every_truncation_of_a_valid_file_is_an_error() {
        let mut v = head();
        int(&mut v, 19);
        int(&mut v, 2);
        int(&mut v, 16);
        int(&mut v, 1);
        chars(&mut v, "abc");
        int(&mut v, 13);
        int(&mut v, 1);
        int(&mut v, 5);
        assert!(read(&v).is_ok());
        for n in 0..v.len() {
            assert!(
                read(v.get(..n).unwrap_or(&[])).is_err(),
                "prefix of {n} bytes"
            );
        }
    }
}
