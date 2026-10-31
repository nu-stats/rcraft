//! Which translated commands are Verified (P2B.10): each one, and the files
//! of the Stata corpus (`engine/tests/stata_corpus`) that check it against
//! Stata 19's own results, to 8 significant digits (6 for the models Stata
//! stops iterating at its tolerance).
//!
//! The list is the corpus's: a test holds it equal to the `commands:` line
//! of every corpus file, and another runs the files and compares their
//! results, so a command is Verified only while a file that uses it passes.
//! The forms checked are the ones in those files; options not in them are
//! not Verified by this, though most have tests of their own against Stata.

/// Each Verified command, with the corpus files that check it.
pub const VERIFIED: &[(&str, &[&str])] = &[
    ("anova", &["panel"]),
    ("collapse", &["data"]),
    ("correlate", &["descriptives"]),
    ("count", &["descriptives"]),
    ("egen", &["descriptives"]),
    ("generate", &["data", "descriptives"]),
    ("gsort", &["data"]),
    (
        "import",
        &["data", "descriptives", "likelihood", "panel", "regress"],
    ),
    ("ivregress", &["panel"]),
    ("lincom", &["regress"]),
    ("logit", &["likelihood"]),
    ("margins", &["regress"]),
    ("mlogit", &["likelihood"]),
    ("ologit", &["likelihood"]),
    ("oprobit", &["likelihood"]),
    ("poisson", &["likelihood"]),
    ("predict", &["regress"]),
    ("probit", &["likelihood"]),
    ("pwcorr", &["descriptives"]),
    ("recode", &["data"]),
    ("regress", &["regress"]),
    ("replace", &["data"]),
    ("sort", &["data"]),
    ("stcox", &["likelihood"]),
    ("stset", &["likelihood"]),
    ("summarize", &["descriptives"]),
    ("tabulate", &["descriptives"]),
    ("test", &["regress"]),
    ("ttest", &["descriptives"]),
    ("xtreg", &["likelihood", "panel"]),
    ("xtset", &["likelihood", "panel"]),
];

/// The corpus files that check a command (its full name), if it is Verified.
pub fn files(command: &str) -> Option<&'static [&'static str]> {
    VERIFIED
        .iter()
        .find(|(c, _)| *c == command)
        .map(|(_, f)| *f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_in_order_and_each_command_is_one_the_translator_offers() {
        let names: Vec<&str> = VERIFIED.iter().map(|(c, _)| *c).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted);
        for c in names {
            assert!(crate::complete::COMMANDS.contains(&c), "{c}");
        }
        assert_eq!(files("regress"), Some(&["regress"][..]));
        assert_eq!(files("xtlogit"), None);
    }
}
