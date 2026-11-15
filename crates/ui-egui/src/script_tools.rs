//! The Script Editor's find and replace and its outline (P7.3), as plain
//! functions on the text so the window only draws them. Positions are in
//! characters, as the editor counts them.

/// What Find looks for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Search {
    pub needle: String,
    pub match_case: bool,
    /// Only where the text is not part of a longer name (letters, digits,
    /// `.` and `_` on neither side).
    pub whole_word: bool,
}

/// Most matches found in one script.
const MAX_MATCHES: usize = 100_000;

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '.' || c == '_'
}

impl Search {
    /// Every match, as character ranges, in order and not overlapping.
    pub fn matches(&self, text: &str) -> Vec<(usize, usize)> {
        let needle: Vec<char> = self.fold(&self.needle).chars().collect();
        if needle.is_empty() {
            return Vec::new();
        }
        let hay: Vec<char> = self.fold(text).chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i + needle.len() <= hay.len() && out.len() < MAX_MATCHES {
            let end = i + needle.len();
            let here = hay.get(i..end).is_some_and(|w| w == needle.as_slice());
            let whole = !self.whole_word
                || (i
                    .checked_sub(1)
                    .and_then(|j| hay.get(j))
                    .is_none_or(|c| !is_name_char(*c))
                    && hay.get(end).is_none_or(|c| !is_name_char(*c)));
            if here && whole {
                out.push((i, end));
                i = end;
            } else {
                i += 1;
            }
        }
        out
    }

    /// Lower case for a search that ignores case, one character for one, so
    /// positions stay the same.
    fn fold(&self, s: &str) -> String {
        if self.match_case {
            s.to_string()
        } else {
            s.chars()
                .map(|c| {
                    let mut l = c.to_lowercase();
                    match (l.next(), l.next()) {
                        (Some(one), None) => one,
                        _ => c,
                    }
                })
                .collect()
        }
    }

    /// The first match starting at or after `from` (wrapping to the top), or
    /// the last one ending at or before `from` going back (wrapping to the end).
    pub fn next(&self, text: &str, from: usize, forward: bool) -> Option<(usize, usize)> {
        let all = self.matches(text);
        if forward {
            all.iter()
                .find(|(a, _)| *a >= from)
                .or_else(|| all.first())
                .copied()
        } else {
            all.iter()
                .rev()
                .find(|(_, b)| *b <= from)
                .or_else(|| all.last())
                .copied()
        }
    }
}

/// `text` with the characters in `range` replaced by `with`; and where the
/// replacement ends.
pub fn replace_range(text: &str, range: (usize, usize), with: &str) -> (String, usize) {
    let mut out: String = text.chars().take(range.0).collect();
    out.push_str(with);
    let end = range.0 + with.chars().count();
    out.extend(text.chars().skip(range.1));
    (out, end)
}

/// `text` with every match replaced, and how many there were.
pub fn replace_all(text: &str, search: &Search, with: &str) -> (String, usize) {
    let all = search.matches(text);
    let mut out = String::new();
    let mut at = 0;
    let chars: Vec<char> = text.chars().collect();
    for (a, b) in &all {
        out.extend(chars.get(at..*a).unwrap_or_default());
        out.push_str(with);
        at = *b;
    }
    out.extend(chars.get(at..).unwrap_or_default());
    (out, all.len())
}

/// One entry of the outline: the line (from 0), its text, and its depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub line: usize,
    pub title: String,
    /// 1 to 3 for a section (`# Title ----`, more `#` for deeper), 4 for a
    /// function defined at the start of a line.
    pub depth: usize,
}

/// The script's sections: comment lines that end in four or more `-`, `=` or
/// `#` (`# Data ----`, `## Models ====`, `* Setup ----` in a do-file), and
/// functions defined at the start of a line (`fit <- function(`).
pub fn outline(text: &str) -> Vec<Section> {
    let mut out = Vec::new();
    for (line, raw) in text.lines().enumerate() {
        let t = raw.trim();
        let comment = t.starts_with('#') || t.starts_with('*') || t.starts_with("//");
        if comment {
            let marks = t.trim_start_matches(['#', '*', '/']);
            let level = t.len() - marks.len();
            let body = marks.trim_end();
            let tail = body.len() - body.trim_end_matches(['-', '=', '#']).len();
            let title = body.trim_end_matches(['-', '=', '#']).trim();
            if tail >= 4 && !title.is_empty() {
                out.push(Section {
                    line,
                    title: title.to_string(),
                    depth: level.clamp(1, 3),
                });
            }
            continue;
        }
        // a function defined at the start of a line
        if raw.starts_with(|c: char| c.is_alphabetic() || c == '.' || c == '`') {
            if let Some((name, rest)) = raw.split_once("<-").or_else(|| raw.split_once('=')) {
                let name = name.trim().trim_matches('`');
                if rest.trim_start().starts_with("function")
                    && !name.is_empty()
                    && name.chars().all(|c| is_name_char(c) || c == ' ')
                {
                    out.push(Section {
                        line,
                        title: format!("{name}()"),
                        depth: 4,
                    });
                }
            }
        }
    }
    out
}

/// Where line `line` (from 0) starts, in characters.
pub fn line_start(text: &str, line: usize) -> usize {
    text.split_inclusive('\n')
        .take(line)
        .map(|l| l.chars().count())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(needle: &str, match_case: bool, whole_word: bool) -> Search {
        Search {
            needle: needle.into(),
            match_case,
            whole_word,
        }
    }

    #[test]
    fn find_respects_case_and_whole_words() {
        let text = "fit <- lm(y ~ x)\nFit2 <- lm(y ~ x + z)\nsummary(fit)";
        assert_eq!(search("fit", false, false).matches(text).len(), 3);
        assert_eq!(search("fit", true, false).matches(text).len(), 2);
        // whole word: not inside Fit2
        assert_eq!(
            search("fit", false, true).matches(text),
            vec![(0, 3), (47, 50)]
        );
        assert!(search("", false, false).matches(text).is_empty());
        // characters, not bytes
        let accents = "é <- 1; é + é";
        assert_eq!(
            search("é", true, false).matches(accents),
            vec![(0, 1), (8, 9), (12, 13)]
        );
    }

    #[test]
    fn next_and_previous_wrap_around() {
        let s = search("x", true, true);
        let text = "x <- 1\ny <- x\nx";
        assert_eq!(s.next(text, 0, true), Some((0, 1)));
        assert_eq!(s.next(text, 1, true), Some((12, 13)));
        assert_eq!(s.next(text, 14, true), Some((14, 15)));
        assert_eq!(s.next(text, 15, true), Some((0, 1)));
        assert_eq!(s.next(text, 12, false), Some((0, 1)));
        assert_eq!(s.next(text, 13, false), Some((12, 13)));
        assert_eq!(s.next(text, 0, false), Some((14, 15)));
        assert_eq!(search("nothing", true, false).next(text, 0, true), None);
    }

    #[test]
    fn replace_one_and_all() {
        let text = "a <- data$a; b <- data$b";
        let (one, end) = replace_range(text, (5, 9), "df");
        assert_eq!(one, "a <- df$a; b <- data$b");
        assert_eq!(end, 7);
        let (all, n) = replace_all(text, &search("data", true, true), "df");
        assert_eq!((all.as_str(), n), ("a <- df$a; b <- df$b", 2));
        // a replacement that contains the text looked for does not loop
        let (grown, n) = replace_all("x x", &search("x", true, true), "xx");
        assert_eq!((grown.as_str(), n), ("xx xx", 2));
    }

    #[test]
    fn the_outline_lists_sections_and_functions() {
        let text = "# Data ----\nd <- read.csv('a.csv')\n## Clean ====\n# just a comment\n\
                    fit_all <- function(d) {\n  inner <- function() 1\n}\n* Stata part ----\n#----\n";
        let o = outline(text);
        let shown: Vec<(usize, &str, usize)> = o
            .iter()
            .map(|s| (s.line, s.title.as_str(), s.depth))
            .collect();
        assert_eq!(
            shown,
            vec![
                (0, "Data", 1),
                (2, "Clean", 2),
                (4, "fit_all()", 4),
                (7, "Stata part", 1),
            ]
        );
        assert_eq!(
            line_start(text, 2),
            "# Data ----\nd <- read.csv('a.csv')\n".chars().count()
        );
        assert_eq!(line_start("", 3), 0);
    }
}
