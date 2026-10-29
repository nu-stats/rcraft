//! Tab completion for the Command window (P2.2).
//!
//! R already knows how to complete a line: function and object names, the
//! columns after `$`, `package::name`, package names inside `library(`, and
//! file names inside quotes. This module only asks R to do it (through
//! `utils`, which is part of R) and reads the answer, so what Tab offers is
//! what R's own console would offer and it always matches the session in use.

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
         found <- sort(unique(as.character(utils:::.retrieveCompletions())))\n\
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
    options.sort();
    options.dedup();
    options.truncate(MAX_OPTIONS);
    Some(Completion {
        from: line.len() - token.len(),
        options,
    })
}

/// The line after Tab: the word completed as far as the choices agree. The
/// second value is the choices to show when there are several. No space is
/// added after a single choice, since the next character depends on the call
/// being typed.
pub fn apply(line: &str, c: &Completion) -> (String, Vec<String>) {
    let typed = line.get(c.from..).unwrap_or("");
    let common = crate::rc::common_prefix(&c.options);
    let mut out = line.get(..c.from).unwrap_or("").to_string();
    out.push_str(if common.len() > typed.len() {
        &common
    } else {
        typed
    });
    if c.options.len() == 1 {
        return (out, Vec::new());
    }
    (out, c.options.clone())
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
}
