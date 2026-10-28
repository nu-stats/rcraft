//! One real HTTPS request, only when `RCRAFT_LIVE_URL` names a URL (skipped
//! with a note otherwise; CI needs no network). It tests what the local-server
//! tests cannot: the TLS handshake and certificate check against a real mirror.

use rcraft_rpkg::http::UreqFetcher;
use rcraft_rpkg::refresh::{Control, Fetcher};

#[test]
fn one_live_https_request_when_a_url_is_given() {
    let Ok(url) = std::env::var("RCRAFT_LIVE_URL") else {
        eprintln!("skipped: set RCRAFT_LIVE_URL to an https URL to try one request");
        return;
    };
    let mut calls = 0;
    let got = UreqFetcher::new().get(&url, 1 << 20, &mut |_, _| {
        calls += 1;
        Control::Continue
    });
    match &got {
        Ok(b) => eprintln!(
            "{} bytes in {calls} progress calls; starts {:?}",
            b.len(),
            String::from_utf8_lossy(b.get(..60).unwrap_or(b))
        ),
        Err(e) => eprintln!("error: {e}"),
    }
    assert!(got.is_ok_and(|b| !b.is_empty()));
}
