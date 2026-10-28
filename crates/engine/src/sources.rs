//! Looking up a package source beyond CRAN, on a worker thread: a GitHub
//! repository's DESCRIPTION, an r-universe's package list, or Bioconductor's
//! list for this R. Only on the user's request, through the same `Fetcher` as
//! the CRAN list (HTTPS only, capped, cancellable). Local files are read by R,
//! not here.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;

use rcraft_rpkg::dcf;
use rcraft_rpkg::http::UreqFetcher;
use rcraft_rpkg::index::{Index, Package, DB_COLUMNS};
use rcraft_rpkg::provider::{
    bioc_release_for, bioc_views_url, github_description_url, universe_url, Provider,
    BIOC_CONFIG_URL,
};
use rcraft_rpkg::refresh::{url, Control, Fetcher, PACKAGES_PATH};
use rcraft_rpkg::search::Searcher;

/// Most bytes accepted for a DESCRIPTION.
const MAX_DESCRIPTION: u64 = 256 * 1024;
/// Most bytes accepted for Bioconductor's config.
const MAX_CONFIG: u64 = 1024 * 1024;
/// Most bytes accepted for a package list.
const MAX_LIST: u64 = 64 * 1024 * 1024;

/// What to look up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    GitHub {
        owner: String,
        repo: String,
        reference: String,
    },
    Universe(String),
    /// Bioconductor's list for the R of this version.
    Bioconductor {
        r_version: String,
    },
}

impl Lookup {
    /// The addresses this will ask, for the window to say before it asks.
    pub fn addresses(&self) -> Vec<String> {
        match self {
            Lookup::GitHub {
                owner,
                repo,
                reference,
            } => vec![github_description_url(owner, repo, reference)],
            Lookup::Universe(u) => vec![url(&universe_url(u), PACKAGES_PATH)],
            Lookup::Bioconductor { .. } => vec![
                BIOC_CONFIG_URL.to_string(),
                "https://bioconductor.org/packages/<release>/bioc/VIEWS".to_string(),
            ],
        }
    }
}

/// What a lookup found.
pub enum Found {
    /// A GitHub repository's package, from its DESCRIPTION.
    Description {
        provider: Provider,
        package: Package,
    },
    /// A list of packages (an r-universe, or Bioconductor's release `release`).
    List {
        provider: Provider,
        release: Option<String>,
        index: Index,
        searcher: Searcher,
    },
}

pub enum LookupState {
    Idle,
    Running(Lookup),
    Done(Found),
    Failed(String),
}

pub struct SourceLookups {
    fetcher: Arc<dyn Fetcher + Send + Sync>,
    job: Option<(Receiver<Result<Found, String>>, Arc<AtomicBool>)>,
    state: LookupState,
}

fn get(
    fetcher: &dyn Fetcher,
    address: &str,
    max: u64,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, String> {
    fetcher
        .get(address, max, &mut |_, _| {
            if cancel.load(Ordering::Relaxed) {
                Control::Cancel
            } else {
                Control::Continue
            }
        })
        .map_err(|e| e.to_string())
}

fn list_from(bytes: &[u8]) -> Result<(Index, Searcher), String> {
    let table = dcf::read(bytes, DB_COLUMNS).map_err(|e| e.to_string())?;
    if table.rows == 0 {
        return Err("the list has no packages".into());
    }
    let index = Index::from_tables(Some(&table), None, crate::package_index::unix_now());
    let searcher = Searcher::new(&index);
    Ok((index, searcher))
}

/// Does a lookup (on the worker thread).
fn work(fetcher: &dyn Fetcher, lookup: &Lookup, cancel: &AtomicBool) -> Result<Found, String> {
    match lookup {
        Lookup::GitHub {
            owner,
            repo,
            reference,
        } => {
            let bytes = get(
                fetcher,
                &github_description_url(owner, repo, reference),
                MAX_DESCRIPTION,
                cancel,
            )
            .map_err(|e| format!("{e}. Check the owner, repository and branch."))?;
            let text = String::from_utf8_lossy(&bytes);
            let table = dcf::table(&text, DB_COLUMNS).map_err(|e| e.to_string())?;
            let index = Index::from_tables(Some(&table), None, 0);
            let package = index
                .packages()
                .first()
                .cloned()
                .ok_or("that repository has no R package DESCRIPTION at its top")?;
            Ok(Found::Description {
                provider: Provider::GitHub {
                    owner: owner.clone(),
                    repo: repo.clone(),
                    reference: reference.clone(),
                },
                package,
            })
        }
        Lookup::Universe(u) => {
            let bytes = get(
                fetcher,
                &url(&universe_url(u), PACKAGES_PATH),
                MAX_LIST,
                cancel,
            )?;
            let (index, searcher) = list_from(&bytes)?;
            Ok(Found::List {
                provider: Provider::Universe(u.clone()),
                release: None,
                index,
                searcher,
            })
        }
        Lookup::Bioconductor { r_version } => {
            let config = get(fetcher, BIOC_CONFIG_URL, MAX_CONFIG, cancel)?;
            let release = bioc_release_for(&String::from_utf8_lossy(&config), r_version)
                .ok_or_else(|| {
                    format!("Bioconductor has no release for R {r_version} (yet, or any more).")
                })?;
            let bytes = get(fetcher, &bioc_views_url(&release), MAX_LIST, cancel)?;
            let (index, searcher) = list_from(&bytes)?;
            Ok(Found::List {
                provider: Provider::Bioconductor,
                release: Some(release),
                index,
                searcher,
            })
        }
    }
}

impl SourceLookups {
    pub fn new(fetcher: Arc<dyn Fetcher + Send + Sync>) -> SourceLookups {
        SourceLookups {
            fetcher,
            job: None,
            state: LookupState::Idle,
        }
    }

    pub fn for_machine() -> SourceLookups {
        SourceLookups::new(Arc::new(UreqFetcher::new()))
    }

    pub fn state(&self) -> &LookupState {
        &self.state
    }

    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    /// Starts a lookup, dropping what an earlier one found. Does nothing while
    /// one is running.
    pub fn start(&mut self, lookup: Lookup) {
        if self.job.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let fetcher = self.fetcher.clone();
        let flag = cancel.clone();
        let l = lookup.clone();
        thread::spawn(move || {
            let _ = tx.send(work(&*fetcher, &l, &flag));
        });
        self.job = Some((rx, cancel));
        self.state = LookupState::Running(lookup);
    }

    pub fn cancel(&mut self) {
        if let Some((_, c)) = &self.job {
            c.store(true, Ordering::Relaxed);
        }
    }

    /// Forgets what was found.
    pub fn clear(&mut self) {
        if self.job.is_none() {
            self.state = LookupState::Idle;
        }
    }

    /// Takes in the worker's answer. Returns whether anything changed.
    pub fn poll(&mut self) -> bool {
        let Some((rx, _)) = &self.job else {
            return false;
        };
        match rx.try_recv() {
            Ok(r) => {
                self.state = match r {
                    Ok(found) => LookupState::Done(found),
                    Err(e) if e.contains("cancelled") => LookupState::Idle,
                    Err(e) => LookupState::Failed(e),
                };
                self.job = None;
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.state = LookupState::Failed("The lookup stopped unexpectedly.".into());
                self.job = None;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rpkg::{Error, Result};
    use std::time::{Duration, Instant};

    const PACKAGES_GZ: &[u8] = include_bytes!("../../rpkg/tests/fixtures/PACKAGES.gz");

    const DESCRIPTION: &str = "Package: madeup\nVersion: 0.3.0.9000\nTitle: A Made-Up Package on GitHub\nImports: fakeone,\n    stats\nLinkingTo: fakethree\nLicense: MIT\nNeedsCompilation: yes\n";

    const CONFIG: &str = "release_version: \"3.20\"\ndevel_version: \"3.21\"\nr_ver_for_bioc_ver:\n    \"3.19\": \"4.4\"\n    \"3.20\": \"4.4\"\n    \"3.21\": \"4.5\"\n";

    const VIEWS: &str = "Package: fakebioc\nVersion: 1.2.0\nTitle: Fake Bioconductor Tools\nImports: limma\n\nPackage: limma\nVersion: 3.60.0\nTitle: Linear Models for Microarray Data\n";

    struct Canned(Vec<(String, Vec<u8>)>);

    impl Fetcher for Canned {
        fn get(
            &self,
            url: &str,
            _max: u64,
            progress: &mut dyn FnMut(u64, Option<u64>) -> Control,
        ) -> Result<Vec<u8>> {
            if progress(0, None) == Control::Cancel {
                return Err(Error::Cancelled);
            }
            self.0
                .iter()
                .find(|(u, _)| u == url)
                .map(|(_, b)| b.clone())
                .ok_or_else(|| Error::Fetch(format!("{url}: the server answered 404")))
        }
    }

    fn served() -> SourceLookups {
        SourceLookups::new(Arc::new(Canned(vec![
            (
                github_description_url("me", "madeup", "HEAD"),
                DESCRIPTION.as_bytes().to_vec(),
            ),
            (
                url(&universe_url("someone"), PACKAGES_PATH),
                PACKAGES_GZ.to_vec(),
            ),
            (BIOC_CONFIG_URL.to_string(), CONFIG.as_bytes().to_vec()),
            (bioc_views_url("3.20"), VIEWS.as_bytes().to_vec()),
        ])))
    }

    fn finish(s: &mut SourceLookups) {
        let started = Instant::now();
        while s.is_busy() {
            s.poll();
            assert!(started.elapsed() < Duration::from_secs(20));
            thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn a_github_description_is_read_into_a_package() {
        let mut s = served();
        s.start(Lookup::GitHub {
            owner: "me".into(),
            repo: "madeup".into(),
            reference: "HEAD".into(),
        });
        finish(&mut s);
        let LookupState::Done(Found::Description { provider, package }) = s.state() else {
            panic!("nothing found");
        };
        assert!(matches!(provider, Provider::GitHub { repo, .. } if repo == "madeup"));
        assert_eq!(package.name, "madeup");
        assert_eq!(package.version, "0.3.0.9000");
        let needs: Vec<String> = package
            .hard_dependencies()
            .into_iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(needs, vec!["fakeone", "stats", "fakethree"]);
    }

    #[test]
    fn a_universe_list_is_read_and_searchable() {
        let mut s = served();
        s.start(Lookup::Universe("someone".into()));
        finish(&mut s);
        let LookupState::Done(Found::List {
            index,
            searcher,
            provider,
            release,
        }) = s.state()
        else {
            panic!("nothing found");
        };
        assert_eq!(*provider, Provider::Universe("someone".into()));
        assert_eq!(*release, None);
        assert_eq!(index.len(), 4);
        assert_eq!(searcher.len(), 4);
    }

    #[test]
    fn bioconductor_picks_the_release_for_this_r_and_reads_its_list() {
        let mut s = served();
        s.start(Lookup::Bioconductor {
            r_version: "4.4.3".into(),
        });
        finish(&mut s);
        let LookupState::Done(Found::List { index, release, .. }) = s.state() else {
            panic!("nothing found");
        };
        assert_eq!(release.as_deref(), Some("3.20"));
        assert_eq!(
            index.get("limma").map(|p| p.title.as_str()),
            Some("Linear Models for Microarray Data")
        );
        // an R that Bioconductor has no release for says so
        s.start(Lookup::Bioconductor {
            r_version: "4.9.0".into(),
        });
        finish(&mut s);
        assert!(
            matches!(s.state(), LookupState::Failed(m) if m.contains("no release for R 4.9.0"))
        );
    }

    #[test]
    fn a_missing_repository_is_a_failure_in_words() {
        let mut s = served();
        s.start(Lookup::GitHub {
            owner: "me".into(),
            repo: "nothere".into(),
            reference: "HEAD".into(),
        });
        finish(&mut s);
        assert!(
            matches!(s.state(), LookupState::Failed(m) if m.contains("404") && m.contains("Check the owner")),
        );
        // the addresses are said before asking
        let a = Lookup::Universe("ropensci".into()).addresses();
        assert_eq!(
            a,
            vec!["https://ropensci.r-universe.dev/src/contrib/PACKAGES.gz"]
        );
    }
}
