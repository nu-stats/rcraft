//! Unpacking gzip with a cap on the size, so a hostile file cannot fill memory.

use std::io::Read;

use flate2::read::MultiGzDecoder;

use crate::{Error, Result};

/// Most bytes accepted after unpacking: the real package database is a few
/// tens of megabytes.
pub const MAX_UNPACKED_BYTES: u64 = 512 * 1024 * 1024;

/// Whether `bytes` start with the gzip magic number.
pub fn is_gzip(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b])
}

/// `bytes` unpacked if gzip, else as they are, never more than `max` bytes.
pub fn unpack(bytes: &[u8], max: u64) -> Result<Vec<u8>> {
    if !is_gzip(bytes) {
        if bytes.len() as u64 > max {
            return Err(Error::TooBig(format!("more than {max} bytes")));
        }
        return Ok(bytes.to_vec());
    }
    let mut out = Vec::new();
    let mut limited = MultiGzDecoder::new(bytes).take(max.saturating_add(1));
    limited
        .read_to_end(&mut out)
        .map_err(|e| Error::Format(format!("damaged gzip data ({e})")))?;
    if out.len() as u64 > max {
        return Err(Error::TooBig(format!(
            "more than {max} bytes after unpacking"
        )));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn gz(data: &[u8]) -> Vec<u8> {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        assert!(e.write_all(data).is_ok());
        e.finish().unwrap_or_default()
    }

    #[test]
    fn plain_bytes_pass_through_and_gzip_is_unpacked() {
        assert_eq!(unpack(b"abc", 100), Ok(b"abc".to_vec()));
        assert_eq!(
            unpack(&gz(b"hello hello hello"), 100),
            Ok(b"hello hello hello".to_vec())
        );
    }

    #[test]
    fn the_size_cap_applies_before_and_after_unpacking() {
        assert!(matches!(unpack(b"abcdef", 3), Err(Error::TooBig(_))));
        // 10,000 zeros pack to a few dozen bytes
        let bomb = gz(&vec![0u8; 10_000]);
        assert!(bomb.len() < 100);
        assert!(matches!(unpack(&bomb, 5_000), Err(Error::TooBig(_))));
        assert_eq!(unpack(&bomb, 10_000).map(|v| v.len()), Ok(10_000));
    }

    #[test]
    fn damaged_gzip_is_an_error() {
        let mut bad = gz(b"some text to damage, some text to damage");
        let n = bad.len();
        bad.truncate(n - 12);
        assert!(unpack(&bad, 1000).is_err());
        assert!(unpack(&[0x1f, 0x8b, 0, 0], 1000).is_err());
    }
}
