//! Reading do-files into command lines.
//!
//! Behaviour reference: `plan/stata/do-files.md`. A logical command line
//! keeps the physical lines it came from, because the log echoes those.
//! Comments: `*` starting a command, `//` after a space or at the start,
//! `/* ... */` anywhere (may span lines). `///` after a space joins the next
//! line.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// the physical lines as written, for the log
    pub physical: Vec<String>,
    /// the command with comments removed and continuations joined
    pub text: String,
}

impl Line {
    /// A comment or empty line: echoed but not run.
    pub fn is_blank(&self) -> bool {
        let t = self.text.trim();
        t.is_empty() || t.starts_with('*')
    }
}

/// Splits a do-file into command lines.
pub fn read(source: &str) -> Vec<Line> {
    let mut out = Vec::new();
    let mut physical: Vec<String> = Vec::new();
    let mut text = String::new();
    let mut in_block = false;
    let mut semicolon = false;
    let raw: Vec<&str> = source.split('\n').collect();
    // the text after the final newline is a line too (Stata echoes it)
    for line in raw {
        let line = line.strip_suffix('\r').unwrap_or(line);
        // Stata reads `\\` as `\`, except just before a `"` (probe 139)
        let reduced = reduce_backslashes(line, true);
        let line = reduced.as_str();
        // `#delimit ;` and `#delimit cr`: commands end at `;` or at the
        // end of the line
        let trimmed = line.trim_start();
        if !in_block
            && (trimmed.starts_with("#delimit") || trimmed.starts_with("#d ") || trimmed == "#d")
        {
            let arg = trimmed
                .split_whitespace()
                .nth(1)
                .unwrap_or("cr")
                .trim_end_matches(';');
            semicolon =
                arg.is_empty() || arg == ";" || trimmed.trim_end().ends_with(';') && arg != "cr";
            physical.push(line.to_string());
            out.push(Line {
                physical: std::mem::take(&mut physical),
                text: String::new(),
            });
            continue;
        }
        physical.push(line.to_string());
        let (clean, continues, still_in_block) = strip_comments(line, in_block, text.is_empty());
        in_block = still_in_block;
        text.push_str(&clean);
        if semicolon {
            text.push(' ');
            // every `;` outside quotes ends a command
            while let Some(at) = semicolon_at(&text) {
                let command = text.get(..at).unwrap_or("").trim().to_string();
                let rest = text.get(at + 1..).unwrap_or("").to_string();
                out.push(Line {
                    physical: std::mem::take(&mut physical),
                    text: command,
                });
                text = rest;
            }
            if text.trim().is_empty() {
                text.clear();
            }
            continue;
        }
        if continues || in_block {
            text.push(' ');
            continue;
        }
        let mut command = std::mem::take(&mut text).trim_end().to_string();
        // `else{` is `else {` (outreg2 writes it so)
        if let Some(at) = command.find("else{") {
            if command.get(..at).is_some_and(|b| b.trim().is_empty()) {
                command.insert(at + 4, ' ');
            }
        }
        out.push(Line {
            physical: std::mem::take(&mut physical),
            text: command,
        });
    }
    if !physical.is_empty() {
        out.push(Line {
            physical,
            text: text.trim_end().to_string(),
        });
    }
    out
}

/// Where the first `;` outside quotes is.
fn semicolon_at(text: &str) -> Option<usize> {
    let mut quote = false;
    for (i, c) in text.char_indices() {
        match c {
            '"' => quote = !quote,
            ';' if !quote => return Some(i),
            _ => {}
        }
    }
    None
}

/// Each run of backslashes loses half its length (`\\` is `\`, `\\\` is
/// `\\`). With `protect_quote`, a run just before a `"` stays as it is,
/// as when Stata reads a line; before version 9 Stata also reduces after
/// macro expansion, quotes or not (probe 139).
pub fn reduce_backslashes(line: &str, protect_quote: bool) -> String {
    if !line.contains("\\\\") {
        return line.to_string();
    }
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c != '\\' {
            out.push(c);
            i += 1;
            continue;
        }
        let run = chars
            .get(i..)
            .unwrap_or_default()
            .iter()
            .take_while(|&&d| d == '\\')
            .count();
        let next = chars.get(i + run).copied();
        let keep = if protect_quote && next == Some('"') {
            run
        } else {
            run / 2 + run % 2
        };
        out.extend(std::iter::repeat_n('\\', keep));
        i += run;
    }
    out
}

/// Removes comments from one physical line. Returns the text, whether a
/// `///` continues it, and whether a `/* */` comment is still open.
fn strip_comments(line: &str, mut in_block: bool, starts_command: bool) -> (String, bool, bool) {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut in_quote = false;
    let at = |i: usize| chars.get(i).copied();
    if !in_block && starts_command && line.trim_start().starts_with('*') {
        return (line.to_string(), false, false);
    }
    while let Some(c) = at(i) {
        if in_block {
            if c == '*' && at(i + 1) == Some('/') {
                in_block = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if c == '"' {
            in_quote = !in_quote;
        }
        if !in_quote && c == '/' && at(i + 1) == Some('*') {
            in_block = true;
            i += 2;
            continue;
        }
        let after_space = i == 0 || at(i - 1).is_some_and(char::is_whitespace);
        if !in_quote && c == '/' && at(i + 1) == Some('/') && after_space {
            if at(i + 2) == Some('/') {
                return (out, true, false);
            }
            return (out, false, false);
        }
        out.push(c);
        i += 1;
    }
    (out, false, in_block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_continuations() {
        let lines = read(
            "* note\ndisplay 1 // tail\ndisplay /* x */ 2\nlocal a 1 ///\n  2\ndisplay \"a//b\"\n",
        );
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "* note",
                "display 1",
                "display  2",
                "local a 1    2",
                "display \"a//b\"",
                ""
            ]
        );
        assert_eq!(lines[3].physical, ["local a 1 ///", "  2"]);
        assert!(lines[0].is_blank());
        assert!(lines[5].is_blank());
        let block = read("display 1 /* start\nstill comment */ + 1\n");
        assert_eq!(block[0].text, "display 1   + 1");
    }
}
