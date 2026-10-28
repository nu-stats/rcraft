//! The CRAN package index as the window uses it: loaded from the cache in the
//! background, refreshed from the mirror only when the user asks, and never
//! needing R.
//!
//! Loading and refreshing run on a worker thread so the window stays alive; the
//! window calls [`PackageIndexService::poll`] each frame to take in what the
//! worker reports. Nothing here contacts the network by itself: a refresh
//! starts only from [`PackageIndexService::refresh`] (AGENTS.md: no background
//! calls), and the index is offered as stale, never refreshed on its own.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use rcraft_rhost::library::app_data_dir;
use rcraft_rhost::Env;
use rcraft_rpkg::cache::Cache;
use rcraft_rpkg::http::UreqFetcher;
use rcraft_rpkg::index::{Index, DEFAULT_MAX_AGE};
use rcraft_rpkg::refresh::{
    self, validate_mirror, Control, Fetcher, Outcome, Progress, DEFAULT_MIRROR,
};

/// What the service is doing.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Idle,
    /// Reading the cache file.
    Loading,
    /// Downloading; `progress` is the latest report, once there is one.
    Refreshing {
        progress: Option<Progress>,
    },
    /// The last load or refresh failed, in words for the user. The index, if
    /// there was one, is still the old one.
    Failed(String),
}

enum Msg {
    Progress(Progress),
    Loaded(Result<Option<Index>, String>),
    Refreshed(Result<Outcome, rcraft_rpkg::Error>),
}

struct Job {
    messages: Receiver<Msg>,
    cancel: Arc<AtomicBool>,
}

/// Seconds since 1970 now (0 if the clock is before that).
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct PackageIndexService {
    cache: Option<Cache>,
    mirror_file: Option<PathBuf>,
    mirror: String,
    index: Option<Index>,
    status: Status,
    /// A note from the last refresh (for example, that titles were missing).
    note: Option<String>,
    job: Option<Job>,
    /// Whether the cache has been looked at yet.
    loaded_once: bool,
    fetcher: Arc<dyn Fetcher + Send + Sync>,
    clock: fn() -> u64,
}

impl PackageIndexService {
    /// The service for this machine: the cache and the mirror setting live in
    /// RCraft's own folder; downloads go through HTTPS.
    pub fn for_machine(env: &Env) -> PackageIndexService {
        let dir = app_data_dir(env);
        PackageIndexService::new(
            dir.as_ref().map(|d| d.join("package-index")),
            dir.as_ref().map(|d| d.join("cran-mirror.txt")),
            Arc::new(UreqFetcher::new()),
            unix_now,
        )
    }

    /// A service with its parts chosen by the caller (tests use stand-ins).
    /// With no `cache_dir` the index lives in memory for this run only.
    pub fn new(
        cache_dir: Option<PathBuf>,
        mirror_file: Option<PathBuf>,
        fetcher: Arc<dyn Fetcher + Send + Sync>,
        clock: fn() -> u64,
    ) -> PackageIndexService {
        let mirror = mirror_file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|t| validate_mirror(&t).ok())
            .unwrap_or_else(|| DEFAULT_MIRROR.to_string());
        PackageIndexService {
            cache: cache_dir.map(Cache::new),
            mirror_file,
            mirror,
            index: None,
            status: Status::Idle,
            note: None,
            job: None,
            loaded_once: false,
            fetcher,
            clock,
        }
    }

    pub fn index(&self) -> Option<&Index> {
        self.index.as_ref()
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn mirror(&self) -> &str {
        &self.mirror
    }

    /// Whether a worker is running (so the window should keep redrawing).
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    /// Whether the index is missing or older than a week: the window offers a
    /// refresh, and does not make one.
    pub fn refresh_suggested(&self) -> bool {
        match &self.index {
            None => true,
            Some(ix) => ix.is_stale((self.clock)(), DEFAULT_MAX_AGE),
        }
    }

    /// Changes the mirror, checking it, and keeps it for next time.
    pub fn set_mirror(&mut self, text: &str) -> Result<(), String> {
        let m = validate_mirror(text)?;
        if let Some(f) = &self.mirror_file {
            if let Some(dir) = f.parent() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| format!("could not save the mirror: {e}"))?;
            }
            std::fs::write(f, format!("{m}\n"))
                .map_err(|e| format!("could not save the mirror: {e}"))?;
        }
        self.mirror = m;
        Ok(())
    }

    /// Goes back to the default mirror.
    pub fn reset_mirror(&mut self) {
        if let Some(f) = &self.mirror_file {
            let _ = std::fs::remove_file(f);
        }
        self.mirror = DEFAULT_MIRROR.to_string();
    }

    /// Starts reading the cache, once. Call it when the package screens open.
    pub fn ensure_loaded(&mut self) {
        if self.loaded_once || self.job.is_some() {
            return;
        }
        self.loaded_once = true;
        let Some(cache) = self.cache.clone() else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.job = Some(Job {
            messages: rx,
            cancel,
        });
        self.status = Status::Loading;
        thread::spawn(move || {
            let r = cache.load().map_err(|e| e.to_string());
            let _ = tx.send(Msg::Loaded(r));
        });
    }

    /// Starts a refresh from the mirror. Does nothing while a worker is busy.
    pub fn refresh(&mut self) {
        if self.job.is_some() {
            return;
        }
        self.loaded_once = true;
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.job = Some(Job {
            messages: rx,
            cancel: cancel.clone(),
        });
        self.status = Status::Refreshing { progress: None };
        self.note = None;
        let fetcher = self.fetcher.clone();
        let mirror = self.mirror.clone();
        let now = (self.clock)();
        // with no folder to keep it in, the cache is a throwaway one
        let cache = self.cache.clone().unwrap_or_else(|| {
            Cache::new(std::env::temp_dir().join(format!("rcraft-index-{}", std::process::id())))
        });
        let keep_on_disk = self.cache.is_some();
        thread::spawn(move || {
            let tx_progress = tx.clone();
            let result = refresh::refresh(&*fetcher, &mirror, &cache, now, &mut |p| {
                if cancel.load(Ordering::Relaxed) || tx_progress.send(Msg::Progress(p)).is_err() {
                    Control::Cancel
                } else {
                    Control::Continue
                }
            });
            if !keep_on_disk {
                let _ = std::fs::remove_dir_all(cache_dir_of(&cache));
            }
            let _ = tx.send(Msg::Refreshed(result));
        });
    }

    /// Asks the worker to stop; the refresh ends as cancelled and changes nothing.
    pub fn cancel(&mut self) {
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Takes in what the worker has reported. Returns whether anything changed
    /// (so the window can redraw).
    pub fn poll(&mut self) -> bool {
        let Some(job) = &self.job else {
            return false;
        };
        let mut changed = false;
        let mut finished = false;
        loop {
            match job.messages.try_recv() {
                Ok(Msg::Progress(p)) => {
                    self.status = Status::Refreshing { progress: Some(p) };
                    changed = true;
                }
                Ok(Msg::Loaded(r)) => {
                    match r {
                        Ok(Some(ix)) => {
                            self.index = Some(ix);
                            self.status = Status::Idle;
                        }
                        Ok(None) => self.status = Status::Idle,
                        Err(e) => {
                            self.status = Status::Failed(format!(
                                "The saved package list could not be read ({e}). Refresh to download it again."
                            ));
                        }
                    }
                    finished = true;
                    changed = true;
                }
                Ok(Msg::Refreshed(r)) => {
                    match r {
                        Ok(out) => {
                            self.index = Some(out.index);
                            self.note = out.note;
                            self.status = Status::Idle;
                        }
                        Err(rcraft_rpkg::Error::Cancelled) => self.status = Status::Idle,
                        Err(e) => self.status = Status::Failed(e.to_string()),
                    }
                    finished = true;
                    changed = true;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    // the worker ended without a result (it panicked)
                    if !finished {
                        self.status =
                            Status::Failed("The package list job stopped unexpectedly.".into());
                        changed = true;
                    }
                    finished = true;
                    break;
                }
            }
            if finished {
                break;
            }
        }
        if finished {
            self.job = None;
        }
        changed
    }
}

fn cache_dir_of(cache: &Cache) -> PathBuf {
    cache.path().parent().map(PathBuf::from).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcraft_rpkg::index::Detail;
    use rcraft_rpkg::refresh::{url, DATABASE_PATH, PACKAGES_PATH};
    use rcraft_rpkg::{Error, Result};
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    const PACKAGES_GZ: &[u8] = include_bytes!("../../rpkg/tests/fixtures/PACKAGES.gz");
    const DB_V3: &[u8] = include_bytes!("../../rpkg/tests/fixtures/db_matrix_v3.rds");

    struct Canned {
        bodies: HashMap<String, Result<Vec<u8>>>,
        /// Wait this long inside `get`, in steps, noticing a cancel.
        slow: Duration,
    }

    impl Canned {
        fn serving(names: Result<Vec<u8>>, db: Result<Vec<u8>>) -> Canned {
            let mut bodies = HashMap::new();
            bodies.insert(url("https://mirror.test", PACKAGES_PATH), names);
            bodies.insert(url("https://mirror.test", DATABASE_PATH), db);
            Canned {
                bodies,
                slow: Duration::ZERO,
            }
        }
    }

    impl Fetcher for Canned {
        fn get(
            &self,
            url: &str,
            _max: u64,
            progress: &mut dyn FnMut(u64, Option<u64>) -> Control,
        ) -> Result<Vec<u8>> {
            let started = Instant::now();
            while started.elapsed() < self.slow {
                if progress(0, None) == Control::Cancel {
                    return Err(Error::Cancelled);
                }
                thread::sleep(Duration::from_millis(5));
            }
            let body = self
                .bodies
                .get(url)
                .cloned()
                .unwrap_or_else(|| Err(Error::Fetch(format!("no such file {url}"))))?;
            if progress(body.len() as u64, Some(body.len() as u64)) == Control::Cancel {
                return Err(Error::Cancelled);
            }
            Ok(body)
        }
    }

    fn temp(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("rcraft-engine-pkg-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn clock_at_1000() -> u64 {
        1000
    }

    fn service(dir: &std::path::Path, f: Canned) -> PackageIndexService {
        let mut s = PackageIndexService::new(
            Some(dir.join("idx")),
            Some(dir.join("mirror.txt")),
            Arc::new(f),
            clock_at_1000,
        );
        assert_eq!(s.set_mirror("https://mirror.test/"), Ok(()));
        s
    }

    /// Polls until the worker is done (or the test times out).
    fn finish(s: &mut PackageIndexService) {
        let started = Instant::now();
        while s.is_busy() {
            s.poll();
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "worker never finished"
            );
            thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn nothing_is_fetched_until_a_refresh_is_asked_for() {
        let dir = temp("idle");
        let mut s = service(
            &dir,
            Canned::serving(
                Err(Error::Fetch("boom".into())),
                Err(Error::Fetch("boom".into())),
            ),
        );
        s.ensure_loaded();
        finish(&mut s);
        assert_eq!(s.status(), &Status::Idle);
        assert!(s.index().is_none());
        assert!(s.refresh_suggested());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_refresh_loads_reports_progress_and_survives_a_restart() {
        let dir = temp("refresh");
        let mut s = service(
            &dir,
            Canned::serving(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec())),
        );
        s.refresh();
        assert!(matches!(s.status(), Status::Refreshing { .. }));
        // a second refresh while one runs does nothing
        s.refresh();
        finish(&mut s);
        assert_eq!(s.status(), &Status::Idle);
        let ix = s.index().cloned();
        assert_eq!(ix.as_ref().map(|i| i.detail), Some(Detail::Full));
        assert_eq!(ix.as_ref().map(|i| i.fetched_at), Some(1000));
        assert!(!s.refresh_suggested());
        // a new service on the same folder reads the cache with no network
        let mut again = service(
            &dir,
            Canned::serving(
                Err(Error::Fetch("offline".into())),
                Err(Error::Fetch("offline".into())),
            ),
        );
        again.ensure_loaded();
        assert_eq!(again.status(), &Status::Loading);
        finish(&mut again);
        assert_eq!(again.index().cloned(), ix);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_refresh_is_reported_and_keeps_the_index() {
        let dir = temp("failed");
        let mut s = service(
            &dir,
            Canned::serving(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec())),
        );
        s.refresh();
        finish(&mut s);
        let before = s.index().cloned();
        let mut bad = service(
            &dir,
            Canned::serving(
                Err(Error::Fetch("offline".into())),
                Err(Error::Fetch("offline".into())),
            ),
        );
        bad.ensure_loaded();
        finish(&mut bad);
        bad.refresh();
        finish(&mut bad);
        assert!(matches!(bad.status(), Status::Failed(m) if m.contains("offline")));
        assert_eq!(bad.index().cloned(), before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancelling_ends_quietly_and_changes_nothing() {
        let dir = temp("cancel");
        let mut f = Canned::serving(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec()));
        f.slow = Duration::from_secs(30);
        let mut s = service(&dir, f);
        s.refresh();
        s.cancel();
        finish(&mut s);
        assert_eq!(s.status(), &Status::Idle);
        assert!(s.index().is_none());
        assert!(!dir.join("idx").join("cran-index.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_damaged_cache_is_reported_and_a_refresh_replaces_it() {
        let dir = temp("damaged");
        let idx = dir.join("idx");
        assert!(std::fs::create_dir_all(&idx).is_ok());
        assert!(std::fs::write(idx.join("cran-index.txt"), "garbage").is_ok());
        let mut s = service(
            &dir,
            Canned::serving(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec())),
        );
        s.ensure_loaded();
        finish(&mut s);
        assert!(matches!(s.status(), Status::Failed(_)));
        s.refresh();
        finish(&mut s);
        assert_eq!(s.status(), &Status::Idle);
        assert_eq!(s.index().map(|i| i.len()), Some(4));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_mirror_is_checked_kept_and_can_be_reset() {
        let dir = temp("mirror");
        let mut s = service(&dir, Canned::serving(Ok(vec![]), Ok(vec![])));
        assert_eq!(s.mirror(), "https://mirror.test");
        assert!(s.set_mirror("http://insecure.test").is_err());
        assert_eq!(s.mirror(), "https://mirror.test");
        assert_eq!(s.set_mirror("https://other.test/cran/"), Ok(()));
        let again = service_without_mirror_write(&dir);
        assert_eq!(again.mirror(), "https://other.test/cran");
        s.reset_mirror();
        assert_eq!(s.mirror(), DEFAULT_MIRROR);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A service that only reads the saved mirror.
    fn service_without_mirror_write(dir: &std::path::Path) -> PackageIndexService {
        PackageIndexService::new(
            Some(dir.join("idx")),
            Some(dir.join("mirror.txt")),
            Arc::new(Canned::serving(Ok(vec![]), Ok(vec![]))),
            clock_at_1000,
        )
    }

    #[test]
    fn a_stale_index_is_offered_for_refresh_not_refreshed() {
        let dir = temp("stale");
        let mut s = service(
            &dir,
            Canned::serving(Ok(PACKAGES_GZ.to_vec()), Ok(DB_V3.to_vec())),
        );
        s.refresh();
        finish(&mut s);
        assert!(!s.refresh_suggested());
        s.clock = || 1000 + DEFAULT_MAX_AGE + 1;
        assert!(s.refresh_suggested());
        assert_eq!(s.status(), &Status::Idle);
        assert!(!s.is_busy());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
