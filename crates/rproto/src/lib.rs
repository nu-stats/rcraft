//! Session protocol to R: request, result, stream, interrupt.
//!
//! This crate is the wire format only: no processes. `rhost` runs R and uses
//! these types.
//!
//! **Requests** go to R's standard input as text headers, then code bytes:
//!
//! ```text
//! RUN <id> <nbytes>\n<code bytes>\n
//! QUIT\n
//! ```
//!
//! **Frames** come back on R's standard output as lines that start with a
//! per-session token and a tab, mixed with whatever the code prints. Fields
//! are separated by tabs; free text is escaped (see [`escape`]). The token is
//! random per session, so printed output cannot be mistaken for a frame.
//!
//! ```text
//! <token>\tREADY\t<R version string>\t<R's process id>
//! <token>\tBEGIN\t<id>
//! <token>\tWARN\t<id>\t<message>
//! <token>\tERROR\t<id>\t<message>\t<call>
//! <token>\tSYNC\t<id>        (on standard error, just before END)
//! <token>\tEND\t<id>\t<ok|error|interrupted>
//! ```
//!
//! Everything else on standard output is the run's printed output; standard
//! error carries messages and is kept separate by the pipe itself.

use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::{BuildHasher, Hasher};

/// The R program that implements the other end. Run as
/// `Rscript --vanilla <file> <token>`.
pub const BOOTSTRAP_R: &str = include_str!("bootstrap.R");

/// Largest request (code) RCraft will send in one run.
pub const MAX_REQUEST_BYTES: usize = 64 * 1024 * 1024;

/// What to write to R's standard input to end the session.
pub const QUIT: &[u8] = b"QUIT\n";

/// Identifies one run within a session.
pub type RunId = u64;

/// How a run ended, as reported by R.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Error,
    Interrupted,
}

/// One message from R's side of the protocol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Frame {
    Ready {
        r_version: String,
        /// R's own process id (on Windows `Rscript` starts R as a child, so
        /// this differs from the id of the process RCraft started).
        pid: u32,
    },
    Begin {
        run: RunId,
    },
    Warning {
        run: RunId,
        message: String,
    },
    Error {
        run: RunId,
        message: String,
        call: String,
    },
    End {
        run: RunId,
        status: Status,
    },
    /// Sent on standard error just before `End`, so the reader of that stream
    /// can tell that everything R wrote there for the run has been read.
    Sync {
        run: RunId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtoError {
    TooLarge(usize),
    BadFrame(String),
}

impl fmt::Display for ProtoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtoError::TooLarge(n) => write!(
                f,
                "that is {n} bytes of code; the limit for one run is {MAX_REQUEST_BYTES}"
            ),
            ProtoError::BadFrame(why) => write!(f, "unreadable message from R: {why}"),
        }
    }
}

/// A random token for one session. Built from the standard library's
/// per-process random hash keys, so no extra crate is needed.
pub fn new_token() -> String {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default(),
    );
    format!("rcraft{:016x}", hasher.finish())
}

/// The bytes that ask R to run `code`.
pub fn encode_run(id: RunId, code: &str) -> Result<Vec<u8>, ProtoError> {
    if code.len() > MAX_REQUEST_BYTES {
        return Err(ProtoError::TooLarge(code.len()));
    }
    let mut out = format!("RUN {id} {}\n", code.len()).into_bytes();
    out.extend_from_slice(code.as_bytes());
    out.push(b'\n');
    Ok(out)
}

/// Escapes free text for a frame field: backslash, newline, carriage return
/// and tab. The R side does the same.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// Undoes [`escape`]. An unknown escape is kept as written.
pub fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Reads one line of R's standard output. `None` means the line is not a
/// frame (it is the run's printed output); `Some(Err(..))` means it carried
/// the token but could not be read.
pub fn parse_frame(token: &str, line: &str) -> Option<Result<Frame, ProtoError>> {
    let line = line.trim_end_matches(['\n', '\r']);
    let rest = line.strip_prefix(token)?.strip_prefix('\t')?;
    Some(parse_fields(rest))
}

/// Like [`parse_frame`], for a frame that follows text with no line break
/// (`cat("hi")` leaves the cursor mid-line, so R's next frame is glued to it).
/// Returns the text before the frame, which is never empty, and the frame.
pub fn parse_frame_after_text<'a>(
    token: &str,
    line: &'a str,
) -> Option<(&'a str, Result<Frame, ProtoError>)> {
    let marker = format!("{token}\t");
    let at = line.find(&marker).filter(|&at| at > 0)?;
    let (text, frame) = line.split_at(at);
    Some((text, parse_frame(token, frame)?))
}

fn parse_fields(rest: &str) -> Result<Frame, ProtoError> {
    let bad = |why: &str| ProtoError::BadFrame(format!("{why}: {rest}"));
    let mut fields = rest.split('\t');
    let kind = fields.next().unwrap_or_default();
    let mut next = |name: &str| fields.next().ok_or_else(|| bad(&format!("no {name}")));
    let run = |text: &str| text.parse::<RunId>().map_err(|_| bad("bad run id"));
    match kind {
        "READY" => Ok(Frame::Ready {
            r_version: unescape(next("version")?),
            pid: next("pid")?.parse().map_err(|_| bad("bad process id"))?,
        }),
        "BEGIN" => Ok(Frame::Begin {
            run: run(next("run id")?)?,
        }),
        "WARN" => Ok(Frame::Warning {
            run: run(next("run id")?)?,
            message: unescape(next("message")?),
        }),
        "ERROR" => Ok(Frame::Error {
            run: run(next("run id")?)?,
            message: unescape(next("message")?),
            call: unescape(next("call")?),
        }),
        "SYNC" => Ok(Frame::Sync {
            run: run(next("run id")?)?,
        }),
        "END" => {
            let id = run(next("run id")?)?;
            let status = match next("status")? {
                "ok" => Status::Ok,
                "error" => Status::Error,
                "interrupted" => Status::Interrupted,
                other => return Err(bad(&format!("unknown status {other}"))),
            };
            Ok(Frame::End { run: id, status })
        }
        other => Err(bad(&format!("unknown frame {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: &str = "rcrafttoken";

    #[test]
    fn escape_round_trips() {
        for text in ["", "plain", "a\nb\tc\r\\d", "\\n is not a newline", "tab\t"] {
            assert_eq!(unescape(&escape(text)), text);
        }
        assert!(!escape("a\tb\nc").contains(['\t', '\n']));
    }

    #[test]
    fn a_frame_glued_to_unterminated_text_is_found() {
        let line = format!("hi{T}\tEND\t3\tok\n");
        let found = parse_frame_after_text(T, &line);
        assert_eq!(
            found,
            Some((
                "hi",
                Ok(Frame::End {
                    run: 3,
                    status: Status::Ok
                })
            ))
        );
        // a frame at the start of the line is parse_frame's job, and plain
        // text, or text that merely resembles the token, is not a frame
        assert!(parse_frame_after_text(T, &format!("{T}\tEND\t3\tok\n")).is_none());
        assert!(parse_frame_after_text(T, "no frame here\n").is_none());
        assert!(parse_frame_after_text(T, &format!("x {T} y\n")).is_none());
    }

    #[test]
    fn unescape_is_lenient() {
        assert_eq!(unescape("a\\qb"), "a\\qb");
        assert_eq!(unescape("trailing\\"), "trailing\\");
    }

    #[test]
    fn encodes_a_run() {
        let bytes = encode_run(7, "1 + 1").unwrap_or_default();
        assert_eq!(bytes, b"RUN 7 5\n1 + 1\n");
        // the length counts bytes, not characters
        let bytes = encode_run(1, "é").unwrap_or_default();
        assert!(bytes.starts_with(b"RUN 1 2\n"));
    }

    #[test]
    fn refuses_huge_code() {
        let big = "x".repeat(MAX_REQUEST_BYTES + 1);
        assert!(matches!(encode_run(1, &big), Err(ProtoError::TooLarge(_))));
    }

    #[test]
    fn parses_frames() {
        let line = |s: &str| format!("{T}\t{s}\n");
        assert_eq!(
            parse_frame(T, &line("READY\tR version 4.4.3\t4242")),
            Some(Ok(Frame::Ready {
                r_version: "R version 4.4.3".into(),
                pid: 4242
            }))
        );
        assert_eq!(
            parse_frame(T, &line("BEGIN\t3")),
            Some(Ok(Frame::Begin { run: 3 }))
        );
        assert_eq!(
            parse_frame(T, &line("WARN\t3\ta\\nb")),
            Some(Ok(Frame::Warning {
                run: 3,
                message: "a\nb".into()
            }))
        );
        assert_eq!(
            parse_frame(T, &line("ERROR\t3\tboom\tf(x)")),
            Some(Ok(Frame::Error {
                run: 3,
                message: "boom".into(),
                call: "f(x)".into()
            }))
        );
        assert_eq!(
            parse_frame(T, &line("SYNC\t3")),
            Some(Ok(Frame::Sync { run: 3 }))
        );
        assert_eq!(
            parse_frame(T, &line("END\t3\tinterrupted")),
            Some(Ok(Frame::End {
                run: 3,
                status: Status::Interrupted
            }))
        );
    }

    #[test]
    fn ordinary_output_is_not_a_frame() {
        assert_eq!(parse_frame(T, "[1] 2\n"), None);
        assert_eq!(parse_frame(T, ""), None);
        // the token without its tab is just text
        assert_eq!(parse_frame(T, "rcrafttoken printed by a user\n"), None);
        // a different token is just text
        assert_eq!(parse_frame(T, "other\tEND\t1\tok\n"), None);
    }

    #[test]
    fn bad_frames_are_errors_not_crashes() {
        for body in [
            "",
            "NOPE",
            "READY\tR version 4.4.3",
            "READY\tR version 4.4.3\tx",
            "BEGIN",
            "BEGIN\tx",
            "END\t1",
            "END\t1\tmaybe",
            "ERROR\t1\tonly",
        ] {
            let line = format!("{T}\t{body}");
            assert!(matches!(parse_frame(T, &line), Some(Err(_))), "{body}");
        }
    }

    #[test]
    fn tokens_differ_between_sessions() {
        assert_ne!(new_token(), new_token());
        assert!(new_token().starts_with("rcraft"));
    }
}
