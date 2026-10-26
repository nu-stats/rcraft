//! How much memory a process is using, for the session's memory limit
//! (P1.4). Uses the system's own tools through `std::process`, so no
//! `unsafe` code or extra crates: `tasklist` on Windows, `ps` elsewhere.

use std::process::{Command, Stdio};

/// Resident memory of process `pid` in bytes, or `None` if it cannot be read
/// (the process has gone, or the tool is missing).
pub fn memory_bytes(pid: u32) -> Option<u64> {
    #[cfg(windows)]
    {
        let mut command = Command::new("tasklist");
        command.args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"]);
        let text = run_for_text(&mut command)?;
        parse_tasklist(&text)
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new("ps");
        command.args(["-o", "rss=", "-p", &pid.to_string()]);
        let text = run_for_text(&mut command)?;
        parse_ps_rss(&text)
    }
}

fn run_for_text(command: &mut Command) -> Option<String> {
    command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command.output().ok()?;
    // the output of these tools is a line or two; anything huge is not it
    let bytes = output.stdout.get(..8192).unwrap_or(&output.stdout);
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// `ps -o rss=` prints the resident size in kilobytes.
pub fn parse_ps_rss(text: &str) -> Option<u64> {
    let kilobytes: u64 = text.trim().parse().ok()?;
    kilobytes.checked_mul(1024)
}

/// `tasklist /FO CSV /NH` prints one line such as
/// `"Rterm.exe","1234","Console","1","123,456 K"`. The last field is the
/// working set in kilobytes. The digit grouping and the unit text depend on
/// the system language, so only the digits are read.
pub fn parse_tasklist(text: &str) -> Option<u64> {
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with('"'))?
        .trim();
    let last = line.trim_end_matches('"').rsplit("\",\"").next()?;
    let digits: String = last.chars().filter(char::is_ascii_digit).collect();
    let kilobytes: u64 = digits.parse().ok()?;
    kilobytes.checked_mul(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ps_output() {
        assert_eq!(parse_ps_rss("  102400\n"), Some(102_400 * 1024));
        assert_eq!(parse_ps_rss(""), None);
        assert_eq!(parse_ps_rss("not a number"), None);
        assert_eq!(parse_ps_rss("18446744073709551615"), None);
    }

    #[test]
    fn reads_tasklist_output() {
        let line = "\"Rterm.exe\",\"1234\",\"Console\",\"1\",\"123,456 K\"\r\n";
        assert_eq!(parse_tasklist(line), Some(123_456 * 1024));
        // other locales group digits differently and spell the unit differently
        let de = "\"Rterm.exe\",\"1234\",\"Console\",\"1\",\"123.456 KB\"";
        assert_eq!(parse_tasklist(de), Some(123_456 * 1024));
    }

    #[test]
    fn tasklist_with_no_match_gives_none() {
        let none = "INFO: No tasks are running which match the specified criteria.\r\n";
        assert_eq!(parse_tasklist(none), None);
        assert_eq!(parse_tasklist(""), None);
        assert_eq!(parse_tasklist("\"only\""), None);
    }

    #[test]
    fn our_own_process_has_memory() {
        // works on the machine running the tests; None only if the tool is
        // missing, which is allowed by the contract
        if let Some(bytes) = memory_bytes(std::process::id()) {
            assert!(bytes > 0);
        }
    }
}
