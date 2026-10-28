//! The source lookups against the real services, only when `RCRAFT_LIVE_SOURCES`
//! is set (skipped with a note otherwise; CI needs no network). Nothing is
//! installed: each lookup reads one list or one DESCRIPTION.

#![allow(clippy::panic)]

use std::time::{Duration, Instant};

use rcraft_engine::sources::{Found, Lookup, LookupState, SourceLookups};

fn run(lookup: Lookup) -> Result<String, String> {
    let mut s = SourceLookups::for_machine();
    s.start(lookup);
    let started = Instant::now();
    while s.is_busy() {
        s.poll();
        assert!(started.elapsed() < Duration::from_secs(120), "no answer");
        std::thread::sleep(Duration::from_millis(20));
    }
    match s.state() {
        LookupState::Done(found) => Ok(match &**found {
            Found::Description { provider, package } => format!(
                "{}: {} {} - {} | imports: {} | needs compiling: {:?}",
                provider.name(),
                package.name,
                package.version,
                package.title,
                package.imports,
                package.needs_compilation
            ),
            Found::List {
                provider,
                release,
                index,
                ..
            } => {
                let titled = index
                    .packages()
                    .iter()
                    .filter(|p| !p.title.is_empty())
                    .count();
                let first: Vec<String> = index
                    .packages()
                    .iter()
                    .take(3)
                    .map(|p| format!("{} {} ({})", p.name, p.version, p.title))
                    .collect();
                format!(
                    "{} {}: {} packages, {} with titles; first: {}",
                    provider.name(),
                    release.clone().unwrap_or_default(),
                    index.len(),
                    titled,
                    first.join("; ")
                )
            }
        }),
        LookupState::Failed(m) => Err(m.clone()),
        _ => Err("nothing".into()),
    }
}

#[test]
fn the_real_sources_answer_when_asked() {
    if std::env::var("RCRAFT_LIVE_SOURCES").is_err() {
        eprintln!("skipped: set RCRAFT_LIVE_SOURCES=1 to ask the real services");
        return;
    }
    let bioc = run(Lookup::Bioconductor {
        r_version: "4.4.3".into(),
    });
    eprintln!("BIOCONDUCTOR: {bioc:?}");
    let universe = run(Lookup::Universe("ropensci".into()));
    eprintln!("R-UNIVERSE: {universe:?}");
    let github = run(Lookup::GitHub {
        owner: "r-lib".into(),
        repo: "cli".into(),
        reference: "HEAD".into(),
    });
    eprintln!("GITHUB: {github:?}");
    assert!(bioc.is_ok() && universe.is_ok() && github.is_ok());
}
