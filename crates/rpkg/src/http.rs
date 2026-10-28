//! The real [`Fetcher`]: HTTPS downloads with `ureq` and rustls.
//!
//! One GET per call, into memory, with a hard cap on the size, a progress
//! callback that can cancel, no cookies and nothing sent but the request line
//! and a User-Agent. Proxy settings in the environment are honoured (ureq's
//! default), which matters on campus networks. HTTP is refused unless the
//! fetcher was made for tests.

use std::io::Read;
use std::time::Duration;

use ureq::Agent;

use crate::refresh::{Control, Fetcher};
use crate::{Error, Result};

/// Bytes read per step between progress reports.
const CHUNK: usize = 64 * 1024;

/// Downloads with `ureq`.
pub struct UreqFetcher {
    agent: Agent,
}

impl UreqFetcher {
    /// A fetcher that accepts HTTPS only (redirects included).
    pub fn new() -> UreqFetcher {
        UreqFetcher::build(true)
    }

    /// A fetcher that also accepts plain HTTP, for tests against a local
    /// server. Never used by the app.
    pub fn plain_http_for_tests() -> UreqFetcher {
        UreqFetcher::build(false)
    }

    fn build(https_only: bool) -> UreqFetcher {
        let config = Agent::config_builder()
            .https_only(https_only)
            .max_redirects(5)
            .timeout_connect(Some(Duration::from_secs(20)))
            .timeout_global(Some(Duration::from_secs(15 * 60)))
            .user_agent(format!(
                "RCraft/{} (package index; +https://github.com/nu-stats/rcraft)",
                rcraft_core::version()
            ))
            .build();
        UreqFetcher {
            agent: config.into(),
        }
    }
}

impl Default for UreqFetcher {
    fn default() -> UreqFetcher {
        UreqFetcher::new()
    }
}

impl Fetcher for UreqFetcher {
    fn get(
        &self,
        url: &str,
        max_bytes: u64,
        progress: &mut dyn FnMut(u64, Option<u64>) -> Control,
    ) -> Result<Vec<u8>> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| Error::Fetch(describe(url, &e)))?;
        let total = response.body().content_length();
        if total.is_some_and(|t| t > max_bytes) {
            return Err(Error::TooBig(format!(
                "{url} is larger than {max_bytes} bytes"
            )));
        }
        if progress(0, total) == Control::Cancel {
            return Err(Error::Cancelled);
        }
        let mut reader = response
            .body_mut()
            .with_config()
            .limit(max_bytes.saturating_add(1))
            .reader();
        let mut out: Vec<u8> =
            Vec::with_capacity(usize::try_from(total.unwrap_or(0).min(max_bytes)).unwrap_or(0));
        let mut chunk = vec![0u8; CHUNK];
        loop {
            let n = reader
                .read(&mut chunk)
                .map_err(|e| Error::Fetch(format!("{url}: the download broke off ({e})")))?;
            if n == 0 {
                break;
            }
            out.extend_from_slice(chunk.get(..n).unwrap_or(&[]));
            if out.len() as u64 > max_bytes {
                return Err(Error::TooBig(format!(
                    "{url} is larger than {max_bytes} bytes"
                )));
            }
            if progress(out.len() as u64, total) == Control::Cancel {
                return Err(Error::Cancelled);
            }
        }
        if let Some(t) = total {
            if out.len() as u64 != t {
                return Err(Error::Fetch(format!(
                    "{url}: the download stopped at {} of {t} bytes",
                    out.len()
                )));
            }
        }
        Ok(out)
    }
}

/// A short, plain description of a failed request.
fn describe(url: &str, e: &ureq::Error) -> String {
    match e {
        ureq::Error::StatusCode(code) => format!("{url}: the server answered {code}"),
        other => format!("{url}: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    /// Serves `responses` one connection each, then stops. Returns the base URL.
    fn serve(responses: Vec<Vec<u8>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0");
        let listener = listener.unwrap_or_else(|e| panic!("{e}"));
        let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
        std::thread::spawn(move || {
            for body in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                // read the request head
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(&body);
                let _ = stream.flush();
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    fn ok(body: &[u8]) -> Vec<u8> {
        let mut r = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        r.extend_from_slice(body);
        r
    }

    fn never(_: u64, _: Option<u64>) -> Control {
        Control::Continue
    }

    #[test]
    fn a_body_arrives_whole_with_progress_ending_at_the_size() {
        let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let base = serve(vec![ok(&body)]);
        let f = UreqFetcher::plain_http_for_tests();
        let mut seen = Vec::new();
        let got = f.get(&format!("{base}/x"), 1 << 20, &mut |d, t| {
            seen.push((d, t));
            Control::Continue
        });
        assert_eq!(got, Ok(body.clone()));
        assert_eq!(seen.first(), Some(&(0, Some(200_000))));
        assert_eq!(seen.last(), Some(&(200_000, Some(200_000))));
        assert!(seen.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn an_error_status_is_a_fetch_error_naming_the_status() {
        let base = serve(vec![
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        ]);
        let f = UreqFetcher::plain_http_for_tests();
        let got = f.get(&format!("{base}/missing"), 1000, &mut never);
        assert!(
            matches!(got, Err(Error::Fetch(ref m)) if m.contains("404")),
            "{got:?}"
        );
    }

    #[test]
    fn a_body_over_the_cap_is_refused_before_and_while_reading() {
        let base = serve(vec![ok(&[7u8; 5000]), {
            // no Content-Length: the cap has to be enforced while reading
            let mut r = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
            r.extend_from_slice(&[7u8; 5000]);
            r
        }]);
        let f = UreqFetcher::plain_http_for_tests();
        let a = f.get(&format!("{base}/a"), 1000, &mut never);
        assert!(matches!(a, Err(Error::TooBig(_))), "{a:?}");
        let b = f.get(&format!("{base}/b"), 1000, &mut never);
        assert!(matches!(b, Err(Error::TooBig(_))), "{b:?}");
    }

    #[test]
    fn cancelling_stops_the_download() {
        let base = serve(vec![ok(&[1u8; 300_000])]);
        let f = UreqFetcher::plain_http_for_tests();
        let got = f.get(&format!("{base}/x"), 1 << 20, &mut |done, _| {
            if done > 0 {
                Control::Cancel
            } else {
                Control::Continue
            }
        });
        assert_eq!(got, Err(Error::Cancelled));
    }

    #[test]
    fn a_connection_cut_short_is_an_error_not_a_short_file() {
        // promises 1000 bytes, sends 10, then closes
        let base = serve(vec![
            b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n0123456789"
                .to_vec(),
        ]);
        let f = UreqFetcher::plain_http_for_tests();
        let got = f.get(&format!("{base}/x"), 1 << 20, &mut never);
        assert!(matches!(got, Err(Error::Fetch(_))), "{got:?}");
    }

    #[test]
    fn plain_http_is_refused_by_the_app_fetcher_and_a_dead_port_is_an_error() {
        let base = serve(vec![ok(b"hi")]);
        let got = UreqFetcher::new().get(&format!("{base}/x"), 100, &mut never);
        assert!(matches!(got, Err(Error::Fetch(_))), "{got:?}");
        // nothing listens on port 1
        let got = UreqFetcher::plain_http_for_tests().get("http://127.0.0.1:1/x", 100, &mut never);
        assert!(matches!(got, Err(Error::Fetch(_))), "{got:?}");
    }
}
