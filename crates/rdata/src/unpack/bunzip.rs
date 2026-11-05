//! A bzip2 decoder, written from the published description of the format and
//! ported from Husky Explorer's (our own) JavaScript one: Huffman-coded
//! move-to-front symbols with run lengths, an inverse Burrows-Wheeler
//! transform, then a final run-length step. Every block checksum and the
//! whole-stream checksum are checked. Several streams one after another are
//! read as one, as bzip2 itself does.

use super::{damaged, Budget, UnpackError};

struct BitReader<'a> {
    bytes: &'a [u8],
    pos: usize,
    buf: u32,
    count: u32,
}

impl BitReader<'_> {
    /// Up to 24 bits, most significant first.
    fn bits(&mut self, n: u32) -> Result<u32, UnpackError> {
        while self.count < n {
            let byte = self
                .bytes
                .get(self.pos)
                .ok_or_else(|| damaged("bzip2: it ends too soon"))?;
            self.pos += 1;
            self.buf = (self.buf << 8) | u32::from(*byte);
            self.count += 8;
        }
        let v = (self.buf >> (self.count - n)) & ((1u32 << n) - 1);
        self.count -= n;
        self.buf &= (1u32 << self.count) - 1;
        Ok(v)
    }

    fn bit(&mut self) -> Result<bool, UnpackError> {
        Ok(self.bits(1)? == 1)
    }

    /// Skips to the next whole byte (streams end on a byte boundary).
    fn align(&mut self) {
        self.count -= self.count % 8;
        self.buf &= (1u32 << self.count) - 1;
    }

    fn at_end(&self) -> bool {
        self.pos >= self.bytes.len() && self.count < 8
    }
}

/// bzip2's checksum: CRC-32 with the 0x04c11db7 polynomial, most significant bit first.
// Built at compile time, where an index out of range would stop the build.
#[allow(clippy::indexing_slicing)]
const CRC_TABLE: [u32; 256] = {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = (i as u32) << 24;
        let mut k = 0;
        while k < 8 {
            c = if c & 0x8000_0000 != 0 {
                (c << 1) ^ 0x04c1_1db7
            } else {
                c << 1
            };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
};

fn crc_step(crc: u32, b: u8) -> u32 {
    let i = ((crc >> 24) ^ u32::from(b)) & 0xff;
    (crc << 8) ^ CRC_TABLE.get(i as usize).copied().unwrap_or(0)
}

struct Table {
    min: usize,
    max: usize,
    limit: Vec<i64>,
    first: Vec<i64>,
    first_index: Vec<usize>,
    perm: Vec<u16>,
}

fn bad(what: &str) -> UnpackError {
    damaged(format!("bzip2: {what}"))
}

fn read_tables(r: &mut BitReader, alpha_size: usize) -> Result<(Vec<u8>, Vec<Table>), UnpackError> {
    let groups = r.bits(3)? as usize;
    let selector_count = r.bits(15)? as usize;
    if !(2..=6).contains(&groups) || selector_count < 1 {
        return Err(bad("bad table count"));
    }
    // which table each run of 50 symbols uses, sent move-to-front coded in unary
    let mut order: Vec<u8> = (0..groups as u8).collect();
    let mut selectors = Vec::with_capacity(selector_count);
    for _ in 0..selector_count {
        let mut j = 0;
        while r.bit()? {
            j += 1;
            if j >= groups {
                return Err(bad("bad table selector"));
            }
        }
        let v = order.remove(j);
        order.insert(0, v);
        selectors.push(v);
    }
    // code lengths for each table, sent as differences, then canonical codes
    // in order of length and symbol
    let mut tables = Vec::with_capacity(groups);
    for _ in 0..groups {
        let mut lengths = vec![0usize; alpha_size];
        let mut len = r.bits(5)? as i32;
        for slot in &mut lengths {
            loop {
                if !(1..=20).contains(&len) {
                    return Err(bad("bad code length"));
                }
                if !r.bit()? {
                    break;
                }
                len += if r.bit()? { -1 } else { 1 };
            }
            *slot = len as usize;
        }
        let min = lengths.iter().copied().min().unwrap_or(1);
        let max = lengths.iter().copied().max().unwrap_or(1);
        let mut limit = vec![-1i64; max + 2];
        let mut first = vec![0i64; max + 2];
        let mut first_index = vec![0usize; max + 2];
        let mut perm = vec![0u16; alpha_size];
        let mut code: i64 = 0;
        let mut idx = 0usize;
        for l in min..=max {
            if let Some(f) = first.get_mut(l) {
                *f = code;
            }
            if let Some(f) = first_index.get_mut(l) {
                *f = idx;
            }
            let start = idx;
            for (s, &sl) in lengths.iter().enumerate() {
                if sl == l {
                    if let Some(p) = perm.get_mut(idx) {
                        *p = s as u16;
                    }
                    idx += 1;
                }
            }
            code += (idx - start) as i64;
            if let Some(lim) = limit.get_mut(l) {
                *lim = code - 1;
            }
            code <<= 1;
        }
        tables.push(Table {
            min,
            max,
            limit,
            first,
            first_index,
            perm,
        });
    }
    Ok((selectors, tables))
}

/// Decodes one block into `out` and gives its checksum.
fn read_block(
    r: &mut BitReader,
    max_block: usize,
    out: &mut Vec<u8>,
    budget: &mut Budget,
) -> Result<u32, UnpackError> {
    let expected_crc = (r.bits(16)? << 16) | r.bits(16)?;
    if r.bit()? {
        return Err(damaged(
            "bzip2: it uses the \"randomized\" blocks of bzip2 0.9.0 and earlier, which are not supported",
        ));
    }
    let orig_ptr = r.bits(24)? as usize;
    // the byte values the block uses
    let used = r.bits(16)?;
    let mut seq_to_unseq: Vec<u8> = Vec::new();
    for i in 0..16u32 {
        if used & (0x8000 >> i) == 0 {
            continue;
        }
        let bits = r.bits(16)?;
        for j in 0..16u32 {
            if bits & (0x8000 >> j) != 0 {
                seq_to_unseq.push((i * 16 + j) as u8);
            }
        }
    }
    if seq_to_unseq.is_empty() {
        return Err(bad("empty symbol map"));
    }
    let alpha_size = seq_to_unseq.len() + 2;
    let eob = alpha_size - 1;
    let (selectors, tables) = read_tables(r, alpha_size)?;

    // Huffman symbols to bytes, undoing move-to-front and the RUNA/RUNB run lengths
    let mut block: Vec<u8> = Vec::with_capacity(max_block.min(1 << 20));
    let mut counts = [0usize; 256];
    let mut mtf: Vec<u8> = (0..seq_to_unseq.len() as u16).map(|i| i as u8).collect();
    let mut run = 0usize;
    let mut run_weight = 1usize;
    let mut selector = 0usize;
    let mut left = 0;
    let mut table = tables.first().ok_or_else(|| bad("no tables"))?;
    loop {
        if left == 0 {
            let which = selectors
                .get(selector)
                .ok_or_else(|| bad("ran out of table selectors"))?;
            selector += 1;
            table = tables
                .get(usize::from(*which))
                .ok_or_else(|| bad("bad table selector"))?;
            left = 50;
        }
        left -= 1;
        let mut len = table.min;
        let mut v = i64::from(r.bits(len as u32)?);
        while v > table.limit.get(len).copied().unwrap_or(-1) {
            len += 1;
            if len > table.max {
                return Err(bad("bad code"));
            }
            v = (v << 1) | i64::from(r.bits(1)?);
        }
        let at = table.first_index.get(len).copied().unwrap_or(0) as i64 + v
            - table.first.get(len).copied().unwrap_or(0);
        let sym = usize::try_from(at)
            .ok()
            .and_then(|at| table.perm.get(at))
            .map(|s| usize::from(*s))
            .ok_or_else(|| bad("bad code"))?;
        if sym <= 1 {
            run += (sym + 1) * run_weight;
            run_weight <<= 1;
            if run > max_block {
                return Err(bad("run too long"));
            }
            continue;
        }
        if run > 0 {
            let b = mtf
                .first()
                .and_then(|m| seq_to_unseq.get(usize::from(*m)))
                .copied()
                .ok_or_else(|| bad("bad symbol"))?;
            if block.len() + run > max_block {
                return Err(bad("block too long"));
            }
            block.resize(block.len() + run, b);
            if let Some(c) = counts.get_mut(usize::from(b)) {
                *c += run;
            }
            run = 0;
            run_weight = 1;
        }
        if sym == eob {
            break;
        }
        let pos = sym - 1;
        if pos >= mtf.len() {
            return Err(bad("bad symbol"));
        }
        let m = mtf.remove(pos);
        mtf.insert(0, m);
        let b = seq_to_unseq
            .get(usize::from(m))
            .copied()
            .ok_or_else(|| bad("bad symbol"))?;
        if block.len() >= max_block {
            return Err(bad("block too long"));
        }
        block.push(b);
        if let Some(c) = counts.get_mut(usize::from(b)) {
            *c += 1;
        }
    }
    let n = block.len();
    if orig_ptr >= n {
        return Err(bad("bad start position"));
    }

    // inverse Burrows-Wheeler transform
    let mut next = vec![0u32; n];
    let mut start = [0usize; 256];
    let mut sum = 0;
    for (s, c) in start.iter_mut().zip(counts.iter()) {
        *s = sum;
        sum += c;
    }
    for (i, &b) in block.iter().enumerate() {
        let slot = start
            .get_mut(usize::from(b))
            .ok_or_else(|| bad("bad symbol"))?;
        if let Some(x) = next.get_mut(*slot) {
            *x = i as u32;
        }
        *slot += 1;
    }

    // the first run-length step: four equal bytes are followed by a count of more
    let mut crc = 0xffff_ffffu32;
    let mut pos = next.get(orig_ptr).copied().unwrap_or(0) as usize;
    let mut last: Option<u8> = None;
    let mut same = 0;
    budget.take(n as u64)?;
    out.reserve(n);
    for _ in 0..n {
        let b = block
            .get(pos)
            .copied()
            .ok_or_else(|| bad("bad start position"))?;
        pos = next.get(pos).copied().unwrap_or(0) as usize;
        if same == 4 {
            // the count byte was counted as one output byte above
            let extra = usize::from(b);
            budget.take((extra as u64).saturating_sub(1))?;
            let fill = last.unwrap_or(0);
            for _ in 0..extra {
                out.push(fill);
                crc = crc_step(crc, fill);
            }
            same = 0;
            last = None;
            continue;
        }
        if Some(b) == last {
            same += 1;
        } else {
            last = Some(b);
            same = 1;
        }
        out.push(b);
        crc = crc_step(crc, b);
    }
    let crc = !crc;
    if crc != expected_crc {
        return Err(bad("a block failed its checksum"));
    }
    Ok(crc)
}

/// Whether `bytes` start like a bzip2 stream.
pub(super) fn is_bzip2(b: &[u8]) -> bool {
    b.starts_with(b"BZh") && b.get(3).is_some_and(|d| (b'1'..=b'9').contains(d))
}

/// Decodes a whole .bz2 file.
pub(super) fn bunzip2(bytes: &[u8], budget: &mut Budget) -> Result<Vec<u8>, UnpackError> {
    let mut r = BitReader {
        bytes,
        pos: 0,
        buf: 0,
        count: 0,
    };
    let mut out = Vec::new();
    let mut streams = 0;
    while !r.at_end() {
        let magic_ok = r.bits(8)? == 0x42 && r.bits(8)? == 0x5a && r.bits(8)? == 0x68;
        if !magic_ok {
            if streams > 0 {
                break; // trailing bytes after the last stream, as bzip2 allows
            }
            return Err(bad("not a bzip2 stream"));
        }
        let level = r.bits(8)?.wrapping_sub(0x30);
        if !(1..=9).contains(&level) {
            return Err(bad("bad block size"));
        }
        let mut combined = 0u32;
        loop {
            let hi = r.bits(24)?;
            let lo = r.bits(24)?;
            if (hi, lo) == (0x31_4159, 0x26_5359) {
                let crc = read_block(&mut r, level as usize * 100_000, &mut out, budget)?;
                combined = combined.rotate_left(1) ^ crc;
            } else if (hi, lo) == (0x17_7245, 0x38_5090) {
                let expected = (r.bits(16)? << 16) | r.bits(16)?;
                if expected != combined {
                    return Err(bad("the whole-file checksum does not match"));
                }
                r.align();
                break;
            } else {
                return Err(bad("bad block marker"));
            }
        }
        streams += 1;
    }
    Ok(out)
}
