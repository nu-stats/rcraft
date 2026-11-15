//! CRAN's archive of older versions: `src/contrib/Archive/<package>/` on a
//! mirror, a plain directory listing with one `<package>_<version>.tar.gz`
//! per earlier release (checked 2026-11-14 on the cloud mirror; the current
//! version is not in it). A package with only one release has no folder, and
//! the mirror answers 404.
//!
//! The listing is read as text and only file names of the expected form are
//! taken; nothing else in the page is used.

use std::cmp::Ordering;

use crate::index::is_package_name;
use crate::plan::compare_versions;

/// Most versions taken from one listing.
const MAX_VERSIONS: usize = 2000;

/// One older version in the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archived {
    pub version: String,
    /// The day the file was put there (`YYYY-MM-DD`), when the listing says.
    pub date: Option<String>,
}

/// The address of a package's folder in the archive, or `None` for a name
/// that is not a package name.
pub fn archive_folder_url(mirror: &str, package: &str) -> Option<String> {
    is_package_name(package).then(|| {
        format!(
            "{}/src/contrib/Archive/{package}/",
            mirror.trim_end_matches('/')
        )
    })
}

/// A version as CRAN's file names have it: starts with a digit, then digits,
/// dots and dashes.
fn is_cran_version(s: &str) -> bool {
    s.len() <= 64
        && s.starts_with(|c: char| c.is_ascii_digit())
        && s.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
}

/// `YYYY-MM-DD` at the start of `s` (after spaces), if there.
fn leading_day(s: &str) -> Option<String> {
    let s = s.trim_start();
    let day = s.get(..10)?;
    let b = day.as_bytes();
    let ok = b.iter().enumerate().all(|(i, c)| match i {
        4 | 7 => *c == b'-',
        _ => c.is_ascii_digit(),
    });
    ok.then(|| day.to_string())
}

/// The versions a folder listing names for `package`, newest first, each once.
pub fn archived_versions(listing: &str, package: &str) -> Vec<Archived> {
    let prefix = format!("href=\"{package}_");
    let mut out: Vec<Archived> = Vec::new();
    let mut rest = listing;
    while let Some(at) = rest.find(&prefix) {
        rest = rest.get(at + prefix.len()..).unwrap_or("");
        let Some(end) = rest.find('"') else {
            break;
        };
        let file = rest.get(..end).unwrap_or("");
        rest = rest.get(end..).unwrap_or("");
        let Some(version) = file.strip_suffix(".tar.gz") else {
            continue;
        };
        if !is_cran_version(version) || out.iter().any(|a| a.version == version) {
            continue;
        }
        // the date follows the link's closing tag on the same line
        let line = rest.split('\n').next().unwrap_or("");
        let date = line
            .find("</a>")
            .and_then(|i| line.get(i + 4..))
            .and_then(leading_day);
        out.push(Archived {
            version: version.to_string(),
            date,
        });
        if out.len() >= MAX_VERSIONS {
            break;
        }
    }
    out.sort_by(|a, b| match compare_versions(&b.version, &a.version) {
        Ordering::Equal => b.version.cmp(&a.version),
        other => other,
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = r#"<html><head><title>Index of /src/contrib/Archive/fixest</title></head><body>
<pre>      <a href="?C=N;O=D">Name</a>                    <a href="?C=M;O=A">Last modified</a>      <a href="?C=S;O=A">Size</a>  <hr>      <a href="/src/contrib/Archive/">Parent Directory</a>                             -
      <a href="fixest_0.1.0.tar.gz">fixest_0.1.0.tar.gz</a>     2019-09-03 07:30  396K
      <a href="fixest_0.10.0.tar.gz">fixest_0.10.0.tar.gz</a>    2021-09-28 12:10  2.2M
      <a href="fixest_0.9.0.tar.gz">fixest_0.9.0.tar.gz</a>    2021-04-01 10:00  2.0M
      <a href="fixest_0.9.0.tar.gz">again</a>
      <a href="fixest_1.0-2.tar.gz">fixest_1.0-2.tar.gz</a>
      <a href="fixest_x.tar.gz">fixest_x.tar.gz</a>    2021-04-01 10:00  2.0M
      <a href="fixest_0.2.0.zip">fixest_0.2.0.zip</a>    2021-04-01 10:00  2.0M
      <a href="fixest_0.3.0.tar.gz/../evil">x</a>
      <a href="fixestx_0.4.0.tar.gz">fixestx_0.4.0.tar.gz</a>    2021-04-01 10:00  2.0M
</pre></body></html>"#;

    #[test]
    fn versions_are_read_newest_first_with_their_days() {
        let v = archived_versions(LISTING, "fixest");
        let names: Vec<&str> = v.iter().map(|a| a.version.as_str()).collect();
        assert_eq!(names, vec!["1.0-2", "0.10.0", "0.9.0", "0.1.0"]);
        assert_eq!(v[0].date, None);
        assert_eq!(v[1].date.as_deref(), Some("2021-09-28"));
        assert_eq!(v[3].date.as_deref(), Some("2019-09-03"));
    }

    #[test]
    fn nothing_odd_is_taken() {
        assert!(archived_versions("", "fixest").is_empty());
        assert!(archived_versions("<a href=\"fixest_1.0.tar.gz", "fixest").is_empty());
        assert!(archived_versions(LISTING, "other").is_empty());
        assert_eq!(
            archive_folder_url("https://cloud.r-project.org/", "fixest").as_deref(),
            Some("https://cloud.r-project.org/src/contrib/Archive/fixest/")
        );
        assert_eq!(archive_folder_url("https://x.org", "../etc"), None);
    }
}
