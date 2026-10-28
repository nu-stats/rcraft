//! The on-disk copy of the index: a trimmed text file, so searching starts
//! fast and works with no network.
//!
//! ```text
//! RCRAN1
//! fetched<TAB>seconds since 1970
//! detail<TAB>full or names
//! count<TAB>number of packages
//! one line per package: the 17 fields of `index::FIELD_NAMES`, tab separated
//! ```
//!
//! In a field a backslash is `\\`, a tab `\t`, a newline `\n` and a carriage
//! return `\r`. The count and the field count of every line are checked, so a
//! file cut short or edited by hand is an error, never a half-loaded index.
//! A new cache is written under a temporary name and renamed over the old
//! one, so a failure part-way leaves the previous cache as it was.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::index::{Detail, Index, Package, FIELD_NAMES};
use crate::{Error, Result};

/// The file name inside the cache folder.
pub const FILE_NAME: &str = "cran-index.txt";
const MAGIC: &str = "RCRAN1";
/// Largest cache read, in bytes.
pub const MAX_BYTES: u64 = 256 * 1024 * 1024;
/// Most packages accepted.
pub const MAX_PACKAGES: usize = 1_000_000;

/// The cache folder, which may not exist yet.
#[derive(Debug, Clone)]
pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn new(dir: impl Into<PathBuf>) -> Cache {
        Cache { dir: dir.into() }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    /// The cached index; `Ok(None)` when there is no cache yet. A damaged
    /// cache is an error and is left in place.
    pub fn load(&self) -> Result<Option<Index>> {
        let path = self.path();
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io(&path, "read", &e)),
        };
        if meta.len() > MAX_BYTES {
            return Err(Error::TooBig(format!(
                "the cache is over {MAX_BYTES} bytes"
            )));
        }
        let bytes = fs::read(&path).map_err(|e| io(&path, "read", &e))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| Error::Format("the cache is not valid text".into()))?;
        from_text(&text).map(Some)
    }

    /// Writes `index` as the cache, replacing the old one only once the new
    /// file is complete.
    pub fn store(&self, index: &Index) -> Result<()> {
        fs::create_dir_all(&self.dir).map_err(|e| io(&self.dir, "create", &e))?;
        let final_path = self.path();
        let tmp = self.dir.join(format!("{FILE_NAME}.tmp"));
        let text = to_text(index);
        let written = (|| -> std::io::Result<()> {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?;
            fs::rename(&tmp, &final_path)
        })();
        if let Err(e) = written {
            let _ = fs::remove_file(&tmp);
            return Err(io(&final_path, "write", &e));
        }
        Ok(())
    }
}

fn io(path: &Path, what: &str, e: &std::io::Error) -> Error {
    Error::Io(format!("could not {what} {}: {e}", path.display()))
}

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
}

fn unescape(s: &str) -> Result<String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            _ => return Err(Error::Format("a bad escape in the cache".into())),
        }
    }
    Ok(out)
}

/// The cache text for `index`.
pub fn to_text(index: &Index) -> String {
    let mut out = String::new();
    out.push_str(MAGIC);
    out.push('\n');
    out.push_str(&format!("fetched\t{}\n", index.fetched_at));
    out.push_str(match index.detail {
        Detail::Full => "detail\tfull\n",
        Detail::NamesOnly => "detail\tnames\n",
    });
    out.push_str(&format!("count\t{}\n", index.len()));
    for p in index.packages() {
        for (i, f) in p.to_fields().iter().enumerate() {
            if i > 0 {
                out.push('\t');
            }
            escape(f, &mut out);
        }
        out.push('\n');
    }
    out
}

/// Reads cache text back into an index.
pub fn from_text(text: &str) -> Result<Index> {
    let mut lines = text.split('\n');
    if lines.next() != Some(MAGIC) {
        return Err(Error::Format("not an RCraft package cache".into()));
    }
    let mut header = |key: &str| -> Result<String> {
        let line = lines.next().ok_or(Error::Truncated)?;
        line.strip_prefix(key)
            .and_then(|r| r.strip_prefix('\t'))
            .map(str::to_string)
            .ok_or_else(|| Error::Format(format!("the cache header lacks {key}")))
    };
    let fetched_at: u64 = header("fetched")?
        .parse()
        .map_err(|_| Error::Format("the cache time is not a number".into()))?;
    let detail = match header("detail")?.as_str() {
        "full" => Detail::Full,
        "names" => Detail::NamesOnly,
        other => return Err(Error::Format(format!("unknown detail level {other}"))),
    };
    let count: usize = header("count")?
        .parse()
        .map_err(|_| Error::Format("the cache count is not a number".into()))?;
    if count > MAX_PACKAGES {
        return Err(Error::TooBig(format!("more than {MAX_PACKAGES} packages")));
    }
    let mut packages = Vec::with_capacity(count.min(100_000));
    for _ in 0..count {
        let line = lines.next().ok_or(Error::Truncated)?;
        let fields = line.split('\t').map(unescape).collect::<Result<Vec<_>>>()?;
        let p = Package::from_fields(&fields).ok_or_else(|| {
            Error::Format(format!(
                "a cache line has {} fields, expected {}",
                fields.len(),
                FIELD_NAMES.len()
            ))
        })?;
        packages.push(p);
    }
    // the last line must end with a newline, and nothing may follow it
    if lines.next() != Some("") {
        return Err(Error::Truncated);
    }
    if lines.next().is_some() {
        return Err(Error::Format("extra lines after the packages".into()));
    }
    Ok(Index::new(packages, fetched_at, detail))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::sample;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rcraft-rpkg-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn text_round_trips_including_awkward_characters() {
        let mut ps = sample().packages().to_vec();
        if let Some(p) = ps.first_mut() {
            p.description = "tabs\there, newlines\nthere, a back\\slash, \r and ünïcode ✓".into();
        }
        let ix = Index::new(ps, 42, Detail::NamesOnly);
        assert_eq!(from_text(&to_text(&ix)), Ok(ix));
    }

    #[test]
    fn a_missing_cache_is_none_and_a_stored_one_loads() {
        let dir = temp("store");
        let cache = Cache::new(&dir);
        assert_eq!(cache.load(), Ok(None));
        let ix = sample();
        assert_eq!(cache.store(&ix), Ok(()));
        assert_eq!(cache.load(), Ok(Some(ix.clone())));
        // the temporary file is gone
        assert!(!dir.join(format!("{FILE_NAME}.tmp")).exists());
        // storing again replaces
        let newer = Index::new(vec![], 99, Detail::NamesOnly);
        assert_eq!(cache.store(&newer), Ok(()));
        assert_eq!(cache.load(), Ok(Some(newer)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn damaged_caches_are_errors_and_stay_on_disk() {
        let dir = temp("damaged");
        let cache = Cache::new(&dir);
        assert_eq!(cache.store(&sample()), Ok(()));
        let text = to_text(&sample());
        // cut at every line boundary and in the middle of lines: never loads
        for (i, _) in text.match_indices('\n') {
            assert!(
                from_text(text.get(..i).unwrap_or("")).is_err(),
                "cut at {i}"
            );
            assert!(from_text(text.get(..i + 1).unwrap_or("")).is_err() || i + 1 == text.len());
        }
        assert!(from_text(&format!("{text}extra\n")).is_err());
        assert!(from_text(&text.replace("count\t6", "count\t7")).is_err());
        assert!(from_text(&text.replace("count\t6", "count\t99999999")).is_err());
        assert!(from_text("RCRAN2\n").is_err());
        assert!(from_text("RCRAN1\nfetched\tsoon\n").is_err());
        assert!(from_text(&text.replace("zoo\t", "zoo\\x\t")).is_err());
        // a damaged file on disk is reported and not deleted
        assert!(fs::write(cache.path(), "garbage").is_ok());
        assert!(cache.load().is_err());
        assert!(cache.path().exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_store_leaves_the_old_cache() {
        let dir = temp("failed");
        let cache = Cache::new(&dir);
        let ix = sample();
        assert_eq!(cache.store(&ix), Ok(()));
        // a folder where the temporary file should go makes the write fail
        assert!(fs::create_dir(dir.join(format!("{FILE_NAME}.tmp"))).is_ok());
        let newer = Index::new(vec![], 99, Detail::NamesOnly);
        assert!(cache.store(&newer).is_err());
        assert_eq!(cache.load(), Ok(Some(ix)));
        let _ = fs::remove_dir_all(&dir);
    }
}
