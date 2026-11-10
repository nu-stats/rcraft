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
    ("_pctile", &["asclogit"]),
    ("alpha", &["alpha"]),
    ("anova", &["panel"]),
    ("append", &["replication"]),
    ("asclogit", &["asclogit"]),
    ("assert", &["checks"]),
    ("bootstrap", &["bootstrap", "bsopts"]),
    ("bsample", &["bsample", "bsopts"]),
    ("center", &["center"]),
    ("clogit", &["clogit"]),
    ("cluster2", &["cluster2"]),
    ("coefplot", &["coefplot"]),
    (
        "collapse",
        &[
            "data",
            "extended_missing_commands",
            "extended_missing_groups",
        ],
    ),
    ("confirm", &["strings"]),
    ("contract", &["contract", "extended_missing_groups"]),
    ("correlate", &["descriptives"]),
    ("count", &["control", "descriptives", "extended_missing"]),
    ("decode", &["labels2"]),
    ("drawnorm", &["asclogit"]),
    ("duplicates", &["extended_missing_groups"]),
    (
        "egen",
        &[
            "descriptives",
            "extended_missing_commands",
            "extended_missing_groups",
        ],
    ),
    ("erase", &["replication"]),
    ("ereturn", &["eclass"]),
    ("estadd", &["estout"]),
    ("estimates", &["estimates", "suest"]),
    ("estpost", &["esttab2"]),
    ("eststo", &["estout"]),
    ("esttab", &["esttab", "esttab2", "esttab3"]),
    ("expand", &["made", "random"]),
    ("file", &["fileread", "strings"]),
    ("filefilter", &["filefilter"]),
    ("foreach", &["aware", "control"]),
    ("forvalues", &["control"]),
    ("fre", &["extended_missing_commands", "fre"]),
    ("frmttable", &["outreg"]),
    ("fvset", &["replication"]),
    (
        "generate",
        &[
            "control",
            "data",
            "descriptives",
            "extended_missing",
            "random",
        ],
    ),
    ("gettoken", &["made"]),
    ("global", &["control"]),
    ("gsort", &["data", "extended_missing_commands"]),
    (
        "import",
        &[
            "aware",
            "control",
            "data",
            "descriptives",
            "likelihood",
            "panel",
            "putexcel",
            "regress",
        ],
    ),
    ("include", &["control"]),
    (
        "input",
        &[
            "arpanel",
            "extended_missing",
            "extended_missing_commands",
            "extended_missing_groups",
        ],
    ),
    ("insheet", &["matrices"]),
    ("isid", &["survwgt"]),
    ("isvar", &["isvar"]),
    ("ivregress", &["panel"]),
    ("label", &["aware", "labels2"]),
    ("levelsof", &["aware", "extended_missing_groups"]),
    ("lincom", &["panel", "regress"]),
    ("local", &["control"]),
    ("logit", &["collinear", "interactions", "likelihood"]),
    ("margins", &["regress"]),
    ("matrix", &["matfuncs", "matrices"]),
    ("merge", &["extended_missing_groups", "merge"]),
    ("mkmat", &["matrices"]),
    ("mlogit", &["collinear", "likelihood"]),
    ("nbreg", &["collinear"]),
    ("newey", &["arpanel"]),
    ("ologit", &["collinear", "likelihood"]),
    ("oprobit", &["collinear", "likelihood"]),
    ("outreg", &["outreg"]),
    ("outreg2", &["outreg2", "outreg2tex"]),
    ("parmest", &["clogit_margins"]),
    ("poisson", &["collinear", "likelihood"]),
    ("post", &["postfile"]),
    ("postclose", &["postfile"]),
    ("postfile", &["postfile"]),
    ("prais", &["arpanel"]),
    ("predict", &["regress"]),
    ("preserve", &["made"]),
    ("probit", &["collinear", "likelihood", "perfect_predict"]),
    ("program", &["programs"]),
    ("putexcel", &["putexcel"]),
    ("pwcorr", &["descriptives"]),
    ("recast", &["contract"]),
    (
        "recode",
        &["data", "extended_missing", "extended_missing_commands"],
    ),
    ("reghdfe", &["reghdfe"]),
    (
        "regress",
        &[
            "aware",
            "collinear",
            "control",
            "interactions",
            "regress",
            "regress_exact",
        ],
    ),
    ("replace", &["data", "extended_missing", "random"]),
    ("reshape", &["replication"]),
    ("restore", &["made"]),
    ("robreg", &["robreg"]),
    ("set", &["made", "random"]),
    ("sort", &["data", "extended_missing_commands"]),
    ("split", &["strings"]),
    ("stcox", &["likelihood"]),
    ("stset", &["likelihood"]),
    ("suest", &["suest"]),
    (
        "summarize",
        &["aware", "control", "descriptives", "extended_missing"],
    ),
    ("survwgt", &["survwgt"]),
    ("svmat", &["matrices"]),
    ("svy", &["svy"]),
    ("svyset", &["svy"]),
    ("syntax", &["programs"]),
    ("tab1", &["extended_missing_commands"]),
    ("tab2", &["extended_missing_commands"]),
    (
        "tabulate",
        &["checks", "descriptives", "extended_missing_commands"],
    ),
    ("tempvar", &["control"]),
    ("test", &["regress"]),
    ("tokenize", &["made"]),
    ("tsset", &["timeseries"]),
    ("ttest", &["descriptives"]),
    ("unab", &["aware"]),
    ("while", &["control"]),
    ("winsor", &["winsor"]),
    ("xpose", &["made"]),
    ("xtile", &["xtile"]),
    (
        "xtreg",
        &[
            "likelihood",
            "panel",
            "xtabsorbed",
            "xtcollinear",
            "xtreg_r2",
        ],
    ),
    ("xtregar", &["arpanel"]),
    ("xtset", &["likelihood", "panel", "xtreg_r2"]),
];

/// Each Verified function, with the corpus files that check it (the
/// `functions:` line of each), where a function was the point of a file.
pub const VERIFIED_FUNCTIONS: &[(&str, &[&str])] = &[
    ("Cdhms", &["dates"]),
    ("Chms", &["dates"]),
    ("Clock", &["dates"]),
    ("Cmdyhms", &["dates"]),
    ("Cofc", &["dates"]),
    ("Cofd", &["dates"]),
    ("ceil", &["extended_missing"]),
    ("char", &["strings"]),
    ("clock", &["dates"]),
    ("cofC", &["dates"]),
    ("colnumb", &["matfuncs"]),
    ("cond", &["extended_missing"]),
    ("date", &["dates"]),
    ("det", &["matfuncs"]),
    ("dhms", &["dates"]),
    ("diag0cnt", &["matfuncs"]),
    ("dofC", &["dates"]),
    ("dofc", &["dates"]),
    ("dofm", &["dates"]),
    ("dofq", &["dates"]),
    ("dofw", &["dates"]),
    ("dow", &["dates"]),
    ("doy", &["dates"]),
    ("float", &["strings"]),
    ("floor", &["extended_missing"]),
    ("halfyearly", &["dates"]),
    ("hh", &["dates"]),
    ("hhC", &["dates"]),
    ("hms", &["dates"]),
    ("inlist", &["extended_missing"]),
    ("int", &["extended_missing"]),
    ("invlogit", &["strings"]),
    ("invnormal", &["random"]),
    ("issymmetric", &["matfuncs"]),
    ("length", &["strings"]),
    ("logit", &["strings"]),
    ("ltrim", &["strings"]),
    ("matmissing", &["matfuncs"]),
    ("mdy", &["dates"]),
    ("mdyhms", &["dates"]),
    ("mm", &["dates"]),
    ("mmC", &["dates"]),
    ("mofd", &["dates"]),
    ("monthly", &["dates"]),
    ("mreldif", &["matfuncs"]),
    ("normal", &["random"]),
    ("qofd", &["dates"]),
    ("quarter", &["dates"]),
    ("quarterly", &["dates"]),
    ("rbeta", &["variates2"]),
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
    ("round", &["extended_missing"]),
    ("rownumb", &["matfuncs"]),
    ("rpareto", &["variates"]),
    ("rt", &["variates2"]),
    ("rtrim", &["strings"]),
    ("runiform", &["random"]),
    ("runiformint", &["variates"]),
    ("rweibull", &["variates"]),
    ("rweibullph", &["variates"]),
    ("ss", &["dates"]),
    ("ssC", &["dates"]),
    ("strltrim", &["strings"]),
    ("strmatch", &["strings"]),
    ("strrtrim", &["strings"]),
    ("strtrim", &["strings"]),
    ("subinstr", &["strings"]),
    ("trace", &["matfuncs"]),
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
            Some(
                &[
                    "aware",
                    "collinear",
                    "control",
                    "interactions",
                    "regress",
                    "regress_exact"
                ][..]
            )
        );
        assert_eq!(files("xtlogit"), None);
        let functions: Vec<&str> = VERIFIED_FUNCTIONS.iter().map(|(f, _)| *f).collect();
        let mut sorted = functions.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(functions, sorted);
    }
}
