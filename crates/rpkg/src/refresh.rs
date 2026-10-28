//! Refreshing the index from a CRAN mirror.
//!
//! This is the only place that asks for the network, and only when the user
//! asked for a refresh (AGENTS.md: no background calls). The download itself is
//! a [`Fetcher`], so everything here is tested with a local stand-in and CI
//! needs no network.
//!
//! The rules, in order of importance:
//!
//! 1. A refresh that fails or is cancelled changes nothing: the old cache
//!    stays as it was.
//! 2. `PACKAGES.gz` (small, plain text) is fetched first. If `packages.rds`
//!    (large, has the titles) cannot be fetched or read, the result is a
//!    names-only index, but only when there is no full cache to lose; a full
//!    cache is never replaced by a names-only one.
//! 3. An empty list is never stored.

use crate::cache::Cache;
use crate::dcf;
use crate::index::{Detail, Index, DB_COLUMNS, DCF_FIELDS};
use crate::rds;
use crate::table;
use crate::views::Views;
use crate::{Error, Result};

/// The default mirror (the "cloud" mirror), until the user picks another.
pub const DEFAULT_MIRROR: &str = "https://cloud.r-project.org";
/// Path of the small plain-text list, under the mirror.
pub const PACKAGES_PATH: &str = "src/contrib/PACKAGES.gz";
/// Path of R's package database, under the mirror.
pub const DATABASE_PATH: &str = "web/packages/packages.rds";
/// Path of the task view list, under the mirror.
pub const VIEWS_PATH: &str = "src/contrib/Views.rds";
/// Most bytes accepted for `Views.rds` (it is about 70 KB).
pub const MAX_VIEWS_BYTES: u64 = 8 * 1024 * 1024;
/// Most bytes accepted for `PACKAGES.gz`.
pub const MAX_PACKAGES_BYTES: u64 = 64 * 1024 * 1024;
/// Most bytes accepted for `packages.rds`.
pub const MAX_DATABASE_BYTES: u64 = 256 * 1024 * 1024;

/// How far a download has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub stage: Stage,
    pub done: u64,
    /// The size, when the server said.
    pub total: Option<u64>,
}

/// Which file is being fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// `PACKAGES.gz`.
    Names,
    /// `packages.rds`.
    Details,
    /// `Views.rds`, the task views.
    Views,
}

/// What a progress callback answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Continue,
    Cancel,
}

/// Fetches one URL into memory.
pub trait Fetcher {
    /// The body of `url`, never more than `max_bytes`. `progress(done, total)`
    /// is called as bytes arrive; answering [`Control::Cancel`] stops the
    /// download and returns [`Error::Cancelled`].
    fn get(
        &self,
        url: &str,
        max_bytes: u64,
        progress: &mut dyn FnMut(u64, Option<u64>) -> Control,
    ) -> Result<Vec<u8>>;
}

/// What a successful refresh did.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub index: Index,
    /// Set when the details could not be had and the index has names only.
    pub note: Option<String>,
    /// The task views, when they could be had (the saved ones are kept when not).
    pub views: Option<Views>,
}

/// `mirror` and `path` joined with one slash.
pub fn url(mirror: &str, path: &str) -> String {
    format!(
        "{}/{}",
        mirror.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

/// Fetches the index from `mirror`, stores it in `cache` and returns it. `now`
/// is the time stamped on it (seconds since 1970).
pub fn refresh(
    fetcher: &dyn Fetcher,
    mirror: &str,
    cache: &Cache,
    now: u64,
    on_progress: &mut dyn FnMut(Progress) -> Control,
) -> Result<Outcome> {
    let names_bytes = fetcher.get(
        &url(mirror, PACKAGES_PATH),
        MAX_PACKAGES_BYTES,
        &mut |done, total| {
            on_progress(Progress {
                stage: Stage::Names,
                done,
                total,
            })
        },
    )?;
    let dcf_table = dcf::read(&names_bytes, DCF_FIELDS)?;
    if dcf_table.rows == 0 {
        return Err(Error::Format(
            "the package list from the mirror is empty".into(),
        ));
    }

    let details = fetcher
        .get(
            &url(mirror, DATABASE_PATH),
            MAX_DATABASE_BYTES,
            &mut |done, total| {
                on_progress(Progress {
                    stage: Stage::Details,
                    done,
                    total,
                })
            },
        )
        .and_then(|bytes| {
            let node = rds::read(&bytes)?;
            let t = table::from_node(&node, DB_COLUMNS)?;
            if t.rows == 0 {
                return Err(Error::Format("the package database is empty".into()));
            }
            Ok(t)
        });

    let (index, note) = match details {
        Ok(db) => (Index::from_tables(Some(&db), Some(&dcf_table), now), None),
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(e) => {
            // never trade a full cache for a names-only one
            if matches!(cache.load(), Ok(Some(old)) if old.detail == Detail::Full) {
                return Err(e);
            }
            (
                Index::from_tables(None, Some(&dcf_table), now),
                Some(format!(
                    "Titles and descriptions could not be loaded ({e}); package names and versions are available."
                )),
            )
        }
    };
    let views = fetcher
        .get(
            &url(mirror, VIEWS_PATH),
            MAX_VIEWS_BYTES,
            &mut |done, total| {
                on_progress(Progress {
                    stage: Stage::Views,
                    done,
                    total,
                })
            },
        )
        .and_then(|bytes| Views::from_node(&rds::read(&bytes)?));
    let (views, note) = match views {
        Ok(v) => (Some(v), note),
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(e) => {
            let extra =
                format!("The task views could not be loaded ({e}); the saved ones are kept.");
            (
                None,
                Some(note.map_or(extra.clone(), |n| format!("{n} {extra}"))),
            )
        }
    };
    cache.store(&index)?;
    if let Some(v) = &views {
        cache.store_views(v, now)?;
    }
    Ok(Outcome { index, note, views })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    const PACKAGES_GZ: &[u8] = include_bytes!("../tests/fixtures/PACKAGES.gz");
    const DB_V3: &[u8] = include_bytes!("../tests/fixtures/db_matrix_v3.rds");
    const VIEWS_RDS: &[u8] = include_bytes!("../tests/fixtures/Views.rds");

    /// Serves canned bodies; a URL not in the map is a failed request.
    struct Canned {
        bodies: HashMap<String, Result<Vec<u8>>>,
        asked: std::cell::RefCell<Vec<String>>,
    }

    impl Canned {
        fn new(names: Result<Vec<u8>>, db: Result<Vec<u8>>) -> Canned {
            let mut bodies = HashMap::new();
            bodies.insert(url("https://mirror.test", PACKAGES_PATH), names);
            bodies.insert(url("https://mirror.test", DATABASE_PATH), db);
            bodies.insert(
                url("https://mirror.test", VIEWS_PATH),
                Ok(VIEWS_RDS.to_vec()),
            );
            Canned {
                bodies,
                asked: Default::default(),
            }
        }
    }

    impl Fetcher for Canned {
        fn get(
            &self,
            url: &str,
            max_bytes: u64,
            progress: &mut dyn FnMut(u64, Option<u64>) -> Control,
        ) -> Result<Vec<u8>> {
            self.asked.borrow_mut().push(url.to_string());
            let body = self
                .bodies
                .get(url)
                .cloned()
                .unwrap_or_else(|| Err(Error::Fetch(format!("no such file {url}"))))?;
            if body.len() as u64 > max_bytes {
                return Err(Error::TooBig("body".into()));
            }
            if progress(body.len() as u64, Some(body.len() as u64)) == Control::Cancel {
                return Err(Error::Cancelled);
            }
            Ok(body)
        }
    }

    fn temp(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("rcraft-rpkg-refresh-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn go(f: &Canned, cache: &Cache, now: u64) -> Result<Outcome> {
        refresh(f, "https://mirror.test/", cache, now, &mut |_| {
            Control::Continue
        })
    }

    #[test]
    fn a_refresh_builds_the_index_and_stores_it() {
        let dir = temp("ok");
        let cache = Cache::new(&dir);
        let f = Canned::new(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec()));
        let out = go(&f, &cache, 1234);
        let out = out.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(out.note, None);
        assert_eq!(out.index.detail, Detail::Full);
        assert_eq!(out.index.fetched_at, 1234);
        let p = out.index.get("fakeone");
        assert_eq!(
            p.map(|p| p.title.as_str()),
            Some("Fake One: A Made-Up Package")
        );
        // in PACKAGES.gz only
        assert!(out.index.get("onlyindcf").is_some());
        assert_eq!(cache.load(), Ok(Some(out.index.clone())));
        // only the two documented files were asked for, the small one first
        assert_eq!(
            *f.asked.borrow(),
            vec![
                "https://mirror.test/src/contrib/PACKAGES.gz".to_string(),
                "https://mirror.test/web/packages/packages.rds".to_string(),
                "https://mirror.test/src/contrib/Views.rds".to_string()
            ]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_download_keeps_the_old_cache() {
        let dir = temp("fail");
        let cache = Cache::new(&dir);
        let old = go(
            &Canned::new(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec())),
            &cache,
            1,
        )
        .map(|o| o.index);
        let old = old.unwrap_or_else(|e| panic!("{e}"));
        // the names file fails
        let f = Canned::new(Err(Error::Fetch("offline".into())), Ok(DB_V3.to_vec()));
        assert_eq!(
            go(&f, &cache, 2).map(|o| o.index),
            Err(Error::Fetch("offline".into()))
        );
        // the names file is garbage
        let f = Canned::new(Ok(b"\x1f\x8b junk".to_vec()), Ok(DB_V3.to_vec()));
        assert!(go(&f, &cache, 3).is_err());
        // the names file lists nothing
        let f = Canned::new(Ok(Vec::new()), Ok(DB_V3.to_vec()));
        assert!(go(&f, &cache, 4).is_err());
        assert_eq!(cache.load(), Ok(Some(old)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_full_cache_is_never_replaced_by_a_names_only_index() {
        let dir = temp("keepfull");
        let cache = Cache::new(&dir);
        let first = go(
            &Canned::new(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec())),
            &cache,
            1,
        )
        .map(|o| o.index)
        .unwrap_or_else(|e| panic!("{e}"));
        let truncated = DB_V3.get(..DB_V3.len() / 2).unwrap_or(&[]).to_vec();
        for bad in [
            Err(Error::Fetch("timed out".into())),
            Ok(truncated.clone()),
            Ok(b"not an rds file".to_vec()),
        ] {
            let f = Canned::new(Ok(PACKAGES_GZ.to_vec()), bad);
            assert!(go(&f, &cache, 9).is_err());
            assert_eq!(cache.load(), Ok(Some(first.clone())));
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn with_no_cache_a_missing_database_gives_names_only_and_says_so() {
        let dir = temp("namesonly");
        let cache = Cache::new(&dir);
        let f = Canned::new(Ok(PACKAGES_GZ.to_vec()), Err(Error::Fetch("503".into())));
        let out = go(&f, &cache, 7).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(out.index.detail, Detail::NamesOnly);
        assert!(out.note.is_some_and(|n| n.contains("503")));
        assert_eq!(out.index.len(), 4);
        assert_eq!(
            cache.load().map(|c| c.map(|i| i.detail)),
            Ok(Some(Detail::NamesOnly))
        );
        // a later full refresh replaces it
        let f = Canned::new(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec()));
        assert_eq!(go(&f, &cache, 8).map(|o| o.index.detail), Ok(Detail::Full));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancelling_stops_and_changes_nothing() {
        let dir = temp("cancel");
        let cache = Cache::new(&dir);
        let f = Canned::new(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec()));
        let r = refresh(&f, "https://mirror.test", &cache, 1, &mut |_| {
            Control::Cancel
        });
        assert_eq!(r.map(|o| o.index.len()), Err(Error::Cancelled));
        assert_eq!(cache.load(), Ok(None));
        // cancelling during the second file is a cancel, not a names-only index
        let mut calls = 0;
        let r = refresh(&f, "https://mirror.test", &cache, 1, &mut |p| {
            calls += 1;
            if p.stage == Stage::Details {
                Control::Cancel
            } else {
                Control::Continue
            }
        });
        assert_eq!(r.map(|o| o.index.len()), Err(Error::Cancelled));
        assert_eq!(cache.load(), Ok(None));
        assert_eq!(calls, 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn urls_are_joined_with_one_slash() {
        assert_eq!(url("https://a.test/", "/x/y"), "https://a.test/x/y");
        assert_eq!(url("https://a.test", "x"), "https://a.test/x");
    }
}

/// Longest mirror address accepted.
pub const MAX_MIRROR_LEN: usize = 300;

/// Checks a mirror address a user typed: `https://` and a host, no spaces or
/// control characters, at most [`MAX_MIRROR_LEN`] long. Returns it without a
/// trailing slash. Plain `http://` is refused (the fetcher would refuse it too).
pub fn validate_mirror(text: &str) -> std::result::Result<String, String> {
    let t = text.trim().trim_end_matches('/');
    let Some(rest) = t.strip_prefix("https://") else {
        return Err("A mirror address starts with https://".into());
    };
    if t.len() > MAX_MIRROR_LEN {
        return Err(format!(
            "A mirror address is at most {MAX_MIRROR_LEN} characters"
        ));
    }
    if t.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("A mirror address has no spaces".into());
    }
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() || host.starts_with('.') || !host.chars().any(|c| c.is_alphanumeric()) {
        return Err("A mirror address needs a host name after https://".into());
    }
    if rest.contains(['?', '#']) {
        return Err("A mirror address has no query or fragment".into());
    }
    Ok(t.to_string())
}

#[cfg(test)]
mod mirror_tests {
    use super::*;

    #[test]
    fn mirrors_are_checked() {
        assert_eq!(
            validate_mirror("  https://cran.r-project.org/  "),
            Ok("https://cran.r-project.org".to_string())
        );
        assert_eq!(
            validate_mirror("https://mirror.example.edu/CRAN//"),
            Ok("https://mirror.example.edu/CRAN".to_string())
        );
        for bad in [
            "",
            "cran.r-project.org",
            "http://cran.r-project.org",
            "https://",
            "https:///x",
            "https://a b.org",
            "https://a.org/x?y=1",
            "https://a.org/#top",
            "ftp://a.org",
            "https://.org",
        ] {
            assert!(validate_mirror(bad).is_err(), "{bad:?}");
        }
        assert!(validate_mirror(&format!("https://a.org/{}", "x".repeat(400))).is_err());
        assert!(validate_mirror("https://a.org/\u{7}").is_err());
    }
}

/// A fetcher that never reaches anything: every request fails with a message.
/// For pictures and tests that must not touch the network.
pub struct Offline;

impl Fetcher for Offline {
    fn get(
        &self,
        url: &str,
        _max_bytes: u64,
        _progress: &mut dyn FnMut(u64, Option<u64>) -> Control,
    ) -> Result<Vec<u8>> {
        Err(Error::Fetch(format!("{url}: no network in this run")))
    }
}

#[cfg(test)]
mod views_tests {
    use super::*;
    use std::fs;

    const PACKAGES_GZ: &[u8] = include_bytes!("../tests/fixtures/PACKAGES.gz");
    const DB_V3: &[u8] = include_bytes!("../tests/fixtures/db_matrix_v3.rds");
    const VIEWS_RDS: &[u8] = include_bytes!("../tests/fixtures/Views.rds");

    struct Serve(Vec<(String, Result<Vec<u8>>)>);

    impl Fetcher for Serve {
        fn get(
            &self,
            url: &str,
            _max: u64,
            progress: &mut dyn FnMut(u64, Option<u64>) -> Control,
        ) -> Result<Vec<u8>> {
            let body = self
                .0
                .iter()
                .find(|(u, _)| u.as_str() == url)
                .map(|(_, b)| b.clone())
                .unwrap_or_else(|| Err(Error::Fetch(format!("no such file {url}"))))?;
            if progress(body.len() as u64, None) == Control::Cancel {
                return Err(Error::Cancelled);
            }
            Ok(body)
        }
    }

    fn serve(views: Result<Vec<u8>>) -> Serve {
        Serve(vec![
            (
                url("https://m.test", PACKAGES_PATH),
                Ok(PACKAGES_GZ.to_vec()),
            ),
            (url("https://m.test", DATABASE_PATH), Ok(DB_V3.to_vec())),
            (url("https://m.test", VIEWS_PATH), views),
        ])
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("rcraft-rpkg-views-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn task_views_arrive_with_the_refresh_and_are_kept_for_next_time() {
        let dir = temp("ok");
        let cache = Cache::new(&dir);
        let out = refresh(
            &serve(Ok(VIEWS_RDS.to_vec())),
            "https://m.test",
            &cache,
            77,
            &mut |_| Control::Continue,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(out.note, None);
        let v = out.views.clone().unwrap_or_default();
        assert_eq!(v.views().len(), 2);
        assert_eq!(cache.load_views(), Ok(Some((v, 77))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_or_damaged_views_do_not_fail_the_refresh_and_keep_the_old_ones() {
        let dir = temp("bad");
        let cache = Cache::new(&dir);
        let first = refresh(
            &serve(Ok(VIEWS_RDS.to_vec())),
            "https://m.test",
            &cache,
            1,
            &mut |_| Control::Continue,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        for bad in [Err(Error::Fetch("503".into())), Ok(b"junk".to_vec())] {
            let out = refresh(&serve(bad), "https://m.test", &cache, 2, &mut |_| {
                Control::Continue
            })
            .unwrap_or_else(|e| panic!("{e}"));
            assert!(out.views.is_none());
            assert!(
                out.note
                    .as_deref()
                    .is_some_and(|n| n.contains("task views")),
                "{:?}",
                out.note
            );
            assert_eq!(out.index.len(), 4, "the package list itself is fine");
            assert_eq!(
                cache.load_views(),
                Ok(Some((first.views.clone().unwrap_or_default(), 1)))
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancelling_during_the_views_changes_nothing() {
        let dir = temp("cancel");
        let cache = Cache::new(&dir);
        let r = refresh(
            &serve(Ok(VIEWS_RDS.to_vec())),
            "https://m.test",
            &cache,
            1,
            &mut |p| {
                if p.stage == Stage::Views {
                    Control::Cancel
                } else {
                    Control::Continue
                }
            },
        );
        assert_eq!(r.map(|o| o.index.len()), Err(Error::Cancelled));
        assert_eq!(cache.load(), Ok(None));
        let _ = fs::remove_dir_all(&dir);
    }
}
