//! Tab completion for the Command window (P2.2).
//!
//! R already knows how to complete a line: function and object names, the
//! columns after `$`, `package::name`, package names inside `library(`, and
//! file names inside quotes. This module only asks R to do it (through
//! `utils`, which is part of R) and reads the answer, so what Tab offers is
//! what R's own console would offer and it always matches the session in use.
//!
//! Two things R's console does not do are done here: the column names inside
//! `df[["`, `df["`, `df[, "` and `df[, c("a", "` ([`column_spot`]), and a
//! finished argument name reads `name = `. Tab works on the text before the
//! cursor; what follows the cursor stays as it is ([`finish`]).

use crate::import::r_string;
use crate::rc::Completion;

/// Most choices kept from one answer.
pub const MAX_OPTIONS: usize = 60;

/// R code that prints the token at the end of `line` and the ways it could
/// continue, one per line: `TOKEN<TAB>text`, then `ITEM<TAB>text`. Printing
/// nothing (or an error) means there is nothing to offer.
pub fn completion_code(line: &str) -> String {
    format!(
        "local({{\n\
         utils::rc.settings(ipck = TRUE, func = TRUE)\n\
         line <- {line}\n\
         utils:::.assignLinebuffer(line)\n\
         utils:::.assignEnd(nchar(line))\n\
         utils:::.guessTokenFromLine()\n\
         utils:::.completeToken()\n\
         found <- unique(as.character(utils:::.retrieveCompletions()))\n\
         arg <- grepl(\"^[[:alnum:]._]+=$\", found)\n\
         found <- c(sort(found[arg]), sort(found[!arg]))\n\
         cat(\"TOKEN\\t\", utils:::.CompletionEnv[[\"token\"]], \"\\n\", sep = \"\")\n\
         for (x in head(found, {max})) cat(\"ITEM\\t\", x, \"\\n\", sep = \"\")\n\
         }})",
        line = r_string(line),
        max = MAX_OPTIONS + 1,
    )
}

/// What R answered, as a [`Completion`] for `line`: where the word being
/// completed starts and what it could become. `None` when R offered nothing
/// or the answer does not fit the line (the token must be the line's end).
pub fn parse_answer(line: &str, answer: &str) -> Option<Completion> {
    let mut token: Option<&str> = None;
    let mut options: Vec<String> = Vec::new();
    for l in answer.lines() {
        if let Some(t) = l.strip_prefix("TOKEN\t") {
            token = Some(t);
        } else if let Some(x) = l.strip_prefix("ITEM\t") {
            if !x.is_empty() {
                options.push(x.to_string());
            }
        }
    }
    let token = token?;
    if options.is_empty() || !line.ends_with(token) {
        return None;
    }
    // argument names first (R lists them first), then the rest
    options.sort_by(|a, b| (!is_argument(a), a).cmp(&(!is_argument(b), b)));
    options.dedup();
    options.truncate(MAX_OPTIONS);
    Some(Completion {
        from: line.len() - token.len(),
        options,
    })
}

/// Whether R offered `word` as an argument name (`na.rm=`).
fn is_argument(word: &str) -> bool {
    word.strip_suffix('=').is_some_and(|name| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
    })
}

/// The line after Tab: the word completed as far as the choices agree. The
/// second value is the choices to show when there are several. No space is
/// added after a single choice, since the next character depends on the call
/// being typed, except that an argument name becomes `name = `.
pub fn apply(line: &str, c: &Completion) -> (String, Vec<String>) {
    let typed = line.get(c.from..).unwrap_or("");
    let mut out = line.get(..c.from).unwrap_or("").to_string();
    if let [only] = c.options.as_slice() {
        match only.strip_suffix('=') {
            Some(name) if is_argument(only) => {
                out.push_str(name);
                out.push_str(" = ");
            }
            _ => out.push_str(only),
        }
        return (out, Vec::new());
    }
    let common = crate::rc::common_prefix(&c.options);
    out.push_str(if common.len() > typed.len() {
        &common
    } else {
        typed
    });
    (out, c.options.clone())
}

/// The whole line after Tab: `before` is the text before the cursor (what was
/// completed), `after` the rest, kept as it is. A single choice in a column
/// spot gets its closing quote, and `]]` too when nothing follows the cursor,
/// unless what follows already starts with the quote. Gives the line, where
/// the cursor goes (in characters) and the choices to show.
pub fn finish(
    before: &str,
    after: &str,
    c: &Completion,
    close: &str,
) -> (String, usize, Vec<String>) {
    let (mut done, shown) = apply(before, c);
    if shown.is_empty() {
        if let Some(quote) = close.chars().next() {
            if !after.starts_with(quote) {
                done.push_str(if after.is_empty() {
                    close
                } else {
                    close.get(..quote.len_utf8()).unwrap_or("")
                });
            }
        }
    }
    let cursor = done.chars().count();
    done.push_str(after);
    (done, cursor, shown)
}

/// A spot where a column name is being typed inside quotes: `df[["`,
/// `df["`, `df[, "`, `df[, c("a", "`, with `df` a name and any `$name` parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnSpot {
    /// The object: a name, then the `$` parts after it.
    pub object: Vec<String>,
    /// `colnames()` (after a comma inside `[`) rather than `names()`.
    pub columns: bool,
    /// The part of the name typed so far (after the quote).
    pub typed: String,
    /// What closes a finished name: the quote, then `]]` inside `[[`.
    pub close: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Open {
    Paren,
    Brace,
    Bracket,
    Double,
}

/// The column spot `before` (the text before the cursor) ends in, if any.
pub fn column_spot(before: &str) -> Option<ColumnSpot> {
    let chars: Vec<(usize, char)> = before.char_indices().collect();
    // each opener: kind, where it starts, whether a comma came straight inside it
    let mut stack: Vec<(Open, usize, bool)> = Vec::new();
    let mut string: Option<(char, usize)> = None;
    let mut i = 0;
    while let Some(&(at, c)) = chars.get(i) {
        let next = chars.get(i + 1).map(|&(_, c)| c);
        i += 1;
        if let Some((quote, _)) = string {
            if c == '\\' {
                i += 1;
            } else if c == quote {
                string = None;
            }
            continue;
        }
        match c {
            '"' | '\'' | '`' => string = Some((c, at)),
            '#' => return None,
            '(' => stack.push((Open::Paren, at, false)),
            '{' => stack.push((Open::Brace, at, false)),
            '[' if next == Some('[') => {
                stack.push((Open::Double, at, false));
                i += 1;
            }
            '[' => stack.push((Open::Bracket, at, false)),
            ']' => {
                if let Some(&(Open::Double, _, _)) = stack.last() {
                    if next == Some(']') {
                        i += 1;
                    }
                }
                stack.pop();
            }
            ')' | '}' => {
                stack.pop();
            }
            ',' => {
                if let Some(top) = stack.last_mut() {
                    top.2 = true;
                }
            }
            _ => {}
        }
    }
    let (quote, start) = string?;
    if quote == '`' {
        return None;
    }
    let typed = before.get(start + quote.len_utf8()..)?;
    if typed.contains('\\') {
        return None;
    }
    let lead = before.get(..start)?.trim_end();
    let &(kind, open_at, comma) = stack.last()?;
    let (bracket_at, columns, close) = match kind {
        Open::Double if lead.ends_with("[[") => (open_at, false, format!("{quote}]]")),
        Open::Bracket if lead.ends_with('[') || lead.ends_with(',') => {
            (open_at, comma, quote.to_string())
        }
        Open::Paren if lead.ends_with('(') || lead.ends_with(',') => {
            // `c(` straight inside a `[`
            let call = before.get(..open_at)?.trim_end().strip_suffix('c')?;
            if ends_in_name_char(call) {
                return None;
            }
            let rest = call.trim_end();
            let &(outer, outer_at, outer_comma) = stack.iter().rev().nth(1)?;
            if outer != Open::Bracket || !(rest.ends_with('[') || rest.ends_with(',')) {
                return None;
            }
            (outer_at, outer_comma, quote.to_string())
        }
        _ => return None,
    };
    let object = object_before(before.get(..bracket_at)?)?;
    Some(ColumnSpot {
        object,
        columns,
        typed: typed.to_string(),
        close,
    })
}

fn ends_in_name_char(s: &str) -> bool {
    s.chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '.' || c == '_')
}

/// The name (and `$` parts) `text` ends in: `df`, `results$by_group`.
fn object_before(text: &str) -> Option<Vec<String>> {
    let start = text
        .char_indices()
        .rev()
        .take_while(|&(_, c)| c.is_alphanumeric() || c == '.' || c == '_' || c == '$')
        .last()
        .map(|(i, _)| i)?;
    let parts: Vec<String> = text.get(start..)?.split('$').map(str::to_string).collect();
    let valid = |p: &String| {
        let mut cs = p.chars();
        match cs.next() {
            Some('.') => !cs.next().is_some_and(|c| c.is_ascii_digit()),
            Some(c) => c.is_alphabetic(),
            None => false,
        }
    };
    parts.iter().all(valid).then_some(parts)
}

/// R code that prints, in [`completion_code`]'s form, the names at `spot`
/// that start with what was typed. The object is looked up by name; nothing
/// on the line is run.
pub fn column_code(spot: &ColumnSpot) -> String {
    let mut code = String::from("local({\n");
    let mut parts = spot.object.iter();
    if let Some(first) = parts.next() {
        code.push_str(&format!(
            "x <- get0({}, envir = globalenv())\n",
            r_string(first)
        ));
    }
    for part in parts {
        code.push_str(&format!(
            "x <- if (is.list(x)) x[[{}, exact = TRUE]] else NULL\n",
            r_string(part)
        ));
    }
    code.push_str(&format!(
        "typed <- {typed}\n\
         nm <- if (is.function(x)) character() else as.character({names}(x))\n\
         nm <- nm[!is.na(nm) & nzchar(nm) & !grepl(\"[\\t\\n\\r]\", nm)]\n\
         found <- sort(unique(nm[startsWith(nm, typed)]))\n\
         cat(\"TOKEN\\t\", typed, \"\\n\", sep = \"\")\n\
         for (n in head(found, {max})) cat(\"ITEM\\t\", n, \"\\n\", sep = \"\")\n\
         }})",
        typed = r_string(&spot.typed),
        names = if spot.columns { "colnames" } else { "names" },
        max = MAX_OPTIONS + 1,
    ));
    code
}

/// What Tab asks R about the text before the cursor: the code to run, and
/// what closes a single finished choice (see [`finish`]).
pub fn query(before: &str) -> (String, String) {
    match column_spot(before) {
        Some(spot) => (column_code(&spot), spot.close),
        None => (completion_code(before), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_is_quoted_into_the_code() {
        let code = completion_code("paste(\"a\\b\", my_d");
        assert!(code.contains(r#"line <- "paste(\"a\\b\", my_d""#));
        assert!(code.contains("utils:::.completeToken()"));
    }

    #[test]
    fn an_answer_becomes_a_completion() {
        let answer = "TOKEN\tmy_d\nITEM\tmy_data\nITEM\tmy_dates\n";
        let c = parse_answer("x <- my_d", answer);
        assert_eq!(
            c,
            Some(Completion {
                from: 5,
                options: vec!["my_data".into(), "my_dates".into()],
            })
        );
    }

    #[test]
    fn nothing_offered_or_a_token_that_is_not_the_end_gives_none() {
        assert_eq!(parse_answer("a", "TOKEN\ta\n"), None);
        assert_eq!(parse_answer("a", ""), None);
        assert_eq!(parse_answer("abc", "TOKEN\tzz\nITEM\tzzz\n"), None);
        assert_eq!(parse_answer("abc", "ITEM\tabcd\n"), None);
        assert_eq!(parse_answer("abc", "garbage\n\u{0}\n"), None);
    }

    #[test]
    fn a_long_answer_is_cut_and_sorted() {
        let mut answer = String::from("TOKEN\ta\n");
        for i in (0..100).rev() {
            answer.push_str(&format!("ITEM\ta{i:03}\n"));
        }
        let c = parse_answer("a", &answer).expect("completion");
        assert_eq!(c.options.len(), MAX_OPTIONS);
        assert_eq!(c.options.first().map(String::as_str), Some("a000"));
    }

    #[test]
    fn tab_completes_as_far_as_the_choices_agree() {
        let c = Completion {
            from: 5,
            options: vec!["my_data".into(), "my_dates".into()],
        };
        let (line, shown) = apply("x <- my_d", &c);
        assert_eq!(line, "x <- my_dat");
        assert_eq!(shown.len(), 2);
    }

    #[test]
    fn one_choice_is_finished_without_a_space() {
        let one = |from, s: &str| Completion {
            from,
            options: vec![s.to_string()],
        };
        assert_eq!(apply("my_d", &one(0, "my_data")).0, "my_data");
        assert_eq!(apply("mea", &one(0, "mean(")).0, "mean(");
        assert_eq!(apply("library(sta", &one(8, "stats")).0, "library(stats");
        assert_eq!(apply("read.csv(\"d", &one(10, "dir/")).0, "read.csv(\"dir/");
    }

    #[test]
    fn arguments_come_first_and_a_finished_one_reads_name_equals() {
        let answer = "TOKEN\tna\nITEM\tna.omit(\nITEM\tna.rm=\nITEM\tnames(\n";
        let c = parse_answer("mean(x, na", answer).expect("completion");
        assert_eq!(c.options, vec!["na.rm=", "na.omit(", "names("]);
        let one = Completion {
            from: 8,
            options: vec!["na.rm=".into()],
        };
        assert_eq!(apply("mean(x, na", &one).0, "mean(x, na.rm = ");
        // an operator is not an argument
        assert!(!is_argument("=="));
        assert!(!is_argument("="));
        assert!(is_argument("...="));
    }

    fn spot(object: &[&str], columns: bool, typed: &str, close: &str) -> Option<ColumnSpot> {
        Some(ColumnSpot {
            object: object.iter().map(|s| s.to_string()).collect(),
            columns,
            typed: typed.into(),
            close: close.into(),
        })
    }

    #[test]
    fn column_spots_are_found() {
        assert_eq!(column_spot("df[[\"mp"), spot(&["df"], false, "mp", "\"]]"));
        assert_eq!(column_spot("df[['"), spot(&["df"], false, "", "']]"));
        assert_eq!(
            column_spot("mean(df[[ \"a"),
            spot(&["df"], false, "a", "\"]]")
        );
        assert_eq!(column_spot("df[\"a"), spot(&["df"], false, "a", "\""));
        assert_eq!(column_spot("df[, \"a"), spot(&["df"], true, "a", "\""));
        assert_eq!(
            column_spot("df[df$x > 1, c(\"a\", \"b"),
            spot(&["df"], true, "b", "\"")
        );
        assert_eq!(column_spot("df[c(\"a"), spot(&["df"], false, "a", "\""));
        assert_eq!(
            column_spot("res$by_group[[\"n"),
            spot(&["res", "by_group"], false, "n", "\"]]")
        );
        assert_eq!(
            column_spot("x <- my.d[, 'é"),
            spot(&["my.d"], true, "é", "'")
        );
    }

    #[test]
    fn other_quotes_are_not_column_spots() {
        for line in [
            "read.csv(\"da",
            "paste(\"a",
            "abc(\"a",
            "df[[1]][\"a\"",
            "f()[[\"a",
            "df[[\"a\\",
            "x[`a",
            "1df[[\"a",
            "# df[[\"a",
            "df[[\"a\"]] + \"b",
            "df[x == \"a",
            "",
        ] {
            assert_eq!(column_spot(line), None, "{line}");
        }
    }

    #[test]
    fn column_code_looks_the_object_up_by_name() {
        let code = column_code(&ColumnSpot {
            object: vec!["res".into(), "tab".into()],
            columns: true,
            typed: "a\"b".into(),
            close: "\"".into(),
        });
        assert!(code.contains("x <- get0(\"res\", envir = globalenv())"));
        assert!(code.contains("x[[\"tab\", exact = TRUE]]"));
        assert!(code.contains("colnames(x)"));
        assert!(code.contains(r#"typed <- "a\"b""#));
        let (code, close) = query("df[[\"a");
        assert!(code.contains("names(x)") && close == "\"]]");
        let (code, close) = query("mea");
        assert!(code.contains(".completeToken") && close.is_empty());
    }

    #[test]
    fn finishing_keeps_what_follows_the_cursor() {
        let one = |from, s: &str| Completion {
            from,
            options: vec![s.to_string()],
        };
        // in the middle of a line
        let (line, cursor, shown) = finish("summary(my_d", ")", &one(8, "my_data"), "");
        assert_eq!(
            (line.as_str(), cursor, shown.len()),
            ("summary(my_data)", 15, 0)
        );
        // a column at the end closes its quote and brackets
        let (line, cursor, _) = finish("df[[\"mp", "", &one(5, "mpg"), "\"]]");
        assert_eq!((line.as_str(), cursor), ("df[[\"mpg\"]]", 11));
        // ... only the quote when something follows, nothing when the quote is there
        assert_eq!(
            finish("df[, \"mp", "]", &one(6, "mpg"), "\"").0,
            "df[, \"mpg\"]"
        );
        assert_eq!(
            finish("df[[\"mp", "\"]]", &one(5, "mpg"), "\"]]").0,
            "df[[\"mpg\"]]"
        );
        // several choices close nothing
        let two = Completion {
            from: 5,
            options: vec!["mpg".into(), "mpx".into()],
        };
        let (line, cursor, shown) = finish("df[[\"m", "", &two, "\"]]");
        assert_eq!((line.as_str(), cursor, shown.len()), ("df[[\"mp", 7, 2));
        // characters, not bytes, for the cursor
        assert_eq!(finish("é <- x", "", &one(6, "xé"), "").1, 7);
    }
}
