//! P1A.2's "Binary for this R" against a real R (skipped with a note when none
//! is found): the folder R names for its binaries makes a list address on
//! Windows and macOS, and R says there is none on Linux. Nothing is fetched.

use std::time::{Duration, Instant};

use rcraft_engine::sources::{binary_list_address, BINARY_FOLDER_CODE};
use rcraft_engine::Engine;
use rcraft_rhost::{find_installs, Env, SessionConfig};

#[test]
fn r_names_the_folder_of_its_binaries() {
    let install = find_installs(&Env::from_process())
        .into_iter()
        .find(|i| i.is_supported());
    let Some(install) = install else {
        eprintln!("skipped: no supported R on this machine");
        return;
    };
    let Ok(mut e) = Engine::start(SessionConfig::new(install)) else {
        eprintln!("skipped: R did not start");
        return;
    };
    let id = e.query(BINARY_FOLDER_CODE);
    let limit = Instant::now() + Duration::from_secs(120);
    while e.query_result(id).is_none() {
        e.poll(Duration::from_millis(50));
        assert!(Instant::now() < limit, "no answer");
    }
    let Some(r) = e.query_result(id) else {
        return;
    };
    let address = binary_list_address("https://cloud.r-project.org/", &r.text).filter(|_| r.ok);
    eprintln!("R said {:?} (ok {}): {address:?}", r.text, r.ok);
    if cfg!(target_os = "linux") {
        assert_eq!(address, None, "{}", r.text);
    } else {
        let minor: String = e
            .r_version()
            .split_whitespace()
            .nth(2)
            .map(|v| v.rsplit_once('.').map_or(v, |(a, _)| a).to_string())
            .unwrap_or_default();
        let a = address.unwrap_or_default();
        assert!(a.starts_with("https://cloud.r-project.org/bin/"), "{a}");
        assert!(a.ends_with(&format!("/contrib/{minor}/PACKAGES.gz")), "{a}");
    }
}
