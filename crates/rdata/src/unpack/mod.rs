//! Opens compressed data files: `.zip`, `.gz`, `.bz2`, `.xz`, `.zst`, `.tar`
//! and their combinations (`.tar.gz`, `.tgz`, `.tar.bz2`, `.tar.xz`,
//! `.tar.zst`). The kind is told from the file's first bytes, not its name.
//! Archives inside archives are opened too (a `.csv.gz` inside a zip).
//!
//! Every checksum is checked (gzip and zip CRC-32, bzip2 block and stream
//! CRCs, the xz block checks, the zstd content checksum), so a damaged file
//! is refused rather than read wrong. Password-protected zip entries, 7-Zip
//! and RAR files get plain messages. The unpacked bytes are capped.
//!
//! All decoders are pure Rust: `flate2` on `miniz_oxide` (gzip, deflate),
//! a bzip2 decoder ported from Husky Explorer (`bunzip`), `lzma-rust2` (xz) and `ruzstd` (Zstandard). The zip and tar
//! readers are here, and never trust a size or offset they read.
//!
//! The files R itself reads packed (saved R objects, whose `.rds` and
//! `.RData` are gzip inside) and Office files (zip inside) are left as they
//! are.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

mod bunzip;
use bunzip::is_bzip2;

/// The most bytes [`unpack`] gives back, all inner files together.
pub const MAX_UNPACKED: u64 = 4 << 30;

/// Endings of packed files, for a file chooser's filter.
pub const COMPRESSED_EXTENSIONS: &[&str] = &[
    "zip", "gz", "tgz", "bz2", "tbz2", "tbz", "xz", "txz", "zst", "tzst", "tar",
];

/// How deep archives inside archives are opened.
const MAX_DEPTH: usize = 3;
/// The most files one archive may list.
const MAX_ENTRIES: usize = 100_000;

/// What a file is, from its first bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Gzip,
    Zip,
    Bzip2,
    Tar,
    Xz,
    Zstd,
    SevenZip,
    Rar,
}

/// Tells the kind of a file from its first bytes (512 are enough).
pub fn kind_of(bytes: &[u8]) -> Kind {
    let starts = |magic: &[u8]| bytes.starts_with(magic);
    if starts(&[0x1f, 0x8b]) {
        Kind::Gzip
    } else if starts(b"PK\x03\x04") || starts(b"PK\x05\x06") {
        Kind::Zip
    } else if is_bzip2(bytes) {
        Kind::Bzip2
    } else if bytes.get(257..262) == Some(b"ustar".as_slice()) {
        Kind::Tar
    } else if starts(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        Kind::Xz
    } else if starts(&[0x28, 0xb5, 0x2f, 0xfd]) {
        Kind::Zstd
    } else if starts(&[b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c]) {
        Kind::SevenZip
    } else if starts(b"Rar!") {
        Kind::Rar
    } else {
        Kind::Plain
    }
}

/// Why a packed file could not be opened, in words for the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnpackError {
    /// The file could not be read from disk.
    Io(String),
    /// A checksum failed, or the data is cut off or malformed. The text says what.
    Damaged(String),
    /// The unpacked files would pass the cap.
    TooLarge(u64),
    /// A zip entry is encrypted.
    Password(String),
    /// 7-Zip or RAR.
    Unsupported(&'static str),
    /// A zip entry packed a way RCraft cannot read.
    Method { name: String, method: u16 },
}

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnpackError::Io(e) => write!(f, "could not read the file: {e}"),
            UnpackError::Damaged(what) => write!(f, "The file is damaged or cut off ({what})."),
            UnpackError::TooLarge(max) => write!(
                f,
                "The unpacked files would be larger than {} MB, more than RCraft unpacks.",
                max / 1_000_000
            ),
            UnpackError::Password(name) => write!(
                f,
                "\"{name}\" in the zip file is password-protected. Unzip it on your computer first."
            ),
            UnpackError::Unsupported(kind) => write!(
                f,
                "This is a {kind} file, which RCraft cannot open. Unpack it on your computer \
                 first, or save it as .zip."
            ),
            UnpackError::Method { name, method } => write!(
                f,
                "\"{name}\" in the zip file is packed a way RCraft cannot open (method {method}). \
                 Unzip it on your computer first."
            ),
        }
    }
}

impl std::error::Error for UnpackError {}

fn damaged(what: impl Into<String>) -> UnpackError {
    UnpackError::Damaged(what.into())
}

/// A file found inside a packed one. `name` is its path inside the archive.
#[derive(Clone, PartialEq, Eq)]
pub struct Inner {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Inner({:?}, {} bytes)", self.name, self.bytes.len())
    }
}

/// Unpacked bytes still allowed.
struct Budget {
    left: u64,
    max: u64,
}

impl Budget {
    fn take(&mut self, n: u64) -> Result<(), UnpackError> {
        self.left = self
            .left
            .checked_sub(n)
            .ok_or(UnpackError::TooLarge(self.max))?;
        Ok(())
    }

    /// Reads all of `r`, counting what it gives against the budget.
    fn read_all(&mut self, r: impl Read, what: &str) -> Result<Vec<u8>, UnpackError> {
        let mut out = Vec::new();
        r.take(self.left.saturating_add(1))
            .read_to_end(&mut out)
            .map_err(|e| damaged(format!("{what}: {e}")))?;
        self.take(out.len() as u64)?;
        Ok(out)
    }
}

/// Files R reads packed as they are, and Office files, are not opened.
fn keep_packed(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        ".rds", ".rdata", ".rda", ".xlsx", ".xlsm", ".docx", ".pptx", ".ods",
    ]
    .iter()
    .any(|end| lower.ends_with(end))
}

/// The files inside `bytes`, a file named `name`. A file that is not packed
/// comes back as itself. `max` caps the unpacked bytes, all files together.
pub fn unpack(bytes: &[u8], name: &str, max: u64) -> Result<Vec<Inner>, UnpackError> {
    let mut budget = Budget { left: max, max };
    // The decoders are other people's code; a panic in one on a hostile file
    // becomes an error here, never a crash.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        unpack_at(bytes, name, &mut budget, 0)
    }))
    .unwrap_or_else(|_| Err(damaged("the decoder stopped")))
}

fn unpack_at(
    bytes: &[u8],
    name: &str,
    budget: &mut Budget,
    depth: usize,
) -> Result<Vec<Inner>, UnpackError> {
    let kind = kind_of(bytes);
    if kind == Kind::Plain || depth > MAX_DEPTH || keep_packed(name) {
        return Ok(vec![Inner {
            name: name.to_string(),
            bytes: bytes.to_vec(),
        }]);
    }
    let inner = match kind {
        Kind::Plain => Vec::new(),
        Kind::SevenZip => return Err(UnpackError::Unsupported("7-Zip")),
        Kind::Rar => return Err(UnpackError::Unsupported("RAR")),
        Kind::Gzip => vec![Inner {
            name: strip(name, &[(".tgz", ".tar"), (".gz", "")]),
            bytes: budget.read_all(flate2::read::MultiGzDecoder::new(bytes), "gzip")?,
        }],
        Kind::Bzip2 => vec![Inner {
            name: strip(name, &[(".tbz2", ".tar"), (".tbz", ".tar"), (".bz2", "")]),
            bytes: bunzip::bunzip2(bytes, budget)?,
        }],
        Kind::Xz => vec![Inner {
            name: strip(name, &[(".txz", ".tar"), (".xz", "")]),
            bytes: budget.read_all(lzma_rust2::XzReader::new(bytes, true), "xz")?,
        }],
        Kind::Zstd => vec![Inner {
            name: strip(name, &[(".tzst", ".tar"), (".zst", ""), (".zstd", "")]),
            bytes: unzstd(bytes, budget)?,
        }],
        Kind::Zip => match read_zip(bytes, budget)? {
            Some(files) => files,
            // an Office file or the like: a zip that is itself the data file
            None => {
                return Ok(vec![Inner {
                    name: name.to_string(),
                    bytes: bytes.to_vec(),
                }])
            }
        },
        Kind::Tar => read_tar(bytes)?,
    };
    let mut out = Vec::new();
    for f in inner {
        out.extend(unpack_at(&f.bytes, &f.name, budget, depth + 1)?);
    }
    Ok(out)
}

/// Swaps the first of `ends` that `name` ends with (any case).
fn strip(name: &str, ends: &[(&str, &str)]) -> String {
    let lower = name.to_ascii_lowercase();
    for (end, to) in ends {
        if lower.ends_with(end) {
            if let Some(stem) = name.get(..name.len() - end.len()) {
                return format!("{stem}{to}");
            }
        }
    }
    name.to_string()
}

/// Zstandard, one frame or several, skipping skippable frames and checking
/// each frame's checksum when it has one.
fn unzstd(bytes: &[u8], budget: &mut Budget) -> Result<Vec<u8>, UnpackError> {
    use ruzstd::decoding::errors::{FrameDecoderError, ReadFrameHeaderError};
    use ruzstd::decoding::{BlockDecodingStrategy, FrameDecoder};
    let bad = |e: FrameDecoderError| damaged(format!("zstd: {e}"));
    let mut src = bytes;
    let mut out = Vec::new();
    let mut dec = FrameDecoder::new();
    while !src.is_empty() {
        match dec.reset(&mut src) {
            Ok(()) => {}
            Err(FrameDecoderError::ReadFrameHeaderError(ReadFrameHeaderError::SkipFrame {
                length,
                ..
            })) => {
                src = src
                    .get(length as usize..)
                    .ok_or_else(|| damaged("zstd: a skippable frame is cut off"))?;
                continue;
            }
            Err(e) => return Err(bad(e)),
        }
        loop {
            dec.decode_blocks(&mut src, BlockDecodingStrategy::UptoBytes(1 << 20))
                .map_err(bad)?;
            let done = dec.is_finished();
            if let Some(chunk) = dec.collect() {
                budget.take(chunk.len() as u64)?;
                out.extend_from_slice(&chunk);
            }
            if done {
                break;
            }
        }
        if let (Some(want), Some(got)) =
            (dec.get_checksum_from_data(), dec.get_calculated_checksum())
        {
            if want != got {
                return Err(damaged("zstd: the checksum does not match"));
            }
        }
    }
    Ok(out)
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([*s.first()?, *s.get(1)?]))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    let s: [u8; 4] = b.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(s))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    let s: [u8; 8] = b.get(at..at.checked_add(8)?)?.try_into().ok()?;
    Some(u64::from_le_bytes(s))
}

/// One file listed in a zip's central directory.
struct ZipEntry {
    name: String,
    flags: u16,
    method: u16,
    crc: u32,
    packed: u64,
    full: u64,
    local: u64,
}

/// The entries of a zip, from the directory at its end.
fn zip_directory(b: &[u8]) -> Result<Vec<ZipEntry>, UnpackError> {
    let list_missing = || damaged("its list of files is missing");
    let unreadable = || damaged("its list of files is unreadable");
    // the end record, searched back past a comment of up to 64 KB
    let last = b.len().checked_sub(22).ok_or_else(list_missing)?;
    let first = last.saturating_sub(65_535);
    let end = (first..=last)
        .rev()
        .find(|&i| u32_at(b, i) == Some(0x0605_4b50))
        .ok_or_else(list_missing)?;
    let mut count = u64::from(u16_at(b, end + 10).ok_or_else(unreadable)?);
    let mut dir_at = u64::from(u32_at(b, end + 16).ok_or_else(unreadable)?);
    // Zip64: more than 65,535 files or a directory past 4 GB
    if (count == 0xffff || dir_at == 0xffff_ffff)
        && end >= 20
        && u32_at(b, end - 20) == Some(0x0706_4b50)
    {
        let rec = u64_at(b, end - 12).ok_or_else(unreadable)?;
        let rec = usize::try_from(rec).map_err(|_| unreadable())?;
        if u32_at(b, rec) == Some(0x0606_4b50) {
            count = u64_at(b, rec + 32).ok_or_else(unreadable)?;
            dir_at = u64_at(b, rec + 48).ok_or_else(unreadable)?;
        }
    }
    if count > MAX_ENTRIES as u64 {
        return Err(damaged("it lists more files than RCraft opens"));
    }
    let mut at = usize::try_from(dir_at).map_err(|_| unreadable())?;
    let mut entries = Vec::new();
    for _ in 0..count {
        if u32_at(b, at) != Some(0x0201_4b50) {
            return Err(unreadable());
        }
        let field16 = |o: usize| u16_at(b, at + o).ok_or_else(unreadable);
        let field32 = |o: usize| u32_at(b, at + o).ok_or_else(unreadable);
        let flags = field16(8)?;
        let method = field16(10)?;
        let crc = field32(16)?;
        let mut packed = u64::from(field32(20)?);
        let mut full = u64::from(field32(24)?);
        let name_len = usize::from(field16(28)?);
        let extra_len = usize::from(field16(30)?);
        let comment_len = usize::from(field16(32)?);
        let mut local = u64::from(field32(42)?);
        let name_at = at + 46;
        let name = b.get(name_at..name_at + name_len).ok_or_else(unreadable)?;
        let name = String::from_utf8_lossy(name).into_owned();
        // Zip64 sizes and offsets live in an extra field
        let extra_end = name_at + name_len + extra_len;
        let mut e = name_at + name_len;
        while e + 4 <= extra_end {
            let id = u16_at(b, e).ok_or_else(unreadable)?;
            let len = usize::from(u16_at(b, e + 2).ok_or_else(unreadable)?);
            if id == 0x0001 {
                let mut p = e + 4;
                for value in [&mut full, &mut packed, &mut local] {
                    if *value == 0xffff_ffff {
                        *value = u64_at(b, p).ok_or_else(unreadable)?;
                        p += 8;
                    }
                }
            }
            e += 4 + len;
        }
        at = extra_end + comment_len;
        entries.push(ZipEntry {
            name,
            flags,
            method,
            crc,
            packed,
            full,
            local,
        });
    }
    Ok(entries)
}

/// The files in a zip, or `None` when the zip is an Office file.
fn read_zip(b: &[u8], budget: &mut Budget) -> Result<Option<Vec<Inner>>, UnpackError> {
    let entries = zip_directory(b)?;
    if entries.iter().any(|e| e.name == "[Content_Types].xml") {
        return Ok(None);
    }
    let mut files = Vec::new();
    for e in entries {
        let name = e.name;
        let is_mac_junk =
            name.starts_with("__MACOSX/") || name == ".DS_Store" || name.ends_with("/.DS_Store");
        if name.ends_with('/') || is_mac_junk {
            continue;
        }
        if e.flags & 1 != 0 {
            return Err(UnpackError::Password(name));
        }
        let missing = || damaged(format!("\"{name}\" cannot be found in it"));
        let local = usize::try_from(e.local).map_err(|_| missing())?;
        if u32_at(b, local) != Some(0x0403_4b50) {
            return Err(missing());
        }
        let skip = usize::from(u16_at(b, local + 26).ok_or_else(missing)?)
            + usize::from(u16_at(b, local + 28).ok_or_else(missing)?);
        let data_at = local + 30 + skip;
        let packed_len = usize::try_from(e.packed).map_err(|_| missing())?;
        let packed = b
            .get(data_at..data_at.saturating_add(packed_len))
            .ok_or_else(|| damaged(format!("\"{name}\" is cut off")))?;
        // the listed size is checked after decoding; it only guards the cap here
        if e.full > budget.left {
            return Err(UnpackError::TooLarge(budget.max));
        }
        let data = match e.method {
            0 => {
                budget.take(packed.len() as u64)?;
                packed.to_vec()
            }
            8 => budget.read_all(flate2::read::DeflateDecoder::new(packed), &name)?,
            12 => bunzip::bunzip2(packed, budget)?,
            93 => unzstd(packed, budget)?,
            95 => budget.read_all(lzma_rust2::XzReader::new(packed, true), &name)?,
            method => return Err(UnpackError::Method { name, method }),
        };
        let mut crc = flate2::Crc::new();
        crc.update(&data);
        if data.len() as u64 != e.full || crc.sum() != e.crc {
            return Err(damaged(format!("\"{name}\" failed its checksum")));
        }
        files.push(Inner { name, bytes: data });
    }
    Ok(Some(files))
}

/// A text field of a tar header, up to its first NUL.
fn tar_text(field: &[u8]) -> String {
    let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    String::from_utf8_lossy(field.get(..end).unwrap_or(field)).into_owned()
}

/// The regular files in a tar (ustar, GNU long names, pax paths).
fn read_tar(b: &[u8]) -> Result<Vec<Inner>, UnpackError> {
    let mut files = Vec::new();
    let mut long_name: Option<String> = None;
    let mut at = 0usize;
    while let Some(header) = b.get(at..at + 512) {
        if header.iter().all(|&c| c == 0) {
            break;
        }
        let field = |from: usize, len: usize| tar_text(header.get(from..from + len).unwrap_or(&[]));
        let size_text = field(124, 12);
        let size_text = size_text.trim_matches([' ', '\0']);
        let size = if size_text.is_empty() {
            0
        } else {
            u64::from_str_radix(size_text, 8).map_err(|_| damaged("a tar header is unreadable"))?
        };
        let size = usize::try_from(size).map_err(|_| damaged("a tar entry is too large"))?;
        let kind = header.get(156).copied().unwrap_or(0);
        let prefix = field(345, 155);
        let short = field(0, 100);
        let name = long_name.take().unwrap_or_else(|| {
            if prefix.is_empty() {
                short
            } else {
                format!("{prefix}/{short}")
            }
        });
        let data_at = at + 512;
        let data = b
            .get(data_at..data_at.saturating_add(size))
            .ok_or_else(|| damaged(format!("\"{name}\" is cut off")))?;
        let padded = size
            .checked_add(511)
            .map(|n| n / 512 * 512)
            .ok_or_else(|| damaged("a tar entry is too large"))?;
        at = data_at.saturating_add(padded);
        match kind {
            b'L' => long_name = Some(tar_text(data)),
            b'x' => {
                let text = String::from_utf8_lossy(data);
                long_name = text.lines().find_map(|line| {
                    let (_, rest) = line.split_once(' ')?;
                    rest.strip_prefix("path=").map(str::to_string)
                });
            }
            b'0' | 0 => {
                let base = name.rsplit('/').next().unwrap_or(&name);
                if !base.starts_with("._") {
                    if files.len() >= MAX_ENTRIES {
                        return Err(damaged("it lists more files than RCraft opens"));
                    }
                    files.push(Inner {
                        name,
                        bytes: data.to_vec(),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(files)
}

/// Opens the file at `path`. `Ok(None)` when it is not packed (or is a file R
/// reads packed by itself); otherwise the files inside.
pub fn open_path(path: &Path, max: u64) -> Result<Option<Vec<Inner>>, UnpackError> {
    let io = |e: std::io::Error| UnpackError::Io(e.to_string());
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if keep_packed(&name) {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path).map_err(io)?;
    let mut head = Vec::new();
    (&mut file).take(512).read_to_end(&mut head).map_err(io)?;
    match kind_of(&head) {
        Kind::Plain => return Ok(None),
        Kind::SevenZip => return Err(UnpackError::Unsupported("7-Zip")),
        Kind::Rar => return Err(UnpackError::Unsupported("RAR")),
        _ => {}
    }
    let mut bytes = head;
    file.take(max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() as u64 > max {
        return Err(UnpackError::TooLarge(max));
    }
    let files = unpack(&bytes, &name, max)?;
    // a zip that is an Office file comes back as itself
    if files.len() == 1
        && files
            .first()
            .is_some_and(|f| f.name == name && f.bytes == bytes)
    {
        return Ok(None);
    }
    Ok(Some(files))
}

/// The files worth offering for a box that takes these endings (without the
/// dot, any case): those that match, or all of them when none do, so a file
/// with an unusual name can still be picked.
pub fn offered<'a>(files: &'a [Inner], extensions: &[&str]) -> Vec<&'a Inner> {
    let fits = |f: &Inner| {
        let lower = f.name.to_ascii_lowercase();
        extensions
            .iter()
            .any(|e| lower.ends_with(&format!(".{}", e.to_ascii_lowercase())))
    };
    let matching: Vec<&Inner> = files.iter().filter(|f| fits(f)).collect();
    if matching.is_empty() {
        files.iter().collect()
    } else {
        matching
    }
}

/// A safe file name for an inner file: its last path part, without the
/// characters file systems refuse.
fn safe_file_name(inner: &str) -> String {
    let base = inner.rsplit(['/', '\\']).next().unwrap_or(inner);
    let clean: String = base
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
        .collect();
    let clean = clean.trim().trim_end_matches('.').to_string();
    if clean.is_empty() || clean == "." || clean == ".." {
        "unpacked".to_string()
    } else {
        clean
    }
}

/// Writes `inner` next to the archive it came from, under its own name, and
/// says where. A file already there with the same bytes is used as it is; a
/// different one is never replaced: the new file gets " (unpacked)" and, if
/// needed, a number. When the archive's folder cannot be written, the file
/// goes to an `rcraft-unpacked` folder in the system's temporary folder.
pub fn write_beside(archive: &Path, inner: &Inner) -> Result<PathBuf, UnpackError> {
    let folder = archive.parent().map(Path::to_path_buf).unwrap_or_default();
    match write_into(&folder, inner) {
        Ok(path) => Ok(path),
        Err(_) => {
            let temp = std::env::temp_dir().join("rcraft-unpacked");
            std::fs::create_dir_all(&temp).map_err(|e| UnpackError::Io(e.to_string()))?;
            write_into(&temp, inner)
        }
    }
}

fn write_into(folder: &Path, inner: &Inner) -> Result<PathBuf, UnpackError> {
    use std::io::Write;
    let name = safe_file_name(&inner.name);
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.clone(), String::new()),
    };
    let mut last_error = String::from("no free file name");
    for n in 0..100u32 {
        let candidate = match n {
            0 => name.clone(),
            1 => format!("{stem} (unpacked){ext}"),
            n => format!("{stem} (unpacked {n}){ext}"),
        };
        let path = folder.join(&candidate);
        if path.exists() {
            let same = std::fs::metadata(&path).is_ok_and(|m| m.len() == inner.bytes.len() as u64)
                && std::fs::read(&path).is_ok_and(|b| b == inner.bytes);
            if same {
                return Ok(path);
            }
            continue;
        }
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(f) => f,
            // made by someone else just now: try the next name
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                last_error = e.to_string();
                break;
            }
        };
        match file.write_all(&inner.bytes) {
            Ok(()) => return Ok(path),
            Err(e) => {
                // only the file made here is removed, never one that was there
                drop(file);
                let _ = std::fs::remove_file(&path);
                last_error = e.to_string();
                break;
            }
        }
    }
    Err(UnpackError::Io(last_error))
}

#[cfg(test)]
mod tests;
