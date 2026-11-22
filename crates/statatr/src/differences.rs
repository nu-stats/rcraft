//! The differences guide (P2B.8): for each command Stata mode translates,
//! where R and Stata differ and what the translation does about it.
//!
//! Each page lists the points that matter for that command: standard
//! errors, degrees of freedom, ties, missing values, sorting, string widths,
//! float against double, printing. Every point says what Stata does, what R
//! does by itself, and what the R that RCraft writes does (which is Stata's
//! wherever it could be matched, checked against Stata 19). The notes under
//! a translated line are the short form; the Help window shows the page.
//!
//! The pages are written from the translator, never from Stata's files.

/// One difference: its topic, and what to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub topic: &'static str,
    pub text: &'static str,
}

/// The page of one command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Guide {
    /// The command's full name (`summarize`), or `general` for what holds
    /// everywhere.
    pub command: &'static str,
    /// What it does, in one line.
    pub about: &'static str,
    pub points: &'static [Point],
}

const fn p(topic: &'static str, text: &'static str) -> Point {
    Point { topic, text }
}

// Points several commands share.
const MISSING_IN_MODELS: Point = p(
    "Missing values",
    "A row with a missing value in any variable of the model is left out, as Stata does. \
     R's own lm() and glm() do the same by default (na.omit), so the estimation sample is \
     Stata's; e(sample) marks it.",
);
const LAYOUT_R: Point = p(
    "Printing",
    "The results are printed in R's layout, not Stata's table; the numbers are Stata's.",
);
const ML_CNS: Point = p(
    "constraints()",
    "logit, probit, poisson and logistic take constraints(numlist) as Stata 19 does: the \
     likelihood over what the constraints leave free, by Stata's Newton-Raphson from its usual \
     start projected onto them (its iteration log too), the variance on the constraints (a fixed \
     coefficient shows `(constrained)` with a variance of 0), the Wald test of the slopes they \
     leave free in place of the LR test and pseudo R2, the constraint lines as `[y]x1 - [y]x2 = 0`, \
     and e(Cns). logit and probit with noconstant (constraints() or not) print Stata's same Wald \
     header, with no e(ll_0) or pseudo R2, and its log from 0. constraints() with exposure(), \
     offset(), iweights, a regressor omitted for collinearity, or poisson with noconstant is not \
     translated yet.",
);
const ML_SE: Point = p(
    "Standard errors",
    "From the observed information (the Hessian), as Stata's. R's summary() uses the \
     expected information for some models (probit), which differs.",
);
const ML_ROBUST: Point = p(
    "Robust and clustered errors",
    "Robust errors carry N / (N - 1) and clustered ones G / (G - 1), as Stata's do for a \
     maximum-likelihood model (not regress's HC1).",
);
const ML_DIGITS: Point = p(
    "Last digits",
    "Stata stops iterating at its tolerance, about 1e-7 from the optimum; R goes on. \
     Estimates agree to 6 or 7 digits, so an 8th printed digit can differ.",
);
const ML_TABLE: Point = p(
    "Printing",
    "logit, probit, poisson, nbreg, ologit, oprobit and mlogit print Stata 19's header and \
     table (LR or Wald chi2, the log likelihood or pseudolikelihood, Pseudo R2, the clusters' \
     line, factor levels, Odds ratio, IRR and RRR with their notes; the cut points, /lnalpha \
     and alpha below their rules; nbreg's Dispersion line and test of alpha=0; each mlogit \
     outcome's equation under its value label, the base outcome marked). Each prints Stata's \
     iteration log too, its own Newton steps: logit, probit, ologit, oprobit and mlogit from \
     the constant-only fit; poisson from Stata's start (the least-squares slopes over the \
     mean of y, the constant that fits the total, one Newton step); nbreg in Stata's three \
     stages (the Poisson model, the constant-only model, the full model). The log is left \
     out with weights, an offset or exposure, or no constant, where Stata's start is not \
     known; a model with an offset prints in R's layout.",
);
const ML_EXACT: Point = p(
    "Where Stata stops",
    "The estimates and their variance are where Stata's own Newton-Raphson stops, from the \
     constant-only fit, with the exact Hessian; R's optimiser stops a hair away and polr's \
     variance is a numerical Hessian's, which moved the last printed digit.",
);
const GRAPH: Point = p(
    "Look",
    "Drawn with ggplot2, so it looks like R's graphs, not Stata's; what is drawn (bins, \
     bandwidths, fitted lines) is worked out as Stata does. Options that only change \
     Stata's look are not carried over and are named in a note.",
);
const PREDICT_ROWS: Point = p(
    "Rows predicted",
    "Every row that has the values the model needs, in the estimation sample or not, as \
     Stata does; R's predict() on new data does the same.",
);

// One file per page in `differences/` (and `*.aliases.rs` for the command names that
// share a page), gathered by build.rs into `GUIDES` (the general page first) and `ALIASES`.
include!(concat!(env!("OUT_DIR"), "/differences.rs"));

/// The page a command has (`reg` is `regress`; `probit` shares `logit`'s,
/// `tab1` and `tab2` `tabulate`'s, `scatter` and `line` `twoway`'s).
pub fn guide(command: &str) -> Option<&'static Guide> {
    let full = full(command);
    let page = ALIASES
        .iter()
        .find(|(_, names)| names.contains(&full.as_str()))
        .map_or(full.as_str(), |(page, _)| *page);
    GUIDES.iter().find(|g| g.command == page)
}

/// The page of the command a Stata line runs, past its prefixes.
pub fn for_line(line: &str) -> Option<&'static Guide> {
    let mut rest = line.trim();
    loop {
        let word: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let after = rest.get(word.len()..).unwrap_or("").trim_start();
        match full(&word).as_str() {
            "quietly" | "capture" | "noisily" => {
                rest = after.trim_start_matches(':').trim_start();
                // a block (`quietly {`): the page of the prefix itself
                if rest.is_empty() || rest.starts_with('{') {
                    return guide(&word).or_else(|| guide("capture"));
                }
            }
            "by" | "bysort" => {
                // the command after the colon
                match after.split_once(':') {
                    Some((_, cmd)) if !cmd.trim().is_empty() => rest = cmd.trim(),
                    _ => return guide("by"),
                }
            }
            _ => return guide(&word),
        }
    }
}

/// A command word's full name.
fn full(word: &str) -> String {
    const MORE: &[(&str, usize)] = &[
        ("quietly", 3),
        ("forvalues", 4),
        ("capture", 3),
        ("noisily", 3),
        ("bysort", 3),
        ("by", 2),
        ("histogram", 4),
        ("kdensity", 5),
        ("scatter", 2),
        ("connected", 4),
        ("twoway", 2),
        ("graph", 2),
        ("logit", 4),
        ("probit", 4),
        ("poisson", 4),
        ("xtreg", 5),
        ("ttest", 5),
        ("pwcorr", 6),
        ("predict", 4),
        ("margins", 6),
    ];
    if let Some((name, _)) = MORE
        .iter()
        .find(|(n, min)| crate::lang::syntax::abbrev(word, n, *min))
    {
        return (*name).to_string();
    }
    // the other names for frame commands
    if matches!(word, "cwf" | "pwf" | "frames" | "frlink" | "frget") {
        return "frame".to_string();
    }
    // the other names Stata takes for bootstrap
    if matches!(word, "bs" | "bstrap") {
        return "bootstrap".to_string();
    }
    crate::lang::commands::resolve(word).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_offered_has_a_page() {
        // the prefixes, which say nothing of their own
        let reader = ["capture", "quietly", "noisily"];
        for cmd in crate::complete::COMMANDS
            .iter()
            .filter(|c| !reader.contains(c))
        {
            let g = guide(cmd);
            assert!(g.is_some(), "{cmd} has no page");
            assert!(
                g.is_some_and(|g| !g.points.is_empty()),
                "{cmd}'s page is empty"
            );
        }
        // each page once, the general one first
        assert_eq!(GUIDES.first().map(|g| g.command), Some("general"));
        let mut names: Vec<&str> = GUIDES.iter().map(|g| g.command).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), GUIDES.len());
    }

    #[test]
    fn each_page_is_in_the_file_named_for_it() {
        assert_eq!(GUIDES.len(), GUIDE_FILES.len());
        for (g, file) in GUIDES.iter().zip(GUIDE_FILES) {
            assert_eq!(
                g.command, *file,
                "differences/{file}.rs holds the page of {}",
                g.command
            );
        }
        // an alias names a page, and is not a page of its own
        for (page, names) in ALIASES {
            assert!(
                GUIDES.iter().any(|g| g.command == *page),
                "{page}.aliases.rs has no page"
            );
            for n in *names {
                assert!(
                    GUIDES.iter().all(|g| g.command != *n),
                    "{n} is an alias and a page"
                );
            }
        }
    }

    #[test]
    fn a_line_finds_its_commands_page() {
        let page = |l: &str| for_line(l).map(|g| g.command);
        assert_eq!(page("reg y x"), Some("regress"));
        assert_eq!(page("quietly summarize y"), Some("summarize"));
        assert_eq!(page("qui: tab g"), Some("tabulate"));
        assert_eq!(page("bysort g: generate n = _n"), Some("generate"));
        assert_eq!(page("by g"), Some("by"));
        assert_eq!(page("probit d x"), Some("logit"));
        assert_eq!(page("tab2 g h"), Some("tabulate"));
        assert_eq!(page("scatter y x"), Some("twoway"));
        assert_eq!(
            page("qui bs m = r(mean), reps(5): sum y"),
            Some("bootstrap")
        );
        assert_eq!(page("prop g"), Some("proportion"));
        assert_eq!(page("capture rm out.dta"), Some("erase"));
        assert_eq!(page("args a b"), Some("program"));
        assert_eq!(page("marksample touse"), Some("program"));
        assert_eq!(page("mata: mata clear"), Some("mata"));
        assert_eq!(page("xtlogit y x"), None);
    }
}
