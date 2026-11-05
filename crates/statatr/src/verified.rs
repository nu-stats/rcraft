//! Which translated commands are Verified (P2B.10): each one, and the files
//! of the Stata corpus (`engine/tests/stata_corpus`) that check it against
//! Stata 19's own results, to 8 significant digits (6 for the models Stata
//! stops iterating at its tolerance; exactly for the random draws).
//!
//! The list is the corpus's: a test holds it equal to the `commands:` line
//! of every corpus file (and the functions to the `functions:` line), and
//! another runs the files and compares their
//! results, so a command is Verified only while a file that uses it passes.
//! The forms checked are the ones in those files; options not in them are
//! not Verified by this, though most have tests of their own against Stata.

/// Each Verified command, with the corpus files that check it.
pub const VERIFIED: &[(&str, &[&str])] = &[
    ("anova", &["panel"]),
    ("append", &["replication"]),
    ("assert", &["checks"]),
    ("bootstrap", &["bootstrap"]),
    ("bsample", &["bsample"]),
    ("clogit", &["clogit"]),
    ("coefplot", &["coefplot"]),
    ("collapse", &["data"]),
    ("correlate", &["descriptives"]),
    ("count", &["control", "descriptives"]),
    ("egen", &["descriptives"]),
    ("erase", &["replication"]),
    ("estadd", &["estout"]),
    ("estimates", &["estimates"]),
    ("eststo", &["estout"]),
    ("esttab", &["esttab"]),
    ("expand", &["made", "random"]),
    ("file", &["strings"]),
    ("foreach", &["aware", "control"]),
    ("forvalues", &["control"]),
    ("fre", &["fre"]),
    ("fvset", &["replication"]),
    ("generate", &["control", "data", "descriptives", "random"]),
    ("gettoken", &["made"]),
    ("global", &["control"]),
    ("gsort", &["data"]),
    (
        "import",
        &[
            "aware",
            "control",
            "data",
            "descriptives",
            "likelihood",
            "panel",
            "regress",
        ],
    ),
    ("include", &["control"]),
    ("input", &["arpanel"]),
    ("insheet", &["matrices"]),
    ("ivregress", &["panel"]),
    ("label", &["aware"]),
    ("levelsof", &["aware"]),
    ("lincom", &["panel", "regress"]),
    ("local", &["control"]),
    ("logit", &["collinear", "interactions", "likelihood"]),
    ("margins", &["regress"]),
    ("matrix", &["matrices"]),
    ("merge", &["merge"]),
    ("mkmat", &["matrices"]),
    ("mlogit", &["collinear", "likelihood"]),
    ("nbreg", &["collinear"]),
    ("newey", &["arpanel"]),
    ("ologit", &["collinear", "likelihood"]),
    ("oprobit", &["collinear", "likelihood"]),
    ("outreg2", &["outreg2"]),
    ("parmest", &["clogit_margins"]),
    ("poisson", &["collinear", "likelihood"]),
    ("prais", &["arpanel"]),
    ("predict", &["regress"]),
    ("preserve", &["made"]),
    ("probit", &["collinear", "likelihood"]),
    ("program", &["programs"]),
    ("pwcorr", &["descriptives"]),
    ("recode", &["data"]),
    ("reghdfe", &["reghdfe"]),
    (
        "regress",
        &["aware", "collinear", "control", "interactions", "regress"],
    ),
    ("replace", &["data", "random"]),
    ("reshape", &["replication"]),
    ("restore", &["made"]),
    ("robreg", &["robreg"]),
    ("set", &["made", "random"]),
    ("sort", &["data"]),
    ("split", &["strings"]),
    ("stcox", &["likelihood"]),
    ("stset", &["likelihood"]),
    ("summarize", &["aware", "control", "descriptives"]),
    ("svmat", &["matrices"]),
    ("syntax", &["programs"]),
    ("tabulate", &["checks", "descriptives"]),
    ("tempvar", &["control"]),
    ("test", &["regress"]),
    ("tokenize", &["made"]),
    ("tsset", &["timeseries"]),
    ("ttest", &["descriptives"]),
    ("unab", &["aware"]),
    ("while", &["control"]),
    ("xpose", &["made"]),
    ("xtile", &["xtile"]),
    ("xtreg", &["likelihood", "panel"]),
    ("xtregar", &["arpanel"]),
    ("xtset", &["likelihood", "panel"]),
];

/// Each Verified function, with the corpus files that check it (the
/// `functions:` line of each), where a function was the point of a file.
pub const VERIFIED_FUNCTIONS: &[(&str, &[&str])] = &[
    ("char", &["strings"]),
    ("clock", &["dates"]),
    ("date", &["dates"]),
    ("dhms", &["dates"]),
    ("dofc", &["dates"]),
    ("dofm", &["dates"]),
    ("dofq", &["dates"]),
    ("dofw", &["dates"]),
    ("dow", &["dates"]),
    ("doy", &["dates"]),
    ("float", &["strings"]),
    ("halfyearly", &["dates"]),
    ("hh", &["dates"]),
    ("hms", &["dates"]),
    ("invnormal", &["random"]),
    ("length", &["strings"]),
    ("ltrim", &["strings"]),
    ("mdy", &["dates"]),
    ("mdyhms", &["dates"]),
    ("mm", &["dates"]),
    ("mofd", &["dates"]),
    ("monthly", &["dates"]),
    ("normal", &["random"]),
    ("qofd", &["dates"]),
    ("quarter", &["dates"]),
    ("quarterly", &["dates"]),
    ("rbinomial", &["variates"]),
    ("rcauchy", &["variates"]),
    ("rchi2", &["variates"]),
    ("rexponential", &["variates"]),
    ("rgamma", &["random"]),
    ("rgeometric", &["variates"]),
    ("rhypergeometric", &["variates"]),
    ("rigaussian", &["variates"]),
    ("rlaplace", &["variates"]),
    ("rlogistic", &["variates"]),
    ("rnbinomial", &["variates"]),
    ("rnormal", &["random"]),
    ("rpareto", &["variates"]),
    ("rtrim", &["strings"]),
    ("runiform", &["random"]),
    ("runiformint", &["variates"]),
    ("rweibull", &["variates"]),
    ("rweibullph", &["variates"]),
    ("ss", &["dates"]),
    ("strltrim", &["strings"]),
    ("strmatch", &["strings"]),
    ("strrtrim", &["strings"]),
    ("strtrim", &["strings"]),
    ("subinstr", &["strings"]),
    ("trim", &["strings"]),
    ("ustrltrim", &["strings"]),
    ("ustrrtrim", &["strings"]),
    ("ustrtrim", &["strings"]),
    ("week", &["dates"]),
    ("weekly", &["dates"]),
    ("wofd", &["dates"]),
    ("year", &["dates"]),
    ("yearly", &["dates"]),
    ("yq", &["dates"]),
    ("yw", &["dates"]),
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
        assert_eq!(
            files("regress"),
            Some(&["aware", "collinear", "control", "interactions", "regress"][..])
        );
        assert_eq!(files("xtlogit"), None);
        let functions: Vec<&str> = VERIFIED_FUNCTIONS.iter().map(|(f, _)| *f).collect();
        let mut sorted = functions.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(functions, sorted);
    }
}
