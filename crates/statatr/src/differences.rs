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

/// Every page, the general one first.
pub const GUIDES: &[Guide] = &[
    Guide {
        command: "general",
        about: "What differs between Stata and the R that Stata mode writes, for every command",
        points: &[
            p(
                "Missing values",
                "Stata's missing value (.) is larger than every number, so x > 5 is true for a \
                 missing x; R's NA compared with anything is NA. The translation writes \
                 Stata's rule out (is.na(x) | x > 5), so conditions keep the rows Stata keeps. \
                 The extended missing values .a to .z are kept as haven's tagged NAs and \
                 compare, sort, group and tabulate in Stata's order (. < .a < ... < .z).",
            ),
            p(
                "Float and double",
                "generate without a type makes a float in Stata (about 7 digits); R keeps every \
                 number as a double. The translation rounds a new variable to float as Stata \
                 stores it, unless the do-file says double, so x == 0.1 behaves as in Stata. \
                 replace keeps a float a float in Stata, and so does the translation for a \
                 variable the do-file made, also after it saved the data and read it back with \
                 use or merge; R keeps full precision for one it did not make, with a note.",
            ),
            p(
                "String widths",
                "str5 holds 5 characters in Stata; the translation cuts the text to that width. \
                 Text has no width in R otherwise.",
            ),
            p(
                "Sorting",
                "Stata's sort leaves rows that tie in no set order unless sort, stable; R's \
                 order() keeps them as they were, which is one of the orders Stata may give.",
            ),
            p(
                "Dates and times",
                "Stata's dates are numbers (days, milliseconds, months, ... since 1960), and so \
                 they stay in R: date(), clock(), monthly() and the others read text against \
                 Stata's masks (DMY, YMDhms, 20YMD, #) with Stata's rules, and mdy(), year(), \
                 dofm() and the rest give Stata's numbers, checked against about 13,000 of \
                 Stata 19's answers. They are not R Date or POSIXct values. Outside 0100 to \
                 9999 they are missing (wofd() there gives Stata arbitrary numbers). The \
                 leap-second functions (Clock(), Cofd(), ...) are not translated yet.",
            ),
            p(
                "Random numbers",
                "set seed, the random functions (runiform(), rnormal(), rgamma(), rbinomial(), \
                 runiformint(), rexponential(), ...), bsample and bootstrap give Stata's own \
                 draws: RCraft runs Stata's generator (the 64-bit Mersenne Twister) in R, row by \
                 row as Stata draws. rt() is Stata's draw to within its last bit or so, the \
                 draws after it exactly Stata's; rbeta() is Stata's draw to its last bits or so, \
                 its stream exactly Stata's. rpoisson(), rbeta() with equal shapes above 1, \
                 rbinomial(), rnbinomial() and \
                 rhypergeometric() past the ranges whose method is known (they stop with the \
                 reason), simulate and permute, two draws in one expression and version 13 or \
                 earlier are refused.",
            ),
            p(
                "Distribution functions",
                "ttail(), invttail(), Ftail(), invFtail(), chi2tail(), normalden(), ibeta(), \
                 gammap(), binomial(), poisson(), comb(), lngamma() and the rest of Stata's \
                 t, F, chi2, normal, beta, gamma, binomial and Poisson functions run on R's \
                 pt(), pf(), pchisq(), pbeta(), pgamma(), pbinom() and ppois(), which give \
                 Stata 19's doubles to 1e-9 or closer. Where R gives a number and Stata a \
                 missing value, the translation gives Stata's: degrees of freedom, shapes or \
                 a standard deviation of 0 or below, t, F and chi2 degrees of freedom above \
                 2e17 and beta and gamma shapes above 1e17, k outside 0 to n in binomialp(), \
                 and comb() or lnfactorial() of a number that is not whole. The counts of \
                 binomial() and poisson() are cut to whole numbers, as Stata does, and \
                 normalden(., s) is 0, as in Stata. invF() and invFtail() come from qbeta(), \
                 since R's qf() gives 0 at small p and an approximation for df2 above 4e5. \
                 Stata's limits are kept too: missing for a Poisson mean above 2^53, \
                 poissonp() with a mean above 1e8 or a count above 1e9, binomialp() with n \
                 above 1e6, and invt() and invttail() past 9.3675e153. Only where Stata's own \
                 number loses digits do the two differ: invt() below p = 1e-200 with a few \
                 degrees of freedom, the densities with degrees of freedom of 1e10 or more, and \
                 the Poisson probabilities near a mean of 1e15 (by 1e-9); RCraft gives R's \
                 exact number there.",
            ),
            p(
                "Returned values",
                "e() and r() are kept in R as stata_e and stata_r, with Stata's names and \
                 values; one Stata keeps that RCraft does not is refused with the reason, not \
                 left missing.",
            ),
            p(
                "What is not translated",
                "A line RCraft cannot translate exactly is kept as a comment with the reason, \
                 never guessed.",
            ),
        ],
    },
    Guide {
        command: "_pctile",
        about: "Percentiles into r(r1), r(r2), ...",
        points: &[p(
            "As Stata 19",
            "Stata's default percentiles (the mean of the two values at a whole rank), for \
             percentiles() or nquantiles(); weights and altdef are not translated yet.",
        )],
    },
    Guide {
        command: "asclogit",
        about: "Alternative-specific conditional logit",
        points: &[
            p(
                "As Stata 19",
                "With alternative-specific variables only and noconstant it is clogit on the \
                 cases: Stata 19's estimates and variance are clogit's, to the last digit. Its \
                 notes (cases with no within-case variability), header, e() (e(N_case), \
                 e(k_alt), e(alteqs), alt: as the equation of e(b)), e(sample) and estat \
                 summarize are Stata's. Like clogit, the estimates are where Stata's \
                 Newton-Raphson stops, from an ordinary logit's slopes.",
            ),
            p(
                "Not yet",
                "The alternatives' constants (without noconstant), casevars(), \
                 basealternative(), weights, predict and the other estat subcommands.",
            ),
        ],
    },
    Guide {
        command: "drawnorm",
        about: "Draws from a multivariate normal",
        points: &[p(
            "As Stata 19",
            "Stata's draws, bit for bit: its rnormal() stream taken a variable at a time, the \
             deviations L z (L the lower Cholesky factor of cov()) kept in float, the means added; \
             a float variable is rounded again. n() with clear, means() and cov() as matrices \
             (e(b), e(V) too), double; corr(), sds() and seed() are not translated yet.",
        )],
    },
    Guide {
        command: "anova",
        about: "Analysis of variance",
        points: &[
            p(
                "Variables",
                "A variable is categorical unless written c.x, as in Stata's anova; in R's \
                 formula a number is continuous unless written factor().",
            ),
            p(
                "Sums of squares",
                "Partial (Type III) sums of squares by default, as Stata's; R's anova() gives \
                 sequential (Type I) ones, which the translation uses only with sequential. A \
                 design with an empty cell stops, since Stata's rule there is not matched yet.",
            ),
            MISSING_IN_MODELS,
        ],
    },
    Guide {
        command: "append",
        about: "Adds the rows of another file",
        points: &[p(
            "Types",
            "A string variable in one file and a number in the other stops with an error, as \
             in Stata; R's rbind() would turn both into text. With force the using file's \
             values of such a variable are missing (\"\" or .), as in Stata, which also prints \
             a note for each.",
        )],
    },
    Guide {
        command: "assert",
        about: "Stops when an expression is false",
        points: &[p(
            "Messages",
            "Stata's count of the rows where it is false (23 contradictions in 60 observations, \
             none for a single row), then assertion is false and r(9); rc0 prints them and \
             goes on, and no rows is (null assertion). A missing value counts as large, as in \
             Stata. fast and null are not translated yet.",
        )],
    },
    Guide {
        command: "bootstrap",
        about: "Repeats a command on samples drawn with replacement",
        points: &[
            p(
                "Draws",
                "Each replication resamples the command's estimation sample with Stata's own \
                 generator, as bsample does, so the same seed gives Stata's standard errors. The \
                 replicates are kept as floats, as Stata keeps them, and the observed values as \
                 they are.",
            ),
            p(
                "Options",
                "strata() resamples within each stratum as Stata does (e(N_strata), and the \
                 strata in the header); saving(file [, replace double]) writes the replicates \
                 as a dataset, a variable each labelled with its expression (_b[x] is _b_x), \
                 floats unless double; a file already there without replace is r(602) before \
                 anything runs. estat bootstrap shows the bias and the normal, percentile and \
                 bias-corrected intervals (bc by default, all for the three).",
            ),
            p(
                "Not yet",
                "Statistics are _b after regress, logit or probit, or r() and e() values; bca, \
                 strata() with cluster(), saving()'s every(), predict after it and \
                 vce(bootstrap) are refused with the reason.",
            ),
        ],
    },
    Guide {
        command: "error",
        about: "Stops with a return code",
        points: &[p(
            "Codes",
            "error # stops the do-file, or the program it is in, with return code #, which \
             capture keeps in _rc; error 0 does nothing. In a replication of simulate or \
             permute it ends that replication, whose results are then missing. An expression \
             (error _rc) is not translated yet.",
        )],
    },
    Guide {
        command: "jackknife",
        about: "Repeats a command with each observation (or cluster) left out",
        points: &[
            p(
                "Method",
                "The command runs on the data, then once for each observation of its estimation \
                 sample (each cluster with cluster()) left out. The replicates are kept as floats, \
                 as Stata keeps them; the variance is (n - 1)/n times the sum of products of the \
                 deviations, with t on n - 1 degrees of freedom. e(b_jk) is the bias-corrected \
                 value, n times the observed minus (n - 1) times the mean of the replicates.",
            ),
            p(
                "Forms",
                "jackknife exp_list: command with r() or e() values, or _b after regress, logit or \
                 probit; regress, logit and probit with vce(jackknife) are the same run. \
                 cluster(), level(), nodots and notable are translated; r(table) is left.",
            ),
            p(
                "Not yet",
                "Other options, a command with if or in that sets no e(sample), a missing value of \
                 the cluster variable, and a replication that stops with an error are refused with \
                 the reason.",
            ),
        ],
    },
    Guide {
        command: "statsby",
        about: "Collects a command's results for each group of the data",
        points: &[
            p(
                "Groups",
                "The command runs on each group's rows, in ascending order of the by() variables; \
                 a missing by value is left out unless by(vars, missing); total adds a last row. \
                 The data become the results, floats labelled with their expressions.",
            ),
            p(
                "Not yet",
                "Without clear, saving(), subsets and _b or _se after logit or probit (Stata names \
                 those variables after the equation) are refused with the reason.",
            ),
        ],
    },
    Guide {
        command: "rolling",
        about: "Collects a command's results over windows of a time series",
        points: &[
            p(
                "Windows",
                "window(#) consecutive observations of the data tsset by a time variable, \
                 stepsize(#), and recursive for windows that keep the first observation. start and \
                 end are the time variable's values; the data become the results.",
            ),
            p(
                "Not yet",
                "Gaps in time, xtset data, rrecursive, saving() and a missing clear are refused \
                 with the reason.",
            ),
        ],
    },
    Guide {
        command: "permute",
        about: "Repeats a command on random permutations of one variable",
        points: &[
            p(
                "Draws",
                "The permutations are Stata's own, drawn from the generator after seed(): the \
                 command runs once on the data as they are, then once for each replication, \
                 with the variable permuted among the estimation sample's rows (within each \
                 stratum with strata()). A variable with two values is permuted the way Stata \
                 does it, with fewer draws. The data are put back afterwards.",
            ),
            p(
                "Results",
                "The table, r() with its matrices (b, n, c_lower, p_twosided, ci_p_*, ...) and \
                 saving(file[, replace double]); e() is empty afterwards. A program the do-file \
                 defines runs once for each replication, so the R holds its lines that many times.",
            ),
            p(
                "Not yet",
                "spearman, r(nosuch), an e-class program, more than 2,000 replications, left, \
                 right and eps() are refused with the reason.",
            ),
        ],
    },
    Guide {
        command: "simulate",
        about: "Repeats a program or command and keeps its results as the data",
        points: &[
            p(
                "Method",
                "seed() is set, then the program or command runs reps() times on the data as the \
                 last run left them; the statistics (r() and e() values, or _b and _se) become \
                 the data, floats labelled with their expressions. r() is what the last run left.",
            ),
            p(
                "Not yet",
                "A replication that stops with an error (Stata records a missing row; R stops), \
                 saving(, double), and _b after a program that does not end with regress, logit or \
                 probit are refused with the reason.",
            ),
        ],
    },
    Guide {
        command: "bsample",
        about: "Replaces the data with a sample drawn with replacement",
        points: &[
            p(
                "Draws",
                "The rows are drawn with Stata's own generator after set seed, so the same seed \
                 gives the rows Stata's bsample gives, in its order, and the draws after it go on \
                 as Stata's do. R's sample() would draw other rows.",
            ),
            p(
                "strata()",
                "The data sorted by stratum, then Stata's draws with the bins taken within each \
                 stratum and a third uniform a row ordering them, so the rows come in Stata's \
                 order; with a size, the first that many of each stratum.",
            ),
            p(
                "Not yet",
                "weight(), strata() with cluster(), if or in, and if or in together with a \
                 sample size or cluster() are refused with the reason.",
            ),
        ],
    },
    Guide {
        command: "by",
        about: "by varlist: and bysort varlist: (the prefix)",
        points: &[
            p(
                "Groups",
                "Each group in turn, in sorted order with a missing value last, as in Stata; \
                 after a model the results kept are the last group's.",
            ),
            p(
                "Order",
                "Without bysort the rows must already be in order of the by variables: R \
                 checks the order itself and stops with Stata's not sorted.",
            ),
        ],
    },
    Guide {
        command: "capture",
        about: "Runs a line and goes on if it fails; _rc says whether it did",
        points: &[
            p("In R", "The line's R runs in tryCatch(): an error is kept from stopping the do-file, as in Stata."),
            p("_rc", "_rc is 0 when the line ran. When it failed because a name is not there, _rc is 111 as in Stata, and 110 for a variable already defined; for any other failure it is missing, since R's errors are not Stata's, so if _rc, if _rc == 0 and if _rc != 0 still pick what Stata would pick. When the line under capture was not translated, _rc is not known at all and the lines that read it say so."),
            p("capture noisily", "Shows Stata's message for the errors above (a not found, variable x already defined) and R's own message for any other."),
        ],
    },
    Guide {
        command: "cd",
        about: "Changes the folder the do-file works in",
        points: &[
            p("In R", "setwd(), with the folder as written: a relative one is taken from the folder the do-file is in then, as in Stata."),
            p("Paths", "A \\ in a path is read as /, which R takes on every system. A folder the do-file names that is somewhere else on this machine can be pointed to before the run, in the do-file window."),
            p("No folder", "cd alone, or cd \"\" (as a global that was never set leaves it), prints the folder and stays there, as Stata does on Windows. On macOS and Linux Stata goes to the home folder instead; RCraft stops there with a message rather than guess."),
        ],
    },
    Guide {
        command: "clear",
        about: "Drops the data in memory",
        points: &[p("In R", "The data frame is removed with rm(); nothing else differs.")],
    },
    Guide {
        command: "collapse",
        about: "Makes a dataset of summary statistics",
        points: &[
            p(
                "Precision",
                "Stata stores means and the like of float variables as floats; R keeps full \
                 precision, so the 8th digit can differ.",
            ),
            p(
                "Order and formats",
                "As Stata 19: the by() variables come first in the data's order (not by()'s), \
                 the rows sorted as by() names them, and each result keeps its source's display \
                 format.",
            ),
            p(
                "Weights",
                "aweights and fweights as Stata 19 uses them: the mean weighted; aweights scaled \
                 to each group's rows, so (sum) is the weighted mean times the rows with a value \
                 and (count) is those rows (with Stata's note); fweights as copies of the row, \
                 percentiles included; (sd) over the weights' sum less one; rawsum, min, max, \
                 first and last unweighted.",
            ),
            p(
                "Not yet",
                "pweights and iweights; percentiles with aweights; missing or zero weights (the \
                 R stops and says so).",
            ),
        ],
    },
    Guide {
        command: "compress",
        about: "Makes variables' storage types smaller",
        points: &[p(
            "Nothing to do",
            "R has no storage types (byte, int, float, str20), so compress changes nothing in R \
             and the values stay as they are. Stata lists each variable it makes smaller and the \
             bytes saved; RCraft prints nothing.",
        )],
    },
    Guide {
        command: "correlate",
        about: "Correlations and covariances",
        points: &[
            p(
                "Missing values",
                "A row with a missing value in any of the variables is dropped (casewise), as \
                 Stata does; R's cor() would stop or need use = \"complete.obs\".",
            ),
            p(
                "Weights",
                "Frequency weights count rows; analytic weights are scaled to sum to N; r() \
                 is Stata's (sum_w with analytic weights).",
            ),
            p(
                "As Stata 19",
                "Stata's matrix, checked character for character: the names cut to 8 over the \
                 columns and 12 beside the rows, %8.4f correlations or %8.0g covariances, seven \
                 columns to a block at a linesize of 80; means' table above it; (sum of wgt is \
                 ...) with analytic weights; no observations as Stata's r(2000).",
            ),
        ],
    },
    Guide {
        command: "count",
        about: "Counts the rows",
        points: &[p("Missing values", "With an if, a missing value counts as larger than every number, as in Stata.")],
    },
    Guide {
        command: "describe",
        about: "Describes the data",
        points: &[
            p(
                "Storage types",
                "R has no storage types (byte, int, float, str20), so the description lists R's \
                 classes instead, and r(width) and r(changed) are refused.",
            ),
            p(
                "Chosen variables",
                "describe varlist (names, ranges, patterns) lists only those, in the data's order; \
                 r(k) stays the number of variables in the data, as in Stata. simple prints the \
                 names in columns down the page, as Stata does; varlist leaves r(varlist). A name not in \
                 the data stops with Stata's r(111). describe using a file is not translated yet.",
            ),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "decode",
        about: "A value-labelled variable to its labels as text",
        points: &[
            p(
                "As Stata 19",
                "decode var, generate(newvar), with if and in: each value's label, \"\" for a \
                 value without one, a missing value or a row left out; the variable's current \
                 value labels (after a second label values too); Stata's errors for a string \
                 variable, one without labels and a variable that exists.",
            ),
            p("Not yet", "maxlength()."),
        ],
    },
    Guide {
        command: "destring",
        about: "Text to numbers",
        points: &[p(
            "Non-numbers",
            "A variable with text that is not a number is left as it is, with Stata's message \
             (contains nonnumeric characters), unless force or ignore() says otherwise.",
        )],
    },
    Guide {
        command: "do",
        about: "Runs another do-file (run quietly; include as if its lines were here)",
        points: &[
            p("Files", "The file is found from the folder the do-file is in then (after any cd), with .do added when it has no extension, as Stata does. RCraft reads it and runs its lines in turn; a file that is not there is said so and not run."),
            p("Macros", "do and run give the file its own locals, with its arguments in `0', `1', ...; include shares the locals of the do-file that includes it."),
        ],
    },
    Guide {
        command: "display",
        about: "Prints text and values",
        points: &[
            p(
                "Formats",
                "Stata's formats (%9.0g, %6.2f, %e, %s, with c, - and 0) are worked out as \
                 Stata does at every width: R's format() and sprintf() give other digits.",
            ),
            p(
                "Variables",
                "A variable in an expression means its first row, as in Stata.",
            ),
            p(
                "%21x",
                "The double's own bits in hexadecimal, as Stata writes them (+1.921fb54442d18X+001), worked out from the double in R; a missing value is Stata's . (+1.0000000000000X+3ff).",
            ),
            p("Colours", "as text, as result and as error are left out: R prints plain text."),
        ],
    },
    Guide {
        command: "drop",
        about: "Drops variables or rows",
        points: &[p("Missing values", "drop if x > 5 drops the rows where x is missing too, as Stata does.")],
    },
    Guide {
        command: "duplicates",
        about: "Reports, tags or drops repeated rows",
        points: &[LAYOUT_R],
    },
    Guide {
        command: "egen",
        about: "Generate with functions over rows or groups",
        points: &[p(
            "Missing values",
            "The functions leave missing values out of their sums and means, as Stata's do; \
             R's mean() would give NA without na.rm.",
        )],
    },
    Guide {
        command: "encode",
        about: "Text to a labelled number",
        points: &[p("Codes", "The codes follow the text's sorted order from 1, as Stata's.")],
    },
    Guide {
        command: "mata",
        about: "Stata's matrix language",
        points: &[p(
            "Not translated",
            "Mata is not translated: a mata block or a mata: line is refused with the reason.              The one exception is mata: mata clear, which outreg do-files use to start over:              it empties outreg's tables, the only thing RCraft keeps where Stata keeps Mata's              memory.",
        )],
    },
    Guide {
        command: "mkdir",
        about: "Makes a folder (rmdir removes an empty one)",
        points: &[
            p(
                "As Stata 19",
                "mkdir makes one folder whose parent is there. Anything that stops it (the folder \
                 or a file is already there, the parent is missing) is could not create directory, \
                 r(693): unlike the shell's mkdir -p, a nested path is not made in one go. No name \
                 is invalid syntax (r(198)), two names varlist not allowed (r(101)), and , public \
                 is accepted. rmdir removes an empty folder, and could not remove directory \
                 (r(693)) for one that is not empty, not there, or a file.",
            ),
        ],
    },
    Guide {
        command: "clonevar",
        about: "Copies a variable with its type, labels and display format",
        points: &[
            p(
                "As Stata 19",
                "clonevar newvar = oldvar [if] [in] keeps the storage type (a float keeps its \
                 rounding), display format, variable label and value label, and puts missing where \
                 if or in does not hold. It says (N missing values generated) with the count of \
                 missing values in the new variable, `.c` and \"\" included. The errors have \
                 Stata's messages and return codes: no = (r(198)), nothing after = (r(102)), more \
                 than one name or weights (r(101)), a new name that is taken (r(110)) and an old \
                 one that is not there (r(111)).",
            ),
            p(
                "In R",
                "The copy is a plain copy of the column with its attributes, so labelled columns \
                 stay labelled. Options are refused rather than guessed; by: stops with Stata's r(190).",
            ),
        ],
    },
    Guide {
        command: "char",
        about: "Sets, lists and reads characteristics of the data and of variables",
        points: &[
            p(
                "As Stata 19",
                "char var[name] text, char _dta[name] text and char list [var[]] behave as in Stata 19: the \
                 text loses blanks at both ends and a pair of quotes round all of it, no text removes the \
                 characteristic, names are case-sensitive, a variable name may be abbreviated and a bad name is \
                 invalid syntax (r(198)). `: char var[name]` and `var[name]` read them in macros.",
            ),
            p(
                "In R",
                "A characteristic is the attribute stata_char on the column (on the data frame for _dta). \
                 They go into a .dta file on save and come back on use. After merge, append, reshape, collapse and \
                 the like they are not known, and the commands say so. char list after tsset, xtset, stset, svyset \
                 or fvset is refused (Stata lists the characteristics those set). The order of a listing is the one \
                 set last first, not always Stata's. char rename and quotes inside the text are refused.",
            ),
        ],
    },
    Guide {
        command: "notes",
        about: "Adds, lists, drops, replaces and searches notes of the data and of variables",
        points: &[
            p(
                "As Stata 19",
                "note: text and note var: text add; notes, notes var, notes list and notes in # list; notes drop \
                 (_dta, _all or variables, in #, #/# or a list) says (N notes dropped); notes replace var in # : text, \
                 notes renumber and notes search word work as in Stata 19. The count is kept in note0 and each note in \
                 noteN, so the numbers stay as Stata leaves them after a drop. A variable that is not there is r(111).",
            ),
            p(
                "In R",
                "Notes are characteristics (see char). A note with the word TS (the date and time Stata puts there), \
                 quotes in the text and notes search with several words are refused.",
            ),
        ],
    },
    Guide {
        command: "labelbook",
        about: "Describes value label sets and their problems",
        points: &[
            p(
                "As Stata 19",
                "labelbook [names] [, problems length(#)] prints a page per set (range, gaps, string lengths, uniqueness, \
                 blanks, numeric labels, the definition and the variables that use it) or the table of problems, and \
                 leaves r(names), r(gaps), r(notused), r(numeric), r(blanks), r(nuniq) and r(nuniq_sh).",
            ),
            p(
                "In R",
                "It knows the sets the do-file defined. After a data file was read, or for a set with an empty label, it is \
                 refused: R does not keep the names of the sets that came with a file. The heading says [unnamed]; list() \
                 and the other options are refused.",
            ),
        ],
    },
    Guide {
        command: "compare",
        about: "Compares two variables value by value",
        points: &[
            p(
                "As Stata 19",
                "compare a b [if] [in] gives the counts of a<b, a=b and a>b, those defined in both with the minimum, \
                 average and maximum of the difference, those missing in one only and in both, and the total; r(N) is the \
                 number of rows. Strings compare in byte order. One variable is r(102), three r(103), a weight r(101).",
            ),
            p(
                "In R",
                "A number against a string is refused (R stops). Options are refused.",
            ),
        ],
    },
    Guide {
        command: "ds",
        about: "Lists variable names in columns",
        points: &[
            p(
                "As Stata 19",
                "ds [varlist] [, not has(...) not(...) alpha] prints the names in columns like describe, simple and \
                 leaves them in r(varlist). not alone leaves out the varlist; has() and not() choose by type string, \
                 type numeric, varlabel [pattern] or vallab.",
            ),
            p(
                "In R",
                "The storage types, format, char and vallab patterns, detail, varwidth() and skip() are refused: R keeps \
                 none of Stata's types or formats.",
            ),
        ],
    },
    Guide {
        command: "lookfor",
        about: "Finds variables by words in their name or label",
        points: &[
            p(
                "As Stata 19",
                "lookfor word ... finds the variables whose name or variable label holds any of the words, capitals \
                 ignored, a quoted phrase as one word, and leaves them in r(varlist). No words is r(198).",
            ),
            p(
                "In R",
                "Stata prints describe's rows; R has no storage types or display formats, so RCraft prints the name, \
                 R's class and the label.",
            ),
        ],
    },
    Guide {
        command: "misstable",
        about: "Tables the missing values of numeric variables",
        points: &[
            p(
                "As Stata 19",
                "misstable summarize [varlist] [if] [in] [, all] prints Obs=., Obs>. (.a to .z) and Obs<. with the \
                 unique values and the minimum and maximum, and leaves the last variable's r(N_lt_dot), r(N_eq_dot), \
                 r(N_gt_dot), r(K_uniq), r(min), r(max) and r(vartype).",
            ),
            p(
                "In R",
                "patterns, tree, nested and the options generate() and exok are refused.",
            ),
        ],
    },
    Guide {
        command: "expandcl",
        about: "Copies whole clusters of observations and numbers the copies",
        points: &[
            p(
                "As Stata 19",
                "expandcl exp, cluster(varlist) generate(newvar) makes exp copies of each selected \
                 observation (a missing, zero or negative count keeps the one, a fraction is rounded) \
                 and numbers the (cluster, copy) pairs in newvar: clusters in the sort order of \
                 their variables, missing last, a cluster with k copies taking k numbers in a row. \
                 It says how many counts were missing, noninteger, negative or zero, then (N \
                 observations created). The expression must be the same within a cluster (r(198)); \
                 both options are required (r(198)); a string count is r(109).",
            ),
            p(
                "In R",
                "Stata keeps the copies of one observation next to it in an order of its own, not \
                 stable; RCraft puts them in the order of their numbers, so code that depends on \
                 the order of the copies, rather than sorting first, can differ. by: stops with Stata's r(190).",
            ),
        ],
    },
    Guide {
        command: "fillin",
        about: "Adds a row for every combination of the values the data lack",
        points: &[
            p(
                "As Stata 19",
                "fillin varlist adds the combinations of the variables' values that the data lack, \
                 each variable's levels being the values it has (`.`, each of `.a` to `.z` and \"\" \
                 are levels of their own). The other variables are missing in the added rows, \
                 _fillin is 0 on the old rows and 1 on the new, and the data end sorted by the \
                 varlist with repeated combinations kept. Nothing is printed. The errors have \
                 Stata's return codes: one variable (r(102)), a name the data lack (r(111)), the \
                 same variable twice (r(5)) and _fillin already there (r(110)).",
            ),
            p(
                "In R",
                "Labels, display formats and value labels stay on the columns. Patterns in the \
                 varlist and data with no rows are refused; by: stops with Stata's r(190).",
            ),
        ],
    },
    Guide {
        command: "joinby",
        about: "Pairs every master row with the using rows of the same key",
        points: &[
            p(
                "As Stata 19",
                "joinby [varlist] using file pairs each master row with every using row that has \
                 the same key (many to many). unmatched(master|using|both) keeps the rows that found \
                 no partner and makes _merge (1 only in master data, 2 only in using data, 3 both); \
                 with no varlist it joins on the variables in common and says so. A variable in \
                 both files keeps the master's value; with update a master value that is missing \
                 or \"\" takes the using file's, and with update replace the using file's value \
                 replaces the master's when it is not missing (replace alone is r(198)). An empty \
                 result says (joinby on k is empty). \
                 The errors have Stata's return codes: no using (r(100)), a name the data lack \
                 (r(111)), no variable in common (r(198)), a key that is a string in one file and a \
                 number in the other (r(106)), _merge already there (r(110)).",
            ),
            p(
                "In R",
                "Stata puts the rows paired within one key in an order of its own, which changes \
                 from run to run; RCraft's are in the using file's order, so sort before looking. \
                 by: stops with Stata's r(190).",
            ),
        ],
    },
    Guide {
        command: "mvdecode",
        about: "Turns codes such as -99 into missing values",
        points: &[
            p(
                "As Stata 19",
                "mvdecode varlist, mv(numlist) makes the listed values . ; mv(numlist = .a \\ numlist = .b) \
                 makes others .a to .z. A value is matched in the variable's own type, so 1.1 matches a float 1.1. \
                 It says (name, right-aligned in 12) N missing values generated for each variable it changed and \
                 string variable ignored for a string. The errors have Stata's messages and return codes: no mv() \
                 (r(198)), . or text or a list such as 5 to 8 or 1(0)3 (r(121)), a code that is not a missing value (r(198)).",
            ),
            p(
                "In R",
                "Labels, display formats and storage stay. When the variable's type is not known (data read \
                 from a file) the value is matched as typed and as rounded to float. The numlist may be 1 2, \
                 5/7, 1(2)9, 1 3 to 9 or 1 3 : 9, as Stata reads them. by: stops with Stata's r(190).",
            ),
        ],
    },
    Guide {
        command: "mvencode",
        about: "Turns missing values into codes such as -99",
        points: &[
            p(
                "As Stata 19",
                "mvencode varlist, mv(#) turns every kind of missing value into #; mv(.a = #) only .a. It says N missing \
                 values recoded, and (recast to int) when the value does not fit the type. If a variable already holds # \
                 it says already # in N observations, then no action taken (r(9)) and changes no variable, unless \
                 override is given. When several clauses name the same missing value the first takes it, and only a \
                 clause that changed something can recast.",
            ),
            p(
                "In R",
                "A value that may not fit the type needs the storage type: known for variables the do-file made, and \
                 for a .dta file use read (its header is read when the R runs) while nothing since could have \
                 changed a type (sort, keep, drop, generate and the like are fine; replace, recode, merge and so \
                 on are not). Otherwise the line is refused.",
            ),
        ],
    },
    Guide {
        command: "snapshot",
        about: "Keeps copies of the data in memory and puts them back",
        points: &[
            p(
                "As Stata 19",
                "snapshot save [, label()] prints snapshot N (label) created at the time and \
                 leaves r(snapshot); the next number is the highest one there plus one (1 when none), \
                 so a number can come back after the highest is erased. list shows them all, \
                 _all or some, label # text changes a label (none clears it), restore # puts the \
                 data back (variables, values, labels) and erase takes _all or a numlist off. \
                 Long lines wrap at the line size, continued with two blanks. The errors have \
                 Stata's messages and return codes: no snapshots, a number outside 1 to the \
                 highest (r(198)), one that was erased (r(111), before anything is erased), a \
                 bad numlist (r(121), r(122)), and a bad number (r(7)).",
            ),
            p(
                "In R",
                "The snapshots are kept in the R session, as Stata keeps them outside the data, \
                 and survive clear. A snapshot of empty data restores empty data, but RCraft \
                 does not follow data that is empty, so the lines after it that need rows are \
                 refused. The time is the computer's, to the minute; a day before the 10th is \
                 written without a leading zero (not observed in Stata).",
            ),
        ],
    },
    Guide {
        command: "putdocx",
        about: "Builds a Word document in memory and saves it",
        points: &[
            p(
                "As Stata 19",
                "begin opens a document (a second one: document already open in memory, r(110)); \
                 every other subcommand without one, or after save, stops with document not \
                 opened (r(198)). paragraph [, style() halign()] starts a paragraph (styles \
                 Title, Subtitle and Heading1 to Heading9; halign left, center or right) and \
                 text (exp) [, bold italic underline linebreak nformat()] adds a run to it; a \
                 number without nformat() is shown in %11.0g, with nformat() in that format \
                 padded to its width; text with no paragraph, or after a table or pagebreak, is \
                 no active paragraph (r(198)). table name = (r, c) makes an empty table, \
                 name(i,j) = (exp) fills a cell, data(varlist) [if] [in] [, varnames obsno] \
                 writes each variable in its display format (value labels used, the original \
                 observation numbers with obsno), matrix(M) [, rownames colnames nformat()] \
                 the matrix (default format %12.0g), and etable the coefficient table the last \
                 regress printed, which Stata takes from r(): none after quietly regress or \
                 once a later command has replaced r() (information for the estimation table \
                 not found, r(111)). Every table is followed by an empty paragraph, a table \
                 name used again is redefined with a note and the old table stays. describe \
                 [name] prints the table's rows and columns (r(nrows), r(ncols)) or the number \
                 of tables and paragraphs (r() unchanged). save [, replace append] adds .docx \
                 to a name without an extension, another extension is r(198), an existing file \
                 without replace or append is r(198), append to one that is not there r(601); \
                 the document is closed after it. append puts an empty paragraph between the \
                 old body and the new one.",
            ),
            p(
                "In R",
                "The document is built in R and the Word file written from it (the package is \
                 zipped by R's zip package; officer's template has no Title or TableGrid style \
                 and is A4 with French style names). The file holds the same paragraphs, styles, \
                 alignment, runs, tables, column spans and page break as Stata's \
                 (crates/engine/tests/stata_putdoc.rs reads both back), but not its fonts, \
                 sizes or cell widths. Refused: image, sectionbreak, pagenumber, textblock, \
                 textfile, append of documents, collect, the options of begin, fonts, cell \
                 formats and table options other than nformat(), etable after a model other \
                 than regress, and a table with no observations.",
            ),
        ],
    },
    Guide {
        command: "putpdf",
        about: "Builds a PDF document in memory and saves it",
        points: &[
            p(
                "As Stata 19",
                "The same commands as putdocx (begin, clear, paragraph [, halign()], text, table \
                 name = (r, c), name(i,j), data(), matrix() and etable, describe, pagebreak, save) \
                 with the same messages and return codes; putpdf has no style() (option \
                 style(Title) not allowed), save has no append (option append not allowed), \
                 adds .pdf, says X.pdf already exists and you must specify the replace option, \
                 and a table with no rows or columns is accepted but leaves Stata's own document \
                 unusable.",
            ),
            p(
                "In R",
                "The PDF is drawn by R's pdf device: A4 pages (as the file Stata wrote here), 36 \
                 points of margin at the sides and 86 at the top, Helvetica 11, a grid per \
                 table and each run and cell in the order Stata puts it (the first line starts \
                 at Stata's left margin, the runs after it at the same points; the rows' and \
                 gaps' heights are measured from one file). Long paragraphs wrap at the \
                 margin. Refused: the same forms as putdocx, and a table with no rows or \
                 columns.",
            ),
        ],
    },
    Guide {
        command: "dyntext",
        about: "Runs a text with dynamic tags into a text file",
        points: &[
            p(
                "As Stata 19",
                "dyntext src, saving(file) [replace] prints the processed text and writes it. \
                 <<dd_do>> blocks run their commands and show each as a do-file's log does \
                 (. and the command, its output, a blank line), less what quietly, nocommands, \
                 nooutput and noprompt leave out; <<dd_display: [%fmt] exp>> is what display \
                 shows; dd_if, dd_else and dd_endif (and dd_skip_if, dd_skip_else, dd_skip_end) \
                 show or skip their lines, a tag on its own line goes with its line and the \
                 blank lines around stay; dd_ignore keeps its lines as they are, tags and all, \
                 and dd_include puts a file's lines in as they are; dd_remove's text is printed \
                 but taken out of the file. A source that is not there is r(601), a target that \
                 is there without replace r(602) with its full path, before any tag runs. On \
                 Windows the lines a dd_do command prints end in a carriage return in the file.",
            ),
            p(
                "In R",
                "The source is read when the line is reached, and its tags become lines of the \
                 run: the dd_do commands are translated as any other line, with what each \
                 prints kept for the document; the file is written only when every line of it \
                 was translated. The text and the file are the same as Stata's for the corpus \
                 file and the probes checked. Refused: a dd_if inside another (Stata does not \
                 nest them), dd_version other than 2, dd_do blocks with blank lines, comments, \
                 braces or lines that go on (///), dd_graph to anything but an .svg file, other \
                 tags, and dyntext under capture or quietly unless Stata stops before the tags \
                 run.",
            ),
        ],
    },
    Guide {
        command: "dyndoc",
        about: "Runs a text with dynamic tags into HTML through Markdown",
        points: &[
            p(
                "As Stata 19",
                "dyndoc src [, saving(file) replace] runs the tags as dyntext does (the file is \
                 src with .html without saving()), prints the text, turns it into HTML through \
                 Markdown (as markdown does) and says successfully converted with the file's \
                 full path. <<dd_graph>> exports the graph as SVG (Graph.svg, or Graph1.svg \
                 when that is there) and puts an <img> tag for it, with saving(), replace, \
                 alt(), height() and width().",
            ),
            p(
                "In R",
                "As dyntext. The graph is R's (ggplot2), written by R's svg device; the HTML is \
                 Stata's for the corpus file and the probes checked.",
            ),
        ],
    },
    Guide {
        command: "markdown",
        about: "Turns a Markdown file into HTML",
        points: &[
            p(
                "As Stata 19",
                "markdown src, saving(file) [replace] writes the HTML body only, one block a \
                 line: headings with an anchor named after their text (lower case, spaces as \
                 -, punctuation dropped, the same id twice for the same text), the \
                 typographer's quotes, dashes (--, ---), ellipses and << >> as entities, tight \
                 and loose lists (in a tight list a paragraph after a blank line keeps its \
                 <p>), indented and fenced code, quotes, rules, HTML blocks, pipe tables whose \
                 cells keep their spaces, code spans, emphasis, links, images, autolinks and \
                 backslash escapes; a soft line break is a space, two trailing spaces a <br />. \
                 A source that is not there is r(601), a target that is there without replace \
                 r(602).",
            ),
            p(
                "In R",
                "RCraft's own converter, written to Stata's rules: the HTML is byte for byte \
                 Stata's for the corpus file and the probes checked. Markdown that Stata's \
                 converter reads in ways not observed (reference links, footnotes, nested \
                 quotes) may differ.",
            ),
        ],
    },
    Guide {
        command: "outfile",
        about: "Writes the data as a text file in columns",
        points: &[
            p(
                "As Stata 19",
                "The free layout: each number right-aligned in its display format's width, each \
                 string quoted and left-aligned in at least 11 (a str20 in 22), a labelled value \
                 quoted and left-aligned in the format's width or its longest label, two blanks \
                 after each field; without wide a field that would bring the line to column 80 \
                 starts a new line and lines lose their trailing blanks. comma writes the fields \
                 trimmed with commas (a missing value empty), noquote and nolabel, runtogether \
                 for string variables only (r(109) otherwise), if, in, a variable list, .raw added \
                 to a name with no extension, and the errors for a file that exists, a missing \
                 variable and no using.",
            ),
            p(
                "String widths",
                "Stata lays a string out by its declared width. R does not keep it, so RCraft \
                 keeps the width a variable was given by input str#, generate str# or a file, and \
                 for a string made any other way takes the longest value; the layout then differs \
                 only when the declared width is above 9 and larger than the longest value.",
            ),
            p(
                "Not yet",
                "The dictionary option, and a variable list with ranges or wildcards.",
            ),
        ],
    },
    Guide {
        command: "outsheet",
        about: "Writes the data as a text file (tab or comma separated)",
        points: &[
            p(
                "As Stata 19",
                "Tab-separated, or comma or delimiter(char); a first line of names unless nonames; \
                 numbers written in their display formats (a %8.3f variable shows 1.500, a double \
                 %10.0g, a float %9.0g), value labels and strings in double quotes (a quote inside \
                 is not doubled) unless noquote, labels dropped by nolabel; a missing number is \
                 empty and .a to .z are written as they are; a file with no extension gets .out; \
                 if, in and a variable list. Nothing is printed when the file is written, \
                 (file x not found) only with replace and no file there, and the errors for a file \
                 that exists (r(602)), a missing variable, no using, and bad delimiter() or comma \
                 together with delimiter().",
            ),
            p(
                "Not yet",
                "A variable list with ranges or wildcards.",
            ),
        ],
    },
    Guide {
        command: "erase",
        about: "Deletes a file (rm is the same)",
        points: &[p(
            "Missing file",
            "A file that is not there stops with Stata's message, file ... not found, and \
             capture gives _rc 601, as in Stata. No extension is added: erase x looks for x, \
             not x.dta.",
        )],
    },
    Guide {
        command: "exit",
        about: "Ends the do-file",
        points: &[p("In R", "Nothing to do in R: the lines after it are not run, as in Stata.")],
    },
    Guide {
        command: "expand",
        about: "Copies rows",
        points: &[
            p("The new rows", "Each row's copies go after all the rows, in the rows' order, as Stata 19 puts them; a count is rounded, and one below 1 or missing leaves the row once."),
            p("Notes", "expand says what Stata 19 says: the noninteger, missing, negative and zero counts and the observations created."),
            p("generate()", "The new variable is 0 for the rows there before and 1 for the copies; Stata also gives 0 and 1 value labels, which RCraft does not."),
        ],
    },
    Guide {
        command: "confirm",
        about: "Checks that something is there, or is not",
        points: &[p(
            "As Stata 19",
            "[numeric|string] variable (r(111), r(7)), new variable (r(110)), [integer] number \
             (r(7)), file and new file (r(601), r(602)), each stopping with Stata's message and \
             return code, so capture leaves Stata's _rc. A variable may be abbreviated. confirm \
             file is declined after a line not translated that may have written files.",
        )],
    },
    Guide {
        command: "file",
        about: "Writes a text or binary file, and reads a text file line by line",
        points: &[p(
            "As Stata 19",
            "file open (write, with replace or append, text by default or binary), file write \
             (strings, _newline(n), _tab(n), _skip(n), _char(n), (exp) as display writes it, \
             %fmt (exp), and %1bu and %1bs bytes in binary files) and file close (or _all). \
             Text files take the system's new lines (\r\n on Windows), as Stata's do. The \
             errors stop where Stata's do (r(602), r(110), r(111)). seek, query and set are \
             not translated yet.",
        ),
        p(
            "file read",
            "file open h using f, read and then file read h name put the next line in the \
             local, without its end; r(status) says how the line ended (win, unix, mac, or none \
             for a last line without one) and r(eof) is 1, the local empty, past the last line, \
             so a while r(eof) == 0 loop goes through a file as in Stata. file open and close \
             leave r() as it was. Binary reads (formats) and read write together are not \
             translated yet.",
        )],
    },
    Guide {
        command: "format",
        about: "Sets how variables are shown",
        points: &[p(
            "In R",
            "The display format is kept as the variable's format.stata attribute, which haven \
             writes to a saved .dta file; the values do not change, and R's own printing does \
             not use it. Patterns and ranges of variables are not translated yet.",
        )],
    },
    Guide {
        command: "foreach",
        about: "Loops: foreach, forvalues and while (and continue)",
        points: &[
            p("How they run", "RCraft runs a loop itself and gives R each round's lines with the loop's macro replaced, so the R script has the loop written out round by round."),
            p("forvalues", "Each value is the one before plus the step, as Stata does, so 0(.1)1 ends at .9999999999999999, and each is kept as Stata keeps a number in a macro (16 digits)."),
            p("numlist", "foreach ... of numlist works its numbers out from the first and the step and keeps 15 digits, as Stata's numlists do."),
            p("varlist", "foreach ... of varlist with a wildcard, a range or _all asks R for the data's variable names when it runs and makes them Stata's way (x* every name that matches, x1-x3 the names between in the data's order, an ambiguous abbreviation Stata's error). Plain names are taken as written. A dry run cannot know the names, so it says the loop is known only when the do-file runs."),
            p("After the loop", "The loop's macro is gone after the loop, as in Stata. A while loop whose condition is still true after 100,000 rounds is stopped and said so."),
            p("Values only R knows", "A condition such as while r(N) > 0 is worked out by R when the do-file runs, after the lines before it have run."),
        ],
    },
    Guide {
        command: "estat",
        about: "Statistics after a model",
        points: &[
            p(
                "Information criteria",
                "estat ic counts the coefficients as the degrees of freedom, as Stata does; R's \
                 AIC() on a linear model also counts the variance.",
            ),
            p(
                "After regress",
                "vif (largest first unless the model has factor variables; uncentered with the \
                 constant's), imtest (Cameron and Trivedi's decomposition, with white; its \
                 statistics agree with Stata's to about 3e-6), and on tsset data bgodfrey and \
                 durbinalt (lags(), nomiss0) and dwatson, with the gaps in the sample counted: \
                 Stata 19's tables and r(), its matrices named lags:1 ... as Stata's. As Stata \
                 keeps them, r(vif_#) are floats.",
            ),
            p(
                "After logit and probit",
                "classification (cutoff(): the table, the rates and r(ctable)) after logit and \
                 probit; gof after logit, Pearson's over the covariate patterns or, with group(), \
                 Hosmer and Lemeshow's over the quantiles of the predicted probabilities (the mean \
                 of two neighbours where a quantile falls between them, as Stata's), with table.",
            ),
            p(
                "Not yet",
                "On panel data bgodfrey, durbinalt and dwatson are Stata's error (sample may not \
                 include multiple panels), and after regress, robust bgodfrey and durbinalt are \
                 too; small is refused. estat vce prints Stata 19's lower triangle (format(), \
                 correlation), and estat sd after mean the means and standard deviations; estat \
                 vce of factor variables or several equations keeps R's print.",
            ),
        ],
    },
    Guide {
        command: "ereturn",
        about: "Sets e() in a program that estimates, and lists or shows it",
        points: &[
            p(
                "What is listed",
                "Stata's scalars, macros and matrices in Stata's layout; what Stata also lists and \
                 RCraft does not keep (the optimizer's settings, e(ilog), ...) is named in a note.",
            ),
            p(
                "In R",
                "e() is a named list in R, stata_e. ereturn post b V makes it new from the \
                 do-file's matrices (which it moves, as Stata does), with depname(), obs(), \
                 dof(), properties() and esample(), whose variable leaves the data and marks \
                 e(sample). ereturn scalar, local and matrix add to it in the order ereturn \
                 list shows, an empty local removes one, and ereturn display prints Stata's \
                 coefficient table, with t when dof() was given and z otherwise.",
            ),
            p(
                "Stata's rules",
                "post and clear work anywhere; scalar, local and matrix only in an e-class \
                 program (r(152) elsewhere). A V whose size is not b's is r(503), and one whose \
                 names are not b's is r(507).",
            ),
            p(
                "Not yet",
                "ereturn repost, a variance matrix that is not symmetric, and predict, test, \
                 lincom and margins after posted results are refused.",
            ),
        ],
    },
    Guide {
        command: "estimates",
        about: "Keeps fitted models by name and brings them back",
        points: &[
            p(
                "Store and restore",
                "estimates store keeps the model with its e(), e(sample) and what predict, test \
                 and margins read; estimates restore brings it back, with Stata's note \
                 (results a are active now). drop and clear forget them, and a name that is not \
                 stored is Stata's error, r(111).",
            ),
            p(
                "_est_name",
                "Stata 19 keeps each stored model's e(sample) in the data, as the byte variable \
                 _est_name (labelled esample() from estimates store) at the end; so does the \
                 translation, and estimates drop and clear (and eststo drop and clear, eststo's \
                 own) take them out again. Where R does not hold e(sample) (after cmclogit, \
                 margins post), a line that reads _est_name is refused.",
            ),
            p(
                "dir, stats and replay",
                "Stata 19's tables: estimates dir (each stored model's command, dependent variable, \
                 number of parameters and title), estimates stats (N, ll(null), ll(model), df, AIC \
                 and BIC, with r(S)) of the active results or of the names given, and estimates \
                 replay of regress, logit, probit and poisson fits (robust and clustered too), \
                 under a banner as wide as the line (Model name, or active results): as Stata 19, \
                 the coefficients at 95% whatever or, irr or level() the fit had, and no \
                 iteration log. A model's name alone (regress, logit, probit, poisson, with \
                 level(), or and irr) shows its last fit again in the same way; after another \
                 command's fit it is Stata's error (last estimates not found).",
            ),
            p(
                "Not yet",
                "estimates replay of other models (or with weights), \
                 estimates save and use, and options are not translated. A name stored by a line \
                 that was not translated (eststo, a program) is not known, so a restore of it is \
                 refused rather than guessed.",
            ),
        ],
    },
    Guide {
        command: "fvset",
        about: "Chooses a factor variable's base level",
        points: &[p(
            "Base levels",
            "fvset base # (and default, and fvset clear) is kept by RCraft, and later models \
             write i.x and a bare x in an interaction as ib#.x, which is what Stata does. The \
             choice stays with the data as in Stata: save and use keep it, preserve and restore \
             bring it back, and import or clear drops it. fvset base first, last, frequent and \
             none, fvset design and fvset report are not translated yet.",
        )],
    },
    Guide {
        command: "generate",
        about: "Makes a new variable",
        points: &[
            p(
                "Float",
                "Without a type the new variable is a float, as in Stata: the translation rounds \
                 it to float precision (about 7 digits).",
            ),
            p(
                "Missing values",
                "A row the if leaves out is missing (.), or empty text for a string, as in Stata, \
                 and Stata's note says how many: (6 missing values generated).",
            ),
        ],
    },
    Guide {
        command: "glm",
        about: "Generalized linear models (maximum likelihood)",
        points: &[
            p(
                "In R",
                "stata_glm fits glm as Stata 19 does by default, matched to its output (its code \
                 was not read): one IRLS step from mu = (y + mean(y)) / 2 (binomial: n(y + 0.5) / \
                 (n + 1)), then Stata's Newton-Raphson on the observed information, which is \
                 the iteration log; OIM standard errors times the scale (1, or Pearson chi2 / df \
                 for gaussian and with scale(x2)), robust ones with N / (N - 1), clustered with \
                 G / (G - 1). The header (deviance, Pearson, AIC, BIC), the OIM table, \
                 offset()/exposure() rows, eform labels and notes, and e() are Stata 19's.",
            ),
            p(
                "Translated",
                "family(gaussian) with the identity link; poisson with log; binomial with logit, \
                 probit or cloglog (binomial n or #: logit); nbinomial # with log. fweights, \
                 pweights, iweights, aweights (gaussian), robust and clustered errors (without \
                 weights), eform, noconstant, scale(x2). predict: mu (the default), xb, eta.",
            ),
            p(
                "Not yet",
                "irls (Stata's IRLS stops by its own rule on the deviance), gamma and inverse \
                 Gaussian (Stata's default scale there is not Pearson / df exactly), gaussian with \
                 other links, other links and families, scale(dev) and numeric scales. \
                 Where the likelihood is very flat (a near-perfect probit), Stata's own steps end \
                 a little elsewhere: the last digits can differ.",
            ),
        ],
    },
    Guide {
        command: "graph",
        about: "graph bar, graph twoway, graph export",
        points: &[GRAPH],
    },
    Guide {
        command: "gsort",
        about: "Sorts, descending where asked",
        points: &[p(
            "Missing values",
            "Missing values sort last ascending and first descending, as in Stata; mfirst is \
             translated for numbers, and for text the empty string counts as missing.",
        )],
    },
    Guide {
        command: "histogram",
        about: "Histogram",
        points: &[
            p(
                "Bins",
                "Stata's: int(min(sqrt(N), 10 log10 N)) bins from the minimum, unless bin(), \
                 width() or discrete say otherwise. R's hist() uses Sturges' rule.",
            ),
            GRAPH,
        ],
    },
    Guide {
        command: "if",
        about: "if and else: blocks, and one command on the same line",
        points: &[
            p("How they run", "RCraft works the condition out (R does when it needs the data or a stored result, after the lines before it) and gives R only the lines that run, as Stata does: if exp { ... } else { ... }, and if exp command with else command on the next line."),
            p("On one line", "In if exp command, the condition runs as far as an expression can go, as in Stata: if _rc == 0 display \"ok\" runs display."),
        ],
    },
    Guide {
        command: "import",
        about: "import delimited and import excel",
        points: &[p(
            "Variable names",
            "Columns without names are v1, v2, ... as Stata names them; names that are not \
             valid Stata names are made valid by R's rules, which can differ from Stata's.",
        )],
    },
    Guide {
        command: "infile",
        about: "Reads raw text, free format",
        points: &[p(
            "As Stata",
            "Values separated by blanks or commas, a quoted string one value, flowing across \
             lines; variables typed byte, int, long, float (the default), double or str#; \
             ranges (x1-x3) and _skip(#); a string cut to its width; what is not a number is \
             missing with Stata's message, numbered among the observations if keeps; an \
             observation the file ends inside is kept, with \"(eof not at end of obs)\". A \
             file with no extension is .raw. Not yet: dictionaries, in, byvariable(), \
             automatic, value labels (v:lbl).",
        )],
    },
    Guide {
        command: "infix",
        about: "Reads raw text by columns",
        points: &[p(
            "As Stata",
            "Each field is its columns, blanks trimmed; str alone is as wide as its columns, \
             str# cut to #; # lines with #: for records of several lines; in counts records; \
             if keeps observations; Stata's messages for what is not a number and its count. \
             Not yet: dictionaries (infix using a .dct).",
        )],
    },
    Guide {
        command: "input",
        about: "Data typed into the do-file",
        points: &[p(
            "As Stata 19",
            "The lines up to end become the data, into empty data: float unless a type says \
             otherwise (double(a b) types a group), strings as written, . and .a missing. \
             Stata's echo of the lines is not printed. A value a numeric variable cannot \
             read, a line with too few or too many values, and input into data already in \
             memory are declined.",
        )],
    },
    Guide {
        command: "newey",
        about: "Regression with Newey-West standard errors",
        points: &[p(
            "As Stata 19",
            "Least squares, with V = n/(n - k) (X'X)^-1 S (X'X)^-1: the White sum plus the \
             Bartlett-weighted cross products of observations l time units apart (StatCraft's \
             decoding). Plain variable names only for now.",
        )],
    },
    Guide {
        command: "prais",
        about: "Prais-Winsten and Cochrane-Orcutt regression",
        points: &[p(
            "As Stata 19",
            "Least squares first, then rho from the residuals on the original data (or 1 - DW/2 \
             with rhotype(dw)) and a fit on the transformed data, until rho moves by less than \
             1e-6 (one round with twostep); corc drops the first observation; vce(robust) is \
             HC1 on the transformed data. Plain variable names and data without gaps only.",
        )],
    },
    Guide {
        command: "xtregar",
        about: "Fixed and random effects with AR(1) disturbances",
        points: &[
            p(
                "As Stata 19",
                "rho from Prais-Winsten on the within-demeaned data (each rhotype(), rhof(), \
                 twostep); fe on Baltagi and Wu's transform, kept in single precision as Stata \
                 keeps it, the constant divided by 1 - rho; re by GLS with each panel's theta; \
                 lbi's statistics (StatCraft's decoding of Stata 19).",
            ),
            p(
                "Differences",
                "re's sigma_u and what follows from it agree with Stata to about 1 part in 1e8, \
                 not to the last digit. Weights, factor variables, time-series operators, \
                 predict and margins after it are not translated yet.",
            ),
        ],
    },
    Guide {
        command: "ivprobit",
        about: "Probit with an endogenous regressor",
        points: &[
            p(
                "As Stata 19",
                "One endogenous regressor: the maximum-likelihood fit from Stata's exogenous \
                 probit and ml's Newton steps (both iteration logs), oim, robust and clustered \
                 variances, first, level(), Newey's two-step estimator, the exogeneity test, e() \
                 and predict's xb, pr and stdp (StatCraft's decoding of Stata 19).",
            ),
            p(
                "Differences",
                "The variance comes from differences of the analytic gradient, as StatCraft's \
                 does: it agrees with Stata's to about 1e-9. Weights, several endogenous \
                 variables, factor variables, twostep with first or vce(), iterate() below the \
                 default, and test, margins or estat afterwards are not translated. An instrument \
                 collinear with the other instruments is left out with Stata's note, as Stata \
                 does; any other collinearity stops the line when it runs, saying so.",
            ),
        ],
    },
    Guide {
        command: "spset",
        about: "Declare spatial data: the id variable and planar coordinates",
        points: &[
            p(
                "As Stata 19",
                "spset id, coord(x y) makes _ID (a copy of id, labelled Spatial-unit ID), _CX and \
                 _CY and prints Stata's block; spset alone shows it again and spset, modify coord() \
                 changes the coordinates. Stata's errors keep their return codes: data not spset, \
                 already spset, option modify required, an id that does not identify the rows, \
                 too few coordinate variables, no dataset in use.",
            ),
            p(
                "Differences",
                "Shapefiles, latitude and longitude (coordsys(latlong)), spset, clear and an id \
                 with missing values are stopped, saying so.",
            ),
        ],
    },
    Guide {
        command: "spmatrix",
        about: "Spatial weighting matrices",
        points: &[
            p(
                "As Stata 19",
                "create idistance with normalize(spectral, row, minmax or none), vtruncate(), if \
                 and in; dir; summarize and its r(); drop, copy and note; export and import in \
                 Stata's text form; fromdata. The weights agree with Stata's to the last digit or \
                 two, and the matrices are kept across clear, as in Stata. Stata's errors keep \
                 their return codes.",
            ),
            p(
                "Differences",
                "save and use write and read RCraft's own file: a .stswm file Stata saved is not \
                 read. Contiguity matrices (which need a shapefile), matafromsp and spfrommata \
                 (Mata), normalize and userdefined are not translated, and create stops on missing \
                 coordinates or two places at the same point.",
            ),
        ],
    },
    Guide {
        command: "spregress",
        about: "Spatial autoregressive models",
        points: &[p(
            "Not translated yet",
            "Stata 19 stops its likelihood iterations where its numerical derivatives lead, \
             before the maximum (by up to 1e-3 in rho on made-up data), and on some data it \
             reaches another optimum. Until that is decoded, spregress lines are marked as not \
             translated rather than given results that differ from Stata's.",
        )],
    },
    Guide {
        command: "stepwise",
        about: "Stepwise selection of terms for regress, logit, probit and poisson",
        points: &[
            p(
                "As Stata 19",
                "Backward, forward, hierarchical, lockterm1 and lr searches over variables and \
                 groups of variables: the trace, the final model with its terms in the order they \
                 were entered, the rows the full model uses, e(cmdline) and e(stepwise), and \
                 Stata's errors with their return codes.",
            ),
            p(
                "Differences",
                "Weights, factor variables, other options, a collinear full model, and a search \
                 that ends with no term are stopped when they run, saying so; so is a search with \
                 both pr() and pe() that could take the other kind of step, whose order is not \
                 decoded. The likelihood models agree with Stata to about 7 digits.",
            ),
        ],
    },
    Guide {
        command: "nestreg",
        about: "Nested regression: blocks of variables added in turn, for regress, logit, probit and poisson",
        points: &[
            p(
                "As Stata 19",
                "Each block's model with its own output, then the Wald table (F, R2 and the change \
                 in R2 for regress; Wald chi2 for the others), the likelihood-ratio table with lr or \
                 lrtable, or both with waldtable; r(wald) and r(lr), store(), if and in, level(), \
                 vce(robust) and vce(cluster) for regress, e() of the last model with its \
                 e(cmdline), and Stata's errors with their return codes.",
            ),
            p(
                "Differences",
                "Weights, factor variables, other models and options, and a variable listed twice \
                 (where Stata notes it and goes on) are not translated; a collinear model, or a \
                 number too wide for its column of the table, stops the line when it runs, saying \
                 so. The likelihood models agree with Stata to about 7 digits. The change in R2 \
                 of block 1 in r(wald) is a plain missing value, which `matrix list` shows as `.` \
                 where Stata shows `.z`.",
            ),
        ],
    },
    Guide {
        command: "npregress",
        about: "Nonparametric kernel regression",
        points: &[
            p(
                "As Stata 19",
                "npregress kernel with one continuous covariate: the local-linear or \
                 local-constant fit, the mean's bandwidth by leave-one-out cross-validation and \
                 the derivative's by Stata's local-cubic proxy, both found by Stata's Nelder-Mead \
                 search; every kernel; given bandwidths, iterate(), the identification rule, the \
                 table, e() and the two variables it leaves (StatCraft's decoding of Stata 19).",
            ),
            p(
                "Differences",
                "Stored bandwidths agree with Stata to about 1e-15, not always to the last bit. \
                 Several covariates, factor variables, imaic, vce(bootstrap), bandwidths given as \
                 a matrix, e(datasignature), and predict or margins afterwards are not \
                 translated.",
            ),
        ],
    },
    Guide {
        command: "xtdpdsys",
        about: "Dynamic panel-data GMM: xtabond, xtdpdsys and xtdpd",
        points: &[
            p(
                "As Stata 19",
                "The differenced equations with the lags of the outcome (and of pre() and \
                 endogenous() variables) as GMM-type instruments, one column per period, and the \
                 level equations with LD.y for xtdpdsys; instruments zero in every panel dropped; \
                 the one-step weight from the differencing, two steps from its residuals, \
                 vce(robust) and Windmeijer's two-step correction; estat sargan and estat abond, \
                 predict's xb, e and difference (StatCraft's decoding of Stata 19).",
            ),
            p(
                "Differences",
                "Stata's e(datasignature) checksum is not kept. Weights, factor variables, the \
                 display options, abbreviated variable names, diffvars() with a constant and \
                 estat abond after a one-step fit without a level equation are not translated, \
                 nor test, lincom or margins after these commands.",
            ),
        ],
    },
    Guide {
        command: "insheet",
        about: "Reads a tab or comma file",
        points: &[p(
            "As Stata 19 reads it",
            "A tab in the first line makes it tab, else comma. Names come from the first row \
             when no cell of it is a number (lowercase, what is not a letter, digit or _ left \
             out, the header kept as the label where it differs; case keeps the case), v1, v2, \
             ... when every cell is one; decimals are floats unless double. A first row of \
             both, headers Stata would have to make names up for, and an empty column stop \
             rather than guess.",
        )],
    },
    Guide {
        command: "ivregress",
        about: "Instrumental-variables regression",
        points: &[
            p(
                "Degrees of freedom",
                "Without small the variance divides by N, with z tests, as in Stata; small \
                 gives N - k and t tests.",
            ),
            p(
                "Clusters",
                "Clustered errors without small carry no G / (G - 1), as Stata's ivregress \
                 (unlike its regress).",
            ),
            p(
                "GMM",
                "The weight matrix from the 2SLS residuals, robust unless wmatrix() says \
                 otherwise, and the variance of the same kind unless vce() says, as Stata's.",
            ),
            MISSING_IN_MODELS,
            LAYOUT_R,
        ],
    },
    Guide {
        command: "kdensity",
        about: "Kernel density",
        points: &[
            p(
                "Kernel and bandwidth",
                "Stata's: the Epanechnikov kernel unless kernel() names another, the bandwidth \
                 0.9 min(sd, IQR / 1.349) N^(-1/5) unless bwidth(), at min(N, 50) points. R's \
                 density() uses a Gaussian kernel and another bandwidth.",
            ),
            GRAPH,
        ],
    },
    Guide {
        command: "keep",
        about: "Keeps variables or rows",
        points: &[p("Missing values", "keep if x > 5 keeps the rows where x is missing, as Stata does.")],
    },
    Guide {
        command: "label",
        about: "Variable and value labels",
        points: &[p(
            "Labels",
            "Kept as labelled vectors (the haven package), which other R functions mostly \
             ignore; a value label not defined earlier in the do-file is expected from the \
             data file.",
        )],
    },
    Guide {
        command: "local",
        about: "Macros: local, global, macro drop, tempvar, tempname, tempfile",
        points: &[
            p("How they work", "RCraft keeps the do-file's macros itself and replaces them in each line before the line is translated, as Stata does before it runs a line: nested `x`i'', `=exp', $name and ${name}, and the escapes \\` and \\$."),
            p("Numbers", "local x = exp keeps a number as Stata does, in %18.0g: 1/3 is .3333333333333333, 16 digits, not the full double."),
            p("Values only R knows", "A value from the data or a stored result (local m = r(mean), local n = _N) is worked out by R when the do-file runs, after the lines before it, and the lines after it use that value. The R script saved after a run holds the value it had then. A dry run says such lines are known only when the do-file runs."),
            p("Macro functions", "word count, word # of, the list functions (uniq, dups, sort, sizeof, retokenize, -, |, &, in, ==, ===, posof), length, strlen, subinstr and display are done as Stata 19 does them; variable label and label (var) # come from the data's labels in R, and label set # from the do-file's own label define. type, format and value label are not translated: R does not keep Stata's storage types, display formats or the names of value-label sets."),
            p("unab", "unab gets the data's variable names from R and expands the varlist as Stata does."),
            p("tokenize", "tokenize sets `1', `2', ... as Stata 19 does: every numbered local is cleared first, the outer quotes are taken off, quotes inside hold words together, and each parse() character is a word of its own."),
            p("gettoken", "gettoken takes the first token of a macro and what is left as Stata 19 does: leading blanks skipped, the rest keeping its delimiter (\" b c\"), quotes taken off, a parse() character a token of its own (<= and == together), and match() for a token in parentheses. The quotes, qed() and bind options are not translated yet."),
            p("Temporary names", "tempvar and tempname give __000000, __000001, ... as Stata does; tempfile gives a file in a folder of RCraft's own."),
        ],
    },
    Guide {
        command: "log",
        about: "Stata's log file (log and cmdlog)",
        points: &[p("In R", "Nothing to do in R: RCraft keeps what runs itself, in Results and in the do-file run's own log, so no Stata log file is written.")],
    },
    Guide {
        command: "levelsof",
        about: "Lists the distinct values of a variable",
        points: &[
            p(
                "The values",
                "Sorted, each number as Stata keeps it in a macro (%18.0g, so a float 0.1 is \
                 .1000000014901161), text in compound quotes unless clean, missing values only \
                 with missing; left in r(levels), with r(r) and r(N), as Stata 19 does.",
            ),
            p(
                "local()",
                "The macro is set from r(levels) once the line has run in R, so the lines after it \
                 (a foreach over it, most often) use the data's own values.",
            ),
        ],
    },
    Guide {
        command: "lincom",
        about: "A linear combination of coefficients",
        points: &[
            p(
                "Tests",
                "t on the model's degrees of freedom after regress, z after a likelihood model, as \
                 Stata's; with or, irr or rrr the estimate and its interval are exponentiated and \
                 the standard error is the delta method's.",
            ),
            p(
                "Layout",
                "Stata 19's line and table: ( 1) with the combination as test writes it (lincom x \
                 + w - 1 is x + w = 1), then the (1) row under the dependent variable (none after \
                 margins, post). A constant is part of the estimate; with or, irr, rrr or eform it \
                 is Stata's own error (r(198)). After robreg, reghdfe, cluster2, asclogit and suest \
                 the table is R's and a constant is refused.",
            ),
            p(
                "After xtregar",
                "t on e(df_r) after fe, z after re, under the dependent variable, with the \
                 constant in the estimate, as Stata 19 prints them; nlcom's table too.",
            ),
        ],
    },
    Guide {
        command: "list",
        about: "Lists rows",
        points: &[
            p(
                "As Stata 19",
                "Stata's table: each column as wide as its widest value or its name, cells \
                 right-aligned (left with a left-justified format), a line every five rows \
                 (separator(#)) or where a sepby() variable changes, the row numbers before the \
                 box; noobs, clean, nolabel, table and display as Stata's. A table wider than \
                 the line puts each row in a box of its own, as Stata does, and a line longer \
                 than the linesize breaks after > as Stata's. Numbers, dates and times in their \
                 display format (a set one, the one their storage type was made with, or else \
                 %9.0g for values a float holds, %10.0g for others and %12.0g for whole \
                 numbers), labelled values by their label.",
            ),
            p(
                "Names in full",
                "RCraft's choice: the names are written out in full, where Stata cuts them to 8 \
                 characters (a_very~e); abbreviate(#) cuts them as Stata does.",
            ),
            p(
                "In R's layout",
                "A variable R holds in a form of its own (a date class, a factor), a display \
                 format other than numbers, dates and times (%21x), or a variable wider than \
                 the line prints the rows in R's layout.",
            ),
        ],
    },
    Guide {
        command: "logit",
        about: "Logistic regression (logit and probit alike)",
        points: &[
            p(
                "Outcome",
                "The outcome is 1 where it is not 0 (and not missing), as Stata takes it; R's \
                 glm() needs 0 and 1.",
            ),
            p(
                "Perfect prediction",
                "A variable that predicts the outcome perfectly is dealt with as Stata does: \
                 omitted with the rows it decides (a column of 0 in e(b), where it was written), \
                 or the fit stops with Stata's message; predict after it is missing on those \
                 rows, in the sample or not, as Stata's default. R's glm() would go on with huge \
                 estimates.",
            ),
            p(
                "Collinear regressors",
                "The later of a collinear set, in the order written, is omitted with Stata's note \
                 and tolerance, as for poisson, nbreg, ologit, oprobit and mlogit (in every \
                 equation). R's glm() checks far more tightly, so a variable stored as float that \
                 is nearly another plus a constant stays in and the fit fails; polr() and \
                 multinom() do not leave a column out at all.",
            ),
            p(
                "Where probit stops",
                "Stata reports where its own Newton-Raphson stops, which can be short of the \
                 maximum by a few parts in 1e8. probit follows that path (from the constant-only \
                 fit, with Stata's steps and stopping rule) when the model has a constant and no \
                 offset, so its estimates are Stata's to the last digits there.",
            ),
            ML_SE,
            ML_TABLE,
            ML_ROBUST,
            ML_DIGITS,
            MISSING_IN_MODELS,
        ],
    },
    Guide {
        command: "lowess",
        about: "Lowess smoothing",
        points: &[
            p(
                "Smoother",
                "Stata's lowess: a local line with tricube weights over the nearest 80% of the \
                 data, with no robustness steps. R's lowess() adds three robustness steps.",
            ),
            GRAPH,
        ],
    },
    Guide {
        command: "qnorm",
        about: "Normal quantile plot",
        points: &[
            p(
                "Points",
                "The sorted values against the mean plus the standard deviation times the \
                 normal quantile of i/(N+1), with the line y = x, as Stata 19 plots them. R's \
                 qqnorm() uses other plotting positions. grid is not translated yet.",
            ),
            GRAPH,
        ],
    },
    Guide {
        command: "pnorm",
        about: "Standardized normal probability plot",
        points: &[
            p(
                "Points",
                "The normal probability of each standardized value against i/(N+1), with the \
                 line y = x and Stata's axes from 0 to 1, as Stata 19 plots them. grid is not \
                 translated yet.",
            ),
            GRAPH,
        ],
    },
    Guide {
        command: "rvfplot",
        about: "Residuals against fitted values",
        points: &[
            p(
                "Points",
                "After regress: the residuals against the fitted values of the estimation \
                 sample, titled Residuals and Fitted values as in Stata.",
            ),
            GRAPH,
        ],
    },
    Guide {
        command: "avplot",
        about: "Added-variable plot",
        points: &[
            p(
                "Points and note",
                "After regress: the residuals of the outcome on the other regressors against \
                 those of the one named, with the line of its coefficient and Stata's note of \
                 the coefficient, its standard error (robust when the model's is) and t.",
            ),
            GRAPH,
        ],
    },
    Guide {
        command: "screeplot",
        about: "Scree plot of eigenvalues",
        points: &[
            p(
                "Points",
                "After pca or factor: the eigenvalues against their number, titled as Stata's.",
            ),
            GRAPH,
        ],
    },
    Guide {
        command: "margins",
        about: "Margins and marginal effects",
        points: &[
            p(
                "Rows averaged",
                "Averaged over the estimation sample, Stata's e(sample); if the data changed \
                 since the model, the current values of its rows are used, as Stata does.",
            ),
            p(
                "Standard errors",
                "The delta method from the model's own variance, as Stata's.",
            ),
            p(
                "r() and at()",
                "r(table) (b se z or t pvalue ll ul df crit eform) and r(b) and r(V) with Stata's \n                 names (the first level of a term 1bn.: 1bn._at 2._at). at() over several \n                 variables is numbered as Stata 19 numbers it: the variables in the model's \n                 order, the last changing fastest. After xtreg the statistics are z, as in \n                 Stata.",
            ),
            p(
                "After clogit",
                "predict(pu0) (the default: invlogit(xb), the fixed effect taken as zero); \n                 vce(unconditional) from the rows' scores of the exact likelihood, clustered as \n                 the model was. ar. compares each level with the one before, r. with the first, \n                 term by term, with Stata's chi2 tests. mcomp(bonferroni) adjusts the printed \n                 p-values and intervals, not e(b) and e(V); post keeps the base levels in e(b) \n                 as zeros.",
            ),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "matrix",
        about: "Matrices: define, elements, names, list",
        points: &[
            p(
                "Columns",
                "e(b) and e(V) have Stata's columns (base levels as zeros, o. for omitted, the \
                 equations), printed in Stata's layout at a linesize of 80.",
            ),
            p(
                "The do-file's matrices",
                "Kept in R as stata_mat, with Stata's row and column names: r1, r2, ... and c1, \
                 c2, ... by default; a product takes the left one's rows and the right one's \
                 columns, a sum the right one's names, a transpose swaps them, and joins with , \
                 and \\ number default names again (read off Stata 19). A 1x1 matrix multiplies \
                 as a number.",
            ),
            p(
                "Functions",
                "The Kronecker product # (names eq:name, numbered across the result when the \
                 right one is typed in place), submatrices (A[1..2, 3...], A[\"a\"..\"b\", 1]), \
                 cholesky(), corr(), hadamard(), vec(), vech(), sweep(), get(_b), get(VCE), and \
                 invsym() of a singular matrix as Stata's generalized inverse (swept in order, \
                 a pivot at or below 1e-13 of its first value left out as 0). trace(), det(), \
                 diag0cnt(), issymmetric(), matmissing(), mreldif(), colnumb() and rownumb() \
                 give numbers, missing where Stata's are. matuniform() and the Kronecker \
                 product of matrices joined in place are declined.",
            ),
            p(
                "When a matrix is not known",
                "A matrix that a line not translated may have changed is declined from then on, \
                 never guessed; nullmat() of it as well.",
            ),
        ],
    },
    Guide {
        command: "mkmat",
        about: "Variables into a matrix",
        points: &[p(
            "Rows",
            "The rows kept (if, in; nomissing leaves out rows with a missing value), named r1, \
             r2, ...; without matrix() a column vector for each variable, as Stata.",
        )],
    },
    Guide {
        command: "merge",
        about: "Joins another file",
        points: &[
            p(
                "Pairing",
                "stata_merge pairs the rows as Stata does for 1:1, m:1, 1:m, m:m and 1:1 _n: \
                 within a key the k-th row of the master goes with the k-th row of the using \
                 data, and the shorter side's last row stands in for the rows it lacks (m:m is \
                 not every combination, as R's joins would give). A variable in both keeps the \
                 master's value.",
            ),
            p(
                "_merge",
                "_merge is made as Stata makes it (1 to 5, with Stata's labels), and Stata's \
                 report is printed. update fills missing values (4) and marks conflicts (5); \
                 replace takes the using value there. assert() is checked before keep(), and \
                 the merged data stay when it fails (r(9)).",
            ),
            p(
                "Order",
                "The master's rows come first, sorted by the key, then the rest, sorted by the \
                 key. Rows with the same key keep their order here; Stata's sort may put them \
                 in another order.",
            ),
            p(
                "Not yet",
                "The old syntax (merge varlist using) and force.",
            ),
        ],
    },
    Guide {
        command: "mlogit",
        about: "Multinomial logistic regression",
        points: &[
            p(
                "Base outcome",
                "The most frequent outcome, as in Stata, unless baseoutcome() says; R's \
                 multinom() takes the first level.",
            ),
            p(
                "Names",
                "Each coefficient is named outcome:variable; the base outcome's equation is \
                 zeros, as in Stata's e(b).",
            ),
            ML_EXACT,
            ML_TABLE,
            ML_ROBUST,
            ML_DIGITS,
            MISSING_IN_MODELS,
        ],
    },
    Guide {
        command: "nbreg",
        about: "Negative binomial regression",
        points: &[
            p(
                "Alpha",
                "alpha is 1 / theta of MASS::glm.nb; the standard errors come from the whole \
                 likelihood, alpha included, as Stata's (glm.nb's own leave theta out).",
            ),
            ML_TABLE,
            ML_ROBUST,
            ML_DIGITS,
            MISSING_IN_MODELS,
        ],
    },
    Guide {
        command: "nsplit",
        about: "Splits a number's digits into new variables (SSC)",
        points: &[
            p(
                "As Stata 19",
                "Each value's digits are cut from the left into parts of the lengths digits() \
                 gives (20060131 with 4 2 2 is 2006, 1 and 31), into var1, var2, ... or \
                 generate()'s names, labelled \"Part 1 of var\"; missing stays missing. Observed \
                 in Stata 19 on made-up data; the add-on's program was not read.",
            ),
            p(
                "Errors",
                "Every value needs as many digits as the pattern adds up to, else Stata's \
                 message (r(459)), after its note when the lengths vary; fractional values and \
                 new names in use stop with Stata's words.",
            ),
            p(
                "Not yet",
                "nsplit without digits(), with if or in, or of negative values.",
            ),
        ],
    },
    Guide {
        command: "ologit",
        about: "Ordered logistic regression (ologit and oprobit alike)",
        points: &[
            p(
                "Cut points",
                "R's zeta are Stata's /cut1, /cut2, ... with the same sign.",
            ),
            ML_EXACT,
            ML_TABLE,
            ML_ROBUST,
            ML_DIGITS,
            MISSING_IN_MODELS,
        ],
    },
    Guide {
        command: "order",
        about: "Orders the variables",
        points: &[p("In R", "The columns are moved as Stata moves them (first, last, before(), after(), alphabetic); the data do not change.")],
    },
    Guide {
        command: "postfile",
        about: "Posts results row by row to a .dta file (postfile, post, postclose)",
        points: &[p(
            "As Stata 19",
            "Each post keeps a row, each value in its variable's type as Stata stores it (float \
             rounded to single precision, a string cut to its width; float is the default); \
             postclose writes the file with haven. replace says \"(file name.dta not found)\" \
             as Stata does; an existing file without replace is r(602), a wrong count of \
             expressions r(198) and a string for a number r(109), each closing the postfile as \
             Stata does. every(#), which only sets how often Stata writes, changes nothing.",
        )],
    },
    Guide {
        command: "poisson",
        about: "Poisson regression",
        points: &[
            p(
                "exposure() and offset()",
                "ln(exposure) or the offset enters with coefficient 1, kept in e(offset) as Stata \
                 keeps it (as for nbreg); Stata's errors stop it as in Stata: an exposure not \
                 above 0, both options, a variable not there, two variables, a string.",
            ),
            ML_SE,
            ML_TABLE,
            ML_ROBUST,
            ML_DIGITS,
            MISSING_IN_MODELS,
        ],
    },
    Guide {
        command: "predict",
        about: "Predictions after a model",
        points: &[
            PREDICT_ROWS,
            p(
                "Defaults",
                "One new variable and no outcome() after mlogit or ologit is the first \
                 outcome's probability, as in Stata.",
            ),
        ],
    },
    Guide {
        command: "preserve",
        about: "Keeps a copy of the data, which restore brings back",
        points: &[
            p("In R", "preserve keeps a copy of the data frame (stata_preserved) and what RCraft knows of it: its variables' types, the value labels defined and xtset or stset; restore puts them back and drops the copy. restore, preserve keeps the copy and restore, not drops it unused."),
            p("At the end", "If the do-file ends before restore, Stata puts the preserved data back by itself; the R session keeps the data as it was at the end."),
        ],
    },
    Guide {
        command: "cumul",
        about: "Stores the empirical distribution function of a variable",
        points: &[
            p(
                "As Stata 19",
                "cumul x, generate(new) makes a float variable, labelled \"ECDF of x\", holding the \
                 running weight over the total (the running weight itself with freq), per by() \
                 group (a missing value is a group of its own). equal gives tied values the value \
                 after the last of them. Frequency and analytic weights are taken, with the \
                 running weight held in a float as Stata does, so the last digits agree. Rows \
                 outside if and in, with a missing value or with a zero or missing weight, are \
                 missing. A string variable makes a variable of missing values without an error, \
                 as in Stata. r() is left as it was.",
            ),
            p(
                "Ties",
                "Without equal, Stata gives tied values different values in an order of its own \
                 sort, which is arbitrary and cannot be reproduced. The R code gives them the \
                 order of the rows. The values themselves, and every row with no tie, are \
                 Stata's; which tied row gets which of the tied values may differ. Use equal \
                 where the order matters.",
            ),
            p(
                "Errors",
                "pweights and iweights r(101), a negative weight r(402), a non-whole fweight \
                 r(401), a missing generate() r(198), a name already there r(110), two \
                 variables r(103). The messages were read off Stata's log by eye.",
            ),
        ],
    },
    Guide {
        command: "inspect",
        about: "Describes the values of each variable: signs, integers, missing, a small histogram",
        points: &[
            p(
                "As Stata 19",
                "inspect prints, for each variable, the counts of negative, zero and positive values \
                 (and how many of each are integers), the missing values, a six-row histogram (a \
                 column for each value up to five, else five equal bins), the smallest and largest \
                 value, the number of distinct values (counted as floats, \"More than 99\" above 99) \
                 and, for a labelled variable, whether every value is in the label. The layout \
                 is Stata's to the column. r() holds the last variable's counts. A string variable \
                 counts as all missing.",
            ),
            p("Errors", "A name the data lack r(111), an unknown option r(101)."),
        ],
    },
    Guide {
        command: "dstdize",
        about: "Direct standardization of rates",
        points: &[
            p(
                "As Stata 19",
                "dstdize cases pop strata, by(group) takes the standard population as every row's \
                 population by stratum, and gives each group its adjusted rate (the sum of the \
                 standard share times the stratum rate), crude rate, normal interval, the tables \
                 and r() with the matrices adj, crude, se, lb_adj, ub_adj and Nobs and the groups' \
                 names. level() is taken.",
            ),
            p(
                "Precision",
                "Stata keeps the adjusted rate and its standard error as floats and sometimes \
                 lands one float step (about 7e-8 relative) from the rounded double result, in a \
                 way not found; r() agrees to about 1e-7.",
            ),
            p(
                "Not yet",
                "Several by() or strata variables, using(), base(), saving(), format(), print, nore \
                 and weights are refused.",
            ),
        ],
    },
    Guide {
        command: "pwmean",
        about: "Pairwise comparisons of means with equal variances",
        points: &[
            p(
                "As Stata 19",
                "pwmean y, over(vars) gives every group's mean and the pairs j vs i (i < j) with the \
                 pooled within-group variance on N - G degrees of freedom, unadjusted or adjusted \
                 by mcompare(bonferroni), sidak, scheffe, tukey, snk, duncan or dunnett; effects, \
                 pveffects, cieffects, cimeans, groups, sort and level() choose the tables. Several \
                 over() variables make the groups g#w, labelled as Stata labels them. A bare pwmean \
                 shows the last results again with other options, and groups with no or one \
                 residual degree of freedom print missing values, as Stata does. The printed tables \
                 are Stata's to the column. e() holds the means and contrasts with b, V, b_vs, V_vs \
                 and the error matrices; r() has table, table_vs, k_groups, level and the method.",
            ),
            p(
                "Precision",
                "Tukey's p-values and critical values come from R's ptukey, which differs from \
                 Stata's tukeyprob by about 1e-10; r(table_vs) and the Tukey columns agree to \
                 about 1e-7 (the SNK and Duncan critical values to about 3e-8 for long spans, which \
                 shows as 1e-6 in a limit near zero). Dunnett's critical value and p-values come \
                 from the exact integral, while Stata's dunnettprob is approximate: the critical \
                 value differs by up to about 6e-6 and a p-value by up to about 3e-4 in the \
                 relative, so a printed Dunnett interval can differ from Stata's in its fifth \
                 digit. Stata leaves rounding noise of about 1e-16 where e(V) is zero in a table of \
                 several over() variables, and two means that are equal to the last bit can be \
                 spanned in another order in SNK and Duncan.",
            ),
            p(
                "Not yet",
                "Weights (an error, as in Stata) and the order of the letters of groups when the \
                 means form overlapping sets: the sets of means that share a letter agree, but the \
                 letters can be dealt in another order, so r(groups1) and the rest are not kept.",
            ),
        ],
    },
    Guide {
        command: "bitest",
        about: "Exact binomial test of a 0/1 variable",
        points: &[
            p(
                "As Stata 19",
                "bitest var == #p counts the successes (frequency weights count rows several \
                 times) and prints the one-sided tails Pr(k >= observed) and Pr(k <= observed), \
                 and the two-sided probability: the tail on k's side plus the opposite tail from \
                 the first outcome, past the expected count, that is no more likely than k \
                 (equal probabilities count). When k is the expected count the two-sided p is 1; \
                 when no outcome on the other side is as unlikely the other tail is empty, \
                 r(k_opp) is missing and a note says so. r() holds k_opp, p, k, P_p, N, p_l and \
                 p_u. The probabilities are exact (pbinom and dbinom) and agree to 9 digits.",
            ),
            p(
                "Errors",
                "Not a 0/1 variable, or #p outside (0,1), r(450); weights other than fweights \
                 r(101); no rows r(2000). The messages were read off Stata's log by eye.",
            ),
            p("Not yet", "The detail option, by:, and a #p that is not a number written out."),
        ],
    },
    Guide {
        command: "bitesti",
        about: "Exact binomial test from the numbers of trials and successes",
        points: &[p(
            "As Stata 19",
            "bitesti #N #succ #p is bitest on the counts, with the same tails, k_opp and r(). \
             A non-whole #N is r(7), #succ above #N or #p outside (0,1) is r(450), a wrong \
             number of arguments r(198). The detail option is refused.",
        )],
    },
    Guide {
        command: "robvar",
        about: "Tests that the variances of groups are equal, robustly",
        points: &[
            p(
                "As Stata 19",
                "robvar x, by(g) prints the summary of x in each group, then Levene's W0 and Brown \
                 and Forsythe's W50 and W10: the one-way F of the absolute deviations from each \
                 group's mean, median and 10% trimmed mean (the integer part of 0.1 n values \
                 dropped from each end, as R's mean(trim = 0.1)), with df(k - 1, N - k) and the \
                 p value. r() holds N, w10, p_w10, w50, p_w50, w0, p_w0, df_1 and df_2; with one \
                 group or constant data they are missing, as in Stata. All agree to 9 digits.",
            ),
            p(
                "Errors",
                "Weights r(101), no by() r(198), two by variables r(103), a string variable \
                 r(109). A sample with no rows is an empty table in Stata; here R stops.",
            ),
        ],
    },
    Guide {
        command: "swilk",
        about: "Shapiro-Wilk test of normality",
        points: &[
            p(
                "What agrees",
                "W is R's shapiro.test (Royston's AS R94), which differs from Stata's by about \
                 5e-9 to 9e-9, so W agrees to 7 digits. V, z and the p value are worked out from W \
                 by Royston's normalising transformation (for 12 to 2000 values), which gives \
                 Stata's own V, z and p from Stata's own W to 16 digits; they carry W's \
                 difference, up to 4e-7 relative in the p value. The printed table agrees to its \
                 digits except where a value is within that rounding of a boundary.",
            ),
            p(
                "What does not",
                "With tied values Stata's W differs from Royston's by 0.003 or more, by a rule \
                 not decoded; for 3 to 11 values V comes from a rule not decoded either. Both \
                 stop with a message instead of printing numbers that would be off. So do more \
                 than 2000 values. A string variable or fewer than 3 values print a row of dots, \
                 as in Stata.",
            ),
        ],
    },
    Guide {
        command: "pcorr",
        about: "Partial and semipartial correlations of one variable with the others",
        points: &[
            p(
                "As Stata 19",
                "pcorr y x1 x2 ... fits y on all the others over the rows where every variable is \
                 present. With t a coefficient's t statistic and df the residual degrees of \
                 freedom, the partial correlation is t / sqrt(t^2 + df), the semipartial one is \
                 t sqrt((1 - R2) / df), and the significance is the coefficient's p value. The \
                 table, r(N), r(df), r(sp_corr) and r(p_corr) are Stata's. Frequency weights count \
                 N as their sum (r(N) and df follow), analytic weights keep it as the rows. A \
                 variable named twice counts once, so pcorr y x x is too few variables.",
            ),
            p(
                "Errors",
                "No variable r(100), fewer than three r(102), a name the data lack r(111), no \
                 rows or a string variable r(2000), pweights and iweights r(101).",
            ),
            p(
                "Not yet",
                "A regressor collinear with the others stops with a message: Stata drops the \
                 first of the collinear variables and prints (dropped) in its row, and what it \
                 then leaves in the two matrices is not known here. by: and patterns in the \
                 varlist are refused.",
            ),
        ],
    },
    Guide {
        command: "pctile",
        about: "Stores the cut points of equal groups in a new variable",
        points: &[
            p(
                "As Stata 19",
                "pctile newvar = exp, nquantiles(#) writes the # - 1 cut points to the first rows of \
                 newvar (a float unless a type comes first), genp() has their percentages, and \
                 r(r1), r(r2), ... hold them as doubles. A percentile is the first value whose \
                 running weight reaches W p / 100, or the average of it and the next value when \
                 the running weight is exactly that; without weights each row weighs 1 (R's \
                 quantile type 2). altdef is the interpolation at (n + 1) p / 100 (type 6) and \
                 may not be used with weights. nquantiles() runs from 2 to the number of rows of \
                 the data plus one, whatever the sample. Frequency, analytic and sampling weights \
                 are taken; the percentiles are the same as summarize, detail gives. Nothing is \
                 printed.",
            ),
            p(
                "Errors",
                "The return codes are Stata's: no observations 2000, a name already there 110, a \
                 string 109, nquantiles() out of range or not whole 198, iweights 101, a \
                 non-whole fweight 401. The messages were read off Stata's log by eye; the corpus \
                 compares the return codes and values, not the messages.",
            ),
            p(
                "In R",
                "genp()'s variable is labelled \"percentages for __000002\", which is what Stata 19 \
                 writes. by: and string types are refused.",
            ),
        ],
    },
    Guide {
        command: "pwd",
        about: "Prints the folder the do-file works in",
        points: &[p("In R", "getwd(), printed as a line.")],
    },
    Guide {
        command: "pwcorr",
        about: "Pairwise correlations",
        points: &[
            p(
                "Rows",
                "Each correlation has the rows where both of its variables are present; with \
                 listwise a row with any missing value is dropped.",
            ),
            p(
                "Significance",
                "The p-value from t on n - 2 degrees of freedom, and Bonferroni's and Sidak's \
                 adjustments over the number of pairs, as Stata's.",
            ),
            p(
                "Layout",
                "Stata 19's table: the names cut to 8 over the columns and 12 beside the rows, \
                 star() marks, print() blanks, sig's p-values and obs's counts below each row \
                 with a blank row after, seven columns to a block. print() with obs is refused \
                 (not observed).",
            ),
        ],
    },
    Guide {
        command: "program",
        about: "Defines a program the do-file then runs by name",
        points: &[
            p(
                "In R",
                "There is no R function for a program: RCraft runs its lines where it is called, \
                 each translated as any line is, with locals of its own (`0', `1' ... and args), \
                 temporary variables dropped when it returns, exit returning from it, and an \
                 r-class program's return scalar and return local as r() afterwards. \
                 marksample marks the rows its if and in keep, 0 where a variable of its \
                 varlist is missing (unless novarlist), as Stata's.",
            ),
            p(
                "Not yet",
                "sclass, byable programs and program list are refused; a program with a line \
                 that is not translated leaves r() unknown, and an e-class one e(). An e-class \
                 program's ereturn lines are on the ereturn page.",
            ),
        ],
    },
    Guide {
        command: "rangestat",
        about: "Statistics over a range of each row's key (SSC)",
        points: &[
            p(
                "As Stata 19",
                "For each row with a key, the rows whose key lies in [key + low, key + high] (. \
                 for no end), in its by() group and among the rows of if/in, itself included. A \
                 window with no rows gives missing; one whose values are all missing gives \
                 (sum) and (count) 0 and (obs) its rows. The new variable is var_stat or the \
                 name given, a double labelled \"sum of x\"; one in use stops (r(110)). Observed \
                 in Stata 19 on made-up data; the add-on's program was not read.",
            ),
            p(
                "Not yet",
                "Statistics other than sum, count, obs, mean, min, max and sd; bounds that are \
                 variables; excludeself and the other options.",
            ),
        ],
    },
    Guide {
        command: "recode",
        about: "Changes values by rules",
        points: &[p(
            "Rules",
            "The rules are taken in Stata's order, the first that matches a value deciding it; \
             missing values are kept unless a rule names them, as in Stata. Labels in the \
             rules (`1 \"low\"`) make a label set for each new variable, named after it or \
             label(), as Stata does; they need generate() or prefix(). The older syntax without \
             parentheses (recode x 1=10 2=20) is taken too. Stata's count follows for each \
             variable, always plural: (1 changes made to x), or (N differences between x and \
             new) with generate().",
        )],
    },
    Guide {
        command: "regress",
        about: "Linear regression",
        points: &[
            p(
                "Collinear regressors",
                "The smallest of a collinear set is omitted, whatever the order written, as Stata \
                 does (it takes the column with the most variation left at each step), with \
                 Stata's note and a 0 (omitted) row. R's lm() would drop the later one.",
            ),
            p(
                "Interactions",
                "g#h is i.g#i.h, as in Stata: in an interaction a bare variable is a factor. \
                 i.g#i.h without its main effects estimates every cell but the one where both \
                 are at their base, as Stata does (R alone would leave out another, giving \
                 other coefficients); i.g##i.h is R's own coding. The cells keep Stata's names \
                 in e(b) (1b.g#1.h, 2o.g#0b.h) and its table, value labels and empty cells \
                 included.",
            ),
            p(
                "Robust errors",
                "robust is HC1, as Stata's: the sandwich times N / (N - k). R's sandwich \
                 package defaults to HC3, which differs.",
            ),
            p(
                "Clustered errors",
                "Scaled by G / (G - 1) and (N - 1) / (N - k), with t tests and intervals on G - 1 \
                 degrees of freedom, as Stata's; a row dropped for a missing value is out of its \
                 cluster too.",
            ),
            p(
                "Weights",
                "Frequency weights count rows; analytic ones are scaled to sum to N; sampling \
                 weights mean robust errors; with importance weights N is their sum rounded \
                 down, as Stata's e(N).",
            ),
            p(
                "Factor variables",
                "i.g leaves out the lowest level, as Stata's base; without the constant Stata \
                 still leaves it out while R would keep every level, so noconstant with factor \
                 variables is refused.",
            ),
            p(
                "Printing",
                "Stata's own table (ANOVA or robust header, coefficients with t, P and the \
                 interval), checked character for character; a shape not checked keeps R's \
                 layout and says so.",
            ),
            MISSING_IN_MODELS,
        ],
    },
    Guide {
        command: "rename",
        about: "Renames variables",
        points: &[p(
            "Options",
            "Renames as Stata does, lower and upper included; rename's other options are not \
             translated yet.",
        )],
    },
    Guide {
        command: "replace",
        about: "Changes the values of a variable",
        points: &[
            p(
                "Storage type",
                "replace keeps a float a float in Stata; R keeps full precision unless the \
             do-file made the variable (also when it saved the data and read it back with use \
             or merge), with a note.",
            ),
            p(
                "Message",
                "Stata 19's count of what changed: (7 real changes made), (1 real change made, 1 \
                 to missing) for numbers (a . made .a is a change), counted group by group under \
                 by; nothing under quietly or capture.",
            ),
        ],
    },
    Guide {
        command: "reshape",
        about: "Long to wide and back",
        points: &[p(
            "Order",
            "Long to wide and back with i() and j() as Stata takes them; the rows come out \
             sorted by i and j, as Stata leaves them. reshape long without j() names the new \
             variable _j, as Stata does. Stata's summary table (wide -> long) is not printed.",
        )],
    },
    Guide {
        command: "save",
        about: "Writes a .dta file",
        points: &[p("File", "Written by haven::write_dta, which Stata reads; value labels and variable labels are kept.")],
    },
    Guide {
        command: "scalar",
        about: "Scalars",
        points: &[p(
            "Names",
            "A variable comes before a scalar of the same name, as in Stata. R cannot see a \
             file's columns before it runs, so a command that reads a scalar by name checks \
             the data first and stops if a variable has that name; scalar(name) is always the \
             scalar.",
        )],
    },
    Guide {
        command: "set",
        about: "Settings",
        points: &[
            p(
                "set obs",
                "With no data, a data frame of that many rows and no variables; with data, new rows at the end, missing (\"\" in a string variable). A number written out is translated; set obs does not drop rows.",
            ),
            p(
                "set seed",
                "stata_set_seed() starts Stata's own generator, so runiform(), rnormal() and \
                 rgamma() then give Stata's draws. Its state is kept in R and goes on across \
                 lines and typed commands; a line that draws and is not translated stops the \
                 draws after it until the next set seed.",
            ),
            p(
                "Other settings",
                "Those that only change how Stata runs or prints (more, linesize, ...) have \
                 nothing to do in R.",
            ),
        ],
    },
    Guide {
        command: "sort",
        about: "Sorts the rows",
        points: &[p(
            "Ties",
            "Stata's sort puts rows that tie in no set order unless stable; R keeps them in \
             their current order. Missing values sort last, as in Stata.",
        )],
    },
    Guide {
        command: "split",
        about: "Cuts a string variable into pieces",
        points: &[p(
            "As Stata 19",
            "Without parse(), into words (runs of spaces as one); with parse(), at each string \
             it gives (\" \" too) after trimming the ends (notrim keeps them), the earliest one \
             winning, empty pieces kept and none after a last delimiter; limit() drops the \
             pieces past it; new variables stub1, stub2, ... as text, r(k_new) and r(varlist). \
             Parse strings that share characters and destring are declined.",
        )],
    },
    Guide {
        command: "ssc",
        about: "ssc install: Stata's add-ons",
        points: &[p(
            "Add-ons",
            "Stata's add-ons (ado programs) are out of scope: their commands are not \
             translated. ssc install name offers the R package of the same name instead, \
             through rc install, when CRAN has one, and says so when it has none; the R \
             package is R's, not a translation of the add-on. net install is refused.",
        )],
    },
    Guide {
        command: "streg",
        about: "Parametric survival regression after stset",
        points: &[
            p(
                "Method",
                "The exponential, Weibull, lognormal, loglogistic and Gompertz distributions, fitted \
                 by maximum likelihood with Newton's method and exact derivatives to its limit. The \
                 log likelihood is that of log time (as Stata reports it); late entry (time0(), \
                 enter()) takes the log survivor function at the entry time off. Stata stops at its \
                 own tolerance, so its last digits can differ unless tolerance() is set tight.",
            ),
            p(
                "Metrics",
                "Hazard ratios by default for the proportional-hazards models (nohr gives the \
                 coefficients), time (and tr, the time ratios) for the accelerated-failure-time \
                 metric of the exponential and Weibull; the ancillary parameter is /ln_p, \
                 /lnsigma, /lngamma or /gamma, with p and 1/p, sigma or gamma below.",
            ),
            p(
                "Variances",
                "robust and vce(cluster v) scale the sandwich by N / (N - 1) or G / (G - 1); with \
                 id() the clusters are the subjects. e() holds what Stata's does (b, V, ll, ll_0, \
                 chi2, N_sub, N_fail, risk ...).",
            ),
            p(
                "Not yet",
                "Other distributions (generalized gamma), factor variables, strata(), ancillary(), \
                 frailty(), offset(), vce(opg), and predict, test, lincom, margins and estat after \
                 it are refused with the reason.",
            ),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "sts",
        about: "Kaplan-Meier lists, new variables and tests of equal survival",
        points: &[
            p(
                "Forms",
                "sts list (survivor, failure or na, by(), at(), level()), sts generate (s, se(s), \
                 lb(s), ub(s), na, se(na), lb(na), ub(na), h, n, d, by()) and sts test (logrank, \
                 wilcoxon, tware, peto, fh(p q), strata()); r(chi2) and r(df) after sts test.",
            ),
            p(
                "Not yet",
                "sts graph, the cox and trend tests, adjustfor(), enter and compare, and any of \
                 these with late entry are refused with the reason.",
            ),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "stdescribe",
        about: "Describes survival-time data per subject",
        points: &[
            p(
                "Method",
                "Records, entry and exit times, gaps, time at risk and failures per subject, with \
                 their mean, minimum, median and maximum; all of r() is kept.",
            ),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "stsum",
        about: "Summarizes survival-time data: time at risk, rate, survival times",
        points: &[
            p(
                "Method",
                "Time at risk, the incidence rate, the subjects and the 25%, 50% and 75% survival \
                 times (the first time the Kaplan-Meier estimate falls to the level), with by(); \
                 r() holds the total's.",
            ),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "stptime",
        about: "Person-time and the incidence rate of survival-time data",
        points: &[
            p(
                "Method",
                "Needs an id() set by stset. Person-time, failures and the rate with the interval \
                 rate * exp(+-z / sqrt(failures)), by(), at() (pieces (a, b] and a last open one \
                 unless trim), per(), dd() and level(); r(ptime), r(failures), r(rate), r(lb) and \
                 r(ub) hold the total.",
            ),
            p("Not yet", "smr(), using(), jackknife, cox, weibull and the other models."),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "stir",
        about: "Incidence-rate ratios from survival-time data",
        points: &[
            p(
                "Method",
                "The comparison ir makes, with failures and time at risk by exposure (the larger \
                 value of a two-valued variable is exposed), and strata() for the stratified table; \
                 r() is ir's.",
            ),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "ltable",
        about: "Life tables for individual-level or aggregate data",
        points: &[
            p(
                "Method",
                "Actuarial intervals [kw, (k + 1)w): those lost count for half the interval, the \
                 survivor function is the product of 1 - d / n', Greenwood's variance gives the \
                 standard error, the interval is on the log-log scale. failure, hazard, noadjust, \
                 level(), intervals(), by(), fweight and test (the likelihood-ratio statistic of \
                 the exponential model, then the log-rank test; r(chi2) and r(df) are the log-rank's).",
            ),
            p(
                "Not yet",
                "graph, overlay, tvid() and saving() are refused with the reason.",
            ),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "stcox",
        about: "Cox regression",
        points: &[
            p(
                "Ties",
                "Breslow's method by default, as Stata's; R's coxph() defaults to Efron's. \
                 efron and exactp are translated as asked.",
            ),
            p(
                "Robust errors",
                "Robust errors carry N / (N - 1) and clustered ones G / (G - 1), as Stata's.",
            ),
            LAYOUT_R,
        ],
    },
    Guide {
        command: "stset",
        about: "Declares survival data",
        points: &[p(
            "Variables",
            "_st, _d, _t and _t0 are made as Stata makes them; a row that enters at or after \
             its end is not used.",
        )],
    },
    Guide {
        command: "svy",
        about: "Survey design and estimation (svyset, svydescribe, svy:)",
        points: &[
            p(
                "As Stata 19",
                "svyset with strata(), fpc(), singleunit(), later stages (a stage with no fpc \
                 drops those after it, with Stata's note), pweight or iweight, and the r() it \
                 leaves; svydescribe: the first stage's table with and without a varlist, \
                 finalstage for one stage, if and in, and r(). svy: clogit as before (the weighted \
                 estimates on Stata's Newton-Raphson path, the variance linearized over the PSUs, \
                 the adjusted Wald F, predict, margins with vce(unconditional)). svy: mean, total, \
                 ratio and proportion under strata, fpc as a count or a rate, the four singleunit() \
                 rules, subpop() as a variable or an if, level(), over() of one variable for mean \
                 and total: the estimates, the stratified linearized variance (PSU totals about \
                 their stratum's mean, n/(n - 1), 1 - f), design df = PSUs - strata, the header \
                 with the design and the counts, the notes under the table, and e() as Stata's.",
            ),
            p(
                "Not yet",
                "svy: regress, logit, probit and poisson; designs with later stages under svy:; \
                 over() with several variables, or with ratio, proportion or subpop(); svyset's \
                 replicate-weight and other variance methods and poststrata; svydescribe for later \
                 stages, single and generate(); a stratum sampled completely; e(V_srs) and the \
                 other SRS matrices.",
            ),
        ],
    },
    Guide {
        command: "survwgt",
        about: "Raking a weight to control totals (survwgt rake, SSC)",
        points: &[
            p(
                "As Stata 19",
                "survwgt rake w, by() totvars() generate() [maxrep()]: each pass adjusts the \
                 dimensions in order to their totals; it stops after the first pass in which no \
                 category was 0.001 or more from its total (maxrep() passes at most, 10 by \
                 default, with Stata's warning when they are reached). The new variable is a \
                 double labelled (raked); a missing weight stays missing, a missing by() value is \
                 a category of its own; the add-on's errors for totals not constant in a category \
                 and for dimensions whose totals differ. The data are left sorted by the last \
                 dimension; rows that tie keep their order in R, which Stata does not promise.",
            ),
            p(
                "Not yet",
                "With several dimensions and totals in the thousands Stata can stop a pass or more \
                 sooner, by a rule not pinned down yet: R stops with a message there rather than \
                 guess (one dimension is exact either way). The add-on's other subcommands, if, in, \
                 and the values it leaves in r().",
            ),
        ],
    },
    Guide {
        command: "center",
        about: "Centers or standardizes variables (center, SSC)",
        points: &[
            p(
                "As Stata 19",
                "Each variable less its mean (over if, in and, with casewise, the rows with none of \
                 them missing; other rows missing), divided by its standard deviation with \
                 standardize; aweights and fweights as summarize takes them. Without double the \
                 mean and standard deviation are rounded to float first, as center does: new \
                 variables c_x (or prefix()) are float, inplace keeps a double double and makes \
                 any other type float. The labels \"(centered)\" and \"(standardized)\", nolabel, \
                 replace and the add-on's messages and errors. Observed on made-up data; the \
                 add-on's program was not read.",
            ),
            p(
                "Not yet",
                "pweights and iweights, generate(), meansave() and sdsave(). With inplace, a \
                 variable whose storage type RCraft does not know (one read from a file) is \
                 taken as double when a value cannot be a float and otherwise as float, Stata's \
                 default, with a note.",
            ),
        ],
    },
    Guide {
        command: "fdasave",
        about: "Writes the data as a SAS transport file (fdause reads one)",
        points: &[
            p(
                "As Stata 19",
                "fdasave file [if] [in] [, rename replace vallabfile(xpf|sascode|both|none)] writes \
                 a SAS XPORT version 5 file, byte for byte what Stata writes bar the time stamps: \
                 names in capitals cut to 8 characters (a taken one ends in 2, 3, ...; without \
                 rename the list of changes is shown and it stops, r(110)), labels cut to 40, \
                 numbers as IBM floats, strings at their width, BEST 12 or MMDDYY 10 for dates, \
                 and the value labels in formats.xpf (or as SAS code, or both) beside the file. \
                 With replace the old file goes first, before the checks that can stop it (not \
                 no variables, r(111)): no observations (r(2000)), a number of 9.046e+74 or more \
                 or a string over 200 bytes (r(459)), a value label that is not defined (r(111)), \
                 and the file that exists without replace (r(602)). fdause file [, clear \
                 novallabels] reads one back: names in lower case, numbers as doubles, dates, the \
                 labels of formats.xpf (a warning when it is not there), and r(601) for a missing \
                 file.",
            ),
            p(
                "In R",
                "A string is written at its declared width where the variable was made by input \
                 str#, generate str# or fdause, else at its longest value. The name of a value \
                 label is kept on the column by label values and fdause; a labelled variable \
                 without one (read from a .dta) is refused. export sasxport5, import sasxport5 \
                 and the member() option of fdause are not translated.",
            ),
        ],
    },
    Guide {
        command: "filefilter",
        about: "Replaces bytes in a file",
        points: &[
            p(
                "As Stata 19",
                "Every occurrence of from() becomes to(), left to right and not overlapping, the \
                 rest copied byte for byte; the codes \\BS \\Q \\LQ \\RQ \\$ \\t \\n \\r \\W \\U \\M and \
                 \\###d; r(occurrences), r(bytes_from), r(bytes_to); replace, and the errors for \
                 a missing file, a file that exists, the same file and an unknown code.",
            ),
            p("Not yet", "Hexadecimal codes (\\##h)."),
        ],
    },
    Guide {
        command: "putexcel",
        about: "Writes values into the cells of an Excel workbook",
        points: &[
            p(
                "As Stata 19",
                "putexcel set file.xlsx with sheet() and modify or replace (or neither, for a \
                 new file); cell = \"text\", a number or ( expression ), several to a line; \
                 putexcel clear. Text cells, numbers to 17 digits and . an empty cell, the other \
                 sheets and cells kept, file … saved after each write, and Stata's errors (a file \
                 that exists, no set, a column past XFD). Written with the R package openxlsx2.",
            ),
            p(
                "matrix()",
                "cell = matrix(M) writes the values from the cell down and right; names puts the \
                 column names on the first row and the row names in the first column (the corner \
                 left as it was), rownames and colnames one of them. The names are Stata's full \
                 ones (1bn._at, 1b.g). putexcel leaves r() as it was, as in Stata 19.",
            ),
            p(
                "Not yet",
                "Ranges, matrices with equation names, etable, formula(), pictures, the formatting options (bold, \
                 nformat() and others), sheet(, replace), .xls files, and putexcel save, close \
                 and describe.",
            ),
        ],
    },
    Guide {
        command: "outreg",
        about: "Tables of estimates and statistics (outreg, SSC)",
        points: &[
            p(
                "As Stata 19",
                "The balance-table use: after a model, keep() rtitle() stats(b) noautosumm \
                 starlevels() starloc(1) store(), the coefficient with three decimals, thousands \
                 commas and stars; replay() with append() or merge(), ctitles() (rows split by \
                 \\), note(\"\"), store(); and using a file with tex fragment plain nocenter, \
                 multicol() and replace, the .tex byte for byte as outreg writes it (CR LF). \
                 Observed on made-up data; the package's programs were not read.",
            ),
            p(
                "Not yet",
                "Every other layout: standard errors or t statistics under the coefficients, \
                 summary statistics, outreg's own notes, Word and RTF files, a table without \
                 fragment. On screen the table is printed in R's layout.",
            ),
        ],
    },
    Guide {
        command: "frmttable",
        about: "A table from a matrix (frmttable, part of outreg)",
        points: &[
            p(
                "As Stata 19",
                "statmat() store() with sfmt() of g and f and sdec(): f with sdec() decimals \
                 (2), g a whole number, a missing cell empty, no row titles; for outreg's \
                 replay() and merge().",
            ),
            p("Not yet", "g of a number with decimals, row and column titles, and writing a file."),
        ],
    },
    Guide {
        command: "alpha",
        about: "Cronbach's alpha and the scale of the items",
        points: &[
            p(
                "As Stata 19",
                "generate(): in each row the mean of the items present (min(), casewise), an item \
                 with a negative loading on the first principal factor reversed unless asis (as \
                 Stata chose in 200 of 200 data sets), std standardizing each item first; summed \
                 in float as Stata does, so the scale is Stata's to the bit. With casewise, \
                 r(alpha), r(cov) and r(rho) as Stata's.",
            ),
            p(
                "Not yet",
                "r(alpha) and r(cov) without casewise (Stata pairs the data in a way not pinned \
                 down yet); item, label, detail, reverse(), weights. Printed in R's layout.",
            ),
        ],
    },
    Guide {
        command: "winsor",
        about: "Winsorizes a variable (winsor, SSC)",
        points: &[
            p(
                "As Stata 19",
                "generate() with p() or h(): over the nonmissing values of if and in, the h lowest \
                 become the (h+1)th smallest and the h highest the (h+1)th largest (high or low \
                 for one end); h is h() or floor(p() N). The new variable keeps the old one's \
                 type and format, is missing outside if and in, and is labelled as the add-on \
                 labels it; its errors. Observed on made-up data; the add-on's program was not \
                 read.",
            ),
            p("Not yet", "String variables (which the add-on takes)."),
        ],
    },
    Guide {
        command: "query",
        about: "Shows Stata's settings",
        points: &[
            p(
                "As Stata 19",
                "Nothing to do in R: query only shows Stata's own settings (graphics, memory, \
                 output and the rest), which R does not have.",
            ),
            p("Not yet", "Nothing known."),
        ],
    },
    Guide {
        command: "codebook",
        about: "Describes variables",
        points: &[
            p(
                "As Stata 19",
                "For each variable: its label, range and units, unique and missing values, then \
                 a tabulation when it has 9 values or fewer, or else the mean, standard \
                 deviation and Stata's percentiles; if and in. Printed in R's layout.",
            ),
            p(
                "Not yet",
                "Options; the storage type (R does not keep Stata's) and the value label's name; \
                 r(), which Stata fills for some checks.",
            ),
        ],
    },
    Guide {
        command: "grstyle",
        about: "Graph style settings (grstyle, SSC)",
        points: &[
            p(
                "As Stata 19",
                "Nothing to do in R: grstyle init, set and clear only build the scheme Stata's \
                 graphs are drawn with, as set scheme chooses one. RCraft's graphs keep their \
                 own look.",
            ),
            p("Not yet", "Graphs drawn in the scheme grstyle builds."),
        ],
    },
    Guide {
        command: "suest",
        about: "Combines stored models into one set of results",
        points: &[
            p(
                "As Stata 19",
                "suest of conditional logits (clogit) stored with estimates store: e(b) with each \
                 model's coefficients and base levels, its equations named name_depvar, and the \
                 sandwich e(V) from the models' own variances and the rows' scores, clustered by \
                 vce(cluster v) or by clogit's groups, times G / (G - 1). After it, margins, \
                 dydx() of factor levels with an expression() that sums invlogit(predict(\
                 equation(eq))) or predict(equation(eq)) terms, posted or not, and test after the \
                 posted margins.",
            ),
            p(
                "Not yet",
                "Other models; models fitted with robust errors or weights; predict and other \
                 margins after suest. The table is printed in R's layout.",
            ),
        ],
    },
    Guide {
        command: "contract",
        about: "A dataset of frequencies of combinations of variables",
        points: &[
            p(
                "As Stata 19",
                "One row per combination of the variables' values, sorted by them (missing last, \
                 an empty string first), with its frequency; freq(), cfreq(), percent(), \
                 cpercent() (percents to the bit, labelled as Stata labels them, doubles or with \
                 float floats, format() or %8.2f), zero (every combination of the values seen), \
                 nomiss, if, in and fweights. The variables keep their labels and formats. A new \
                 name already in the data is r(110), no rows r(2000); the data stay as they were.",
            ),
            p(
                "Not yet",
                "Weights of zero; the storage type Stata gives the frequency (byte, int or long), \
                 which R does not have.",
            ),
        ],
    },
    Guide {
        command: "recast",
        about: "Changes a variable's storage type",
        points: &[
            p(
                "As Stata 19",
                "byte, int, long, float and double: with force a value the type cannot hold is \
                 truncated toward zero or rounded to float, and one out of range becomes \
                 missing; without it nothing changes when a value would, and Stata's count of \
                 the values is shown.",
            ),
            p("Not yet", "String types."),
        ],
    },
    Guide {
        command: "isvar",
        about: "Which names are variables (isvar, SSC)",
        points: &[
            p(
                "As Stata 19",
                "Each name, abbreviation, pattern or range that names variables gives them in \
                 r(varlist); every other word goes to r(badlist); the lines Stata shows. Observed \
                 in Stata on made-up data; the add-on's program was not read.",
            ),
            p("Not yet", "Nothing known."),
        ],
    },
    Guide {
        command: "isid",
        about: "Checks that variables identify the observations",
        points: &[
            p(
                "As Stata 19",
                "Nothing when they do; Stata's r(459) messages when a variable is missing (missok \
                 allows it) or two observations share the values.",
            ),
            p("Not yet", "sort, if and in."),
        ],
    },
    Guide {
        command: "summarize",
        about: "Summary statistics",
        points: &[
            p(
                "Standard deviation",
                "The sample standard deviation (n - 1), as R's sd(); with weights Stata's \
                 rules for each kind of weight.",
            ),
            p(
                "Percentiles",
                "With detail, Stata's rule: x[ceil(np)], or the mean of the two neighbours \
                 when np is whole, which is R's quantile type 2, not R's default type 7.",
            ),
            p(
                "Skewness and kurtosis",
                "Stata's, over the whole sample, so a normal sample's kurtosis is near 3, not 0.",
            ),
            p(
                "Printing",
                "Stata's own table, checked character for character: fweights count in Obs, \
                 aweights and iweights add the Weight column; detail's block (the label wrapped \
                 at 50 and centred, the percentiles beside the four smallest and largest values, \
                 the moments added as Stata adds them, so a skewness that is 0 but for rounding \
                 is Stata's too). iweights with detail is Stata's error. meanonly may be written \
                 mean, as in Stata.",
            ),
        ],
    },
    Guide {
        command: "svmat",
        about: "A matrix's columns as variables",
        points: &[p(
            "As Stata 19",
            "A1, A2, ... by default, the column names with names(col), stub1, stub2, ... with \
             names(stub); floats unless double; observations added when the matrix has more \
             rows than the data, with Stata's message; a name already taken stops (r(110)).",
        )],
    },
    Guide {
        command: "syntax",
        about: "Reads a program's call as the program describes it",
        points: &[p(
            "In R",
            "Done as the do-file runs: the call's text sets the locals varlist (or anything, \
             namelist), if, in and one for each option, abbreviated by its capitals \
             (Detail, noCONstant), with real and integer defaults and * for the rest, as \
             Stata sets them; what Stata refuses (a varlist required, an option not allowed) \
             is refused with its message. A varlist with a pattern, weights and passthru \
             options are not translated yet.",
        )],
    },
    Guide {
        command: "tabulate",
        about: "One-way and two-way tables (and tab1, tab2)",
        points: &[
            p(
                "Missing values",
                "Left out unless missing is given, as in Stata; R's table() leaves them out \
                 too unless useNA says otherwise.",
            ),
            p(
                "Order",
                "Categories in the order of the values underneath, with their labels, as \
                 Stata's.",
            ),
            p(
                "generate()",
                "One indicator per value, labelled as Stata labels them (cat3== followed by the \
                 number in 11 characters, 1.0000 padded on the left; s==a; f==one); rows the \
                 table leaves out are missing, and with missing a missing value has an \
                 indicator of its own.",
            ),
            p(
                "chi2",
                "Pearson's chi-squared with no continuity correction, as Stata's; R's \
                 chisq.test() corrects 2 x 2 tables unless told not to.",
            ),
            p(
                "Printing",
                "Stata's own tables for counts, and for two-way tables with row, column and \
                 cell percentages (with nofreq too: the Key box, each cell's lines, the rules \
                 between rows), checked character for character; analytic weights and string \
                 variables keep R's layout. nofreq alone prints nothing and leaves r() empty, as \
                 in Stata 19; with chi2 it prints the chi2 line only and leaves r() whole.",
            ),
            p(
                "More options",
                "lrchi2 (2 sum O ln(O/E), its line after Pearson's, r(chi2_lr) and r(p_lr); not \
                 with analytic weights yet), nokey, and matcell(), matrow() and matcol(): the \
                 counts as the table has them (analytic weights scaled to the rows) and the \
                 values of the rows and columns, r1 r2 ... and c1 c2 ..., as Stata 19 leaves \
                 them; matrow() of a string variable is Stata's error.",
            ),
            p(
                "summarize()",
                "Stata 19's tables of the summarized variable's mean, standard deviation, Freq. \
                 and (with weights or obs) Obs: one way oneway's summary table, two ways a line \
                 for each statistic in each cell (%10.0g, . for an empty cell, the title naming \
                 the statistics shown, columns in blocks of five); means, standard, freq, obs and \
                 their no forms choose the lines, with frequency and analytic weights as oneway's. \
                 r() is left empty, as in Stata.",
            ),
        ],
    },
    Guide {
        command: "test",
        about: "Wald tests after a model",
        points: &[
            p(
                "Statistic",
                "F on the model's degrees of freedom after regress, chi2 after a likelihood \
                 model, as Stata's.",
            ),
            p(
                "Lines",
                "Each constraint is written as Stata 19 writes it: the coefficients on the left in \
                 the order typed, the constant on the right (test 2*x = 3*w + 1 is ( 1)  2*x - 3*w \
                 = 1), each name with its equation after the likelihood models ([yb]x) and the \
                 base outcome's as omitted after mlogit ([3]o.x). After the models where Stata's \
                 lines were not seen the line is the constraint as typed.",
            ),
            p(
                "After xtregar",
                "xtregar's e(b) and e(V) as Stata 19 reads them: F on e(df_r) after fe, chi2 \
                 after re. estat vce prints e(V) in Stata's layout; estat's other forms after \
                 xtregar are refused.",
            ),
            p(
                "Options",
                "matvlc() keeps the constraints' variance R V R' as a matrix; mtest (noadjust, \
                 bonferroni, holm, sidak) prints Stata 19's table of each constraint's own test \
                 with its adjusted p and the joint test, and leaves r(mtest) and r(mtmethod); \
                 accumulate tests the last test's constraints with these; notest prints the \
                 constraints only and empties r(). mtest with accumulate is Stata's error.",
            ),
        ],
    },
    Guide {
        command: "tostring",
        about: "Numbers to text",
        points: &[p(
            "Format",
            "Whole numbers are written as Stata's %12.0g writes them, a missing value as \".\"; \
             other numbers are not translated yet.",
        )],
    },
    Guide {
        command: "ttest",
        about: "t tests: one sample, paired, two samples (and ttesti from summary numbers)",
        points: &[
            p(
                "As Stata 19",
                "Stata's table and lines below it: one sample, paired, two samples by() (named \
                 by their value labels, cut to 8) or unpaired; equal variances unless unequal \
                 (Satterthwaite's degrees of freedom) or welch (Welch's); level(); the three \
                 alternatives with their p-values; r() as Stata's. ttesti takes the counts, \
                 means and standard deviations typed in. Stata's errors for unequal or welch \
                 with one sample or pairs and for by() with other than two groups.",
            ),
            p(
                "Digits",
                "r() can differ from Stata's in the 16th digit (R's mean and standard \
                 deviation are worked out a little differently).",
            ),
        ],
    },
    Guide {
        command: "ttesti",
        about: "t tests from the counts, means and standard deviations typed in",
        points: &[p(
            "As Stata 19",
            "ttesti #obs #mean #sd #val (one sample) and ttesti #obs1 #mean1 #sd1 #obs2 #mean2 \
             #sd2 (two samples, with unequal or welch), with level(): ttest's table and lines, \
             the two samples together as Combined, and r() as Stata's.",
        )],
    },
    Guide {
        command: "type",
        about: "Prints a text file",
        points: &[p(
            "As Stata",
            "The file's lines with tabs expanded to stops every 8 columns, <T> with showtabs, \
             the first # with lines(#); a file not there stops with Stata's r(601). Lines \
             longer than the line size are not wrapped (Stata wraps them with > in its \
             log). asis, smcl and starbang are not translated yet.",
        )],
    },
    Guide {
        command: "twoway",
        about: "scatter, line, connected and the other twoway plots",
        points: &[GRAPH],
    },
    Guide {
        command: "use",
        about: "Reads a .dta file",
        points: &[p(
            "Labels",
            "Read by haven::read_dta: value labels and variable labels are kept as labelled \
             vectors; storage types are not.",
        )],
    },
    Guide {
        command: "version",
        about: "Sets how Stata reads the do-file",
        points: &[p("In R", "Nothing to do in R.")],
    },
    Guide {
        command: "xtreg",
        about: "Panel regression",
        points: &[
            p(
                "Estimators",
                "re is Swamy-Arora random effects with z tests, as Stata's default; fe is the \
                 within regression with the constant as the grand mean's; be the regression on \
                 the panel means; mle and pa as Stata's (pa iterated until b changes by less \
                 than 1e-6, where Stata stops).",
            ),
            p(
                "Robust errors",
                "Clustered on the panel (or the cluster variable), scaled by G / (G - 1) and \
                 (N - 1) / (N - k), as Stata's. With fewer clusters than slopes the variance is \
                 not of full rank, and the Wald test is missing, as in Stata.",
            ),
            p(
                "Collinearity",
                "A variable the constant and the ones before it explain is omitted, the later one \
                 of a collinear set, as Stata omits it; fe also omits one that does not vary \
                 within the panels. When re has more columns than panels, sigma_u is 0 and re is \
                 least squares, as in Stata. A variable named twice is kept once, the second an \
                 omitted column after it (o.x), as Stata keeps it.",
            ),
            p(
                "Differences",
                "fe with variables collinear only within the panels (age with time dummies): one \
                 must be left out, and the data cannot tell which. RCraft leaves out the one \
                 Stata leaves out: the later one, or, clustered on another variable than the \
                 panel, the one its largest-pivot sweep leaves with nothing ('collinearity with \
                 the absorbed variable'). Where two columns are left with the same (time \
                 dummies of one year, say), Stata's rounding decides between them; RCraft then \
                 leaves out the earlier, and a note names the coefficients that may differ \
                 (those of the set and _cons). Every other result is the same whichever is \
                 left out: the other coefficients and their standard errors, N, the \
                 R-squareds, sigma_u, sigma_e, rho, F and the predictions.",
            ),
            MISSING_IN_MODELS,
            LAYOUT_R,
        ],
    },
    Guide {
        command: "xtdescribe",
        about: "Describes the participation pattern of a panel",
        points: &[
            p(
                "As Stata 19",
                "xtdescribe prints the panel and time lines (first, second, ..., last), Delta and \
                 Span, the distribution of T_i and the patterns of presence, most frequent first \
                 (patterns(#), 9 by default; width(#) groups periods into one column each, with \
                 the note under the table). r() is summarize T_i, detail. A missing time counts \
                 in T as a value of its own, and rows with a missing panel are left out. The \
                 corpus file xtpanel checks r() and the tables were compared with Stata's \
                 printout.",
            ),
            p(
                "Errors",
                "Without xtset (or with a panel and no time) r(459), a time with fewer than two \
                 values r(459), weights and a varlist r(101), an unknown option r(198). \
                 i() and t() other than the variables xtset holds are refused, as is width() \
                 below 3 and a %tc time.",
            ),
        ],
    },
    Guide {
        command: "xtsum",
        about: "Summarizes variation within and between panels",
        points: &[
            p(
                "As Stata 19",
                "xtsum [varlist] prints the overall, between and within mean, standard \
                 deviation, minimum and maximum with N, n and T-bar (T when every panel has as \
                 many rows). Without a varlist every variable is shown (a string one as no \
                 rows). A missing panel value is a panel of its own. r() holds the last \
                 variable's thirteen values.",
            ),
            p(
                "Errors",
                "Without xtset r(459), weights r(101), an unknown option r(198), a name the \
                 data lack r(111). i() other than the panel variable is refused.",
            ),
        ],
    },
    Guide {
        command: "xttab",
        about: "Tabulates a variable overall, between and within panels",
        points: &[
            p(
                "As Stata 19",
                "xttab x prints the overall, between and within frequencies and percents (the \
                 between percent of all panels, so the column can pass 100; the within percent \
                 the mean share of the panels' rows). A value shows as its value label cut to 8 \
                 characters, or as %9.0g writes it. r(results) is the table (its values as \
                 floats) and r(n_panels) the number of panels.",
            ),
            p(
                "Errors",
                "No varlist r(100), two variables r(103), a string variable r(109), no rows \
                 r(2000), without xtset r(459), weights r(101), any option other than i() \
                 r(198).",
            ),
        ],
    },
    Guide {
        command: "tsreport",
        about: "Reports the periods and gaps of time-series data",
        points: &[
            p(
                "As Stata 19",
                "tsreport prints the starting and ending period, the observations and the gaps \
                 (with a panel the end of each panel counts too, the last row only when its \
                 period is before the sample's end, unless panel is given). detail lists the \
                 gaps; a varlist gives a summary table (one report each with detail, or one \
                 for the rows complete in all with casewise). r() holds N_gaps, N_obs, start, \
                 end (numbered with a varlist), the text panelvar, timevar and tsfmt, var# and \
                 the table(s) of the gap report.",
            ),
            p(
                "Errors",
                "Without tsset r(111), a name the data lack r(111), a string variable r(109), \
                 casewise with one variable r(198), an unknown option r(198), weights r(101). \
                 Data not in panel and time order and a %tc time are refused.",
            ),
        ],
    },
    Guide {
        command: "tsappend",
        about: "Adds periods to time-series data",
        points: &[
            p(
                "As Stata 19",
                "tsappend, add(#) adds # periods after each panel's last; last(date) \
                 tsfmt(td, tw, tm, tq or th) adds periods up to a date. The periods missing \
                 inside each panel are filled too, as tsfill does; the other variables are \
                 missing in new rows, and the data end in panel and time order. r(add) is # (or, \
                 with last(), the periods added after the panels' ends). When last() is not \
                 after the data's end Stata says so and adds nothing.",
            ),
            p(
                "Errors",
                "Neither add() nor last() r(198), add() not a positive whole number, add() with \
                 last() or tsfmt(), last() without tsfmt(), a missing time, an unknown option \
                 r(198); a varlist, if, in and weights r(101); without tsset r(111). Data not in \
                 panel and time order, a missing panel value and tsfmt() other than td, tw, tm, \
                 tq and th are refused.",
            ),
        ],
    },
    Guide {
        command: "dfuller",
        about: "Augmented Dickey-Fuller unit-root test",
        points: &[
            p(
                "As Stata 19",
                "dfuller y regresses D.y on L.y, lags(#) lagged differences, _trend with trend and a constant unless noconstant, over the rows if and in keep; a gap in time or a missing value only drops the rows that need it. The statistic is L.y's t, with Fuller's critical values (linear in the sample size) and MacKinnon's approximate p-value; drift uses the t distribution of the regression. regress shows the regression table. A time-series operator may stand for the variable. r() holds lags, N, Zt, p (not with noconstant) and cv_1, cv_5, cv_10. A negative lags() counts as 0.",
            ),
            p(
                "Errors",
                "Without tsset r(111), a second variable r(103), no variable r(100), trend with noconstant, drift with trend or noconstant and an unknown option r(198), lags() that is not a whole number r(198), several panels in the sample r(459), a string variable r(109). A sample too small for the regression is refused.",
            ),
        ],
    },
    Guide {
        command: "pperron",
        about: "Phillips-Perron unit-root test",
        points: &[
            p(
                "As Stata 19",
                "pperron y regresses y on L.y (and _trend, and a constant unless noconstant) and corrects the statistics for the residuals' autocorrelation with Newey-West weights; lags(#) sets the lags (the default is floor(4 (T/100)^(2/9)); a negative number gives the default). Z(rho) and Z(t) have Fuller's critical values; the p-value is MacKinnon's for Z(t) (none with noconstant). r() holds N, lags, Zrho, Zt, pval and the matrix cvalues (the statistic and the 1%, 5% and 10% values of each row).",
            ),
            p(
                "Errors",
                "As dfuller. A sample too small for the regression is refused.",
            ),
        ],
    },
    Guide {
        command: "dfgls",
        about: "DF-GLS unit-root test",
        points: &[
            p(
                "As Stata 19",
                "dfgls y detrends the series by GLS (a trend, or only a mean with notrend) and tests the detrended series for each lag from maxlag(#) (the default is Schwert's floor(12 (T/100)^(1/4))) down to 1. The 1% value is Elliott, Rothenberg and Stock's, the 5% and 10% values Cheung and Lai's response surface (all three Elliott-Rothenberg-Stock's with ers). It prints the lag Ng and Perron's sequential test picks and the minimum SIC and MAIC. r() holds maxlag, N, the chosen lags and criteria, and the matrices cvalues and results; with maxlag(0) only N, cv_1, cv_5, cv_10, rmse0 and dft0, with maxlag(1) only maxlag, N and the matrices.",
            ),
            p(
                "Errors",
                "Without tsset r(111), a gap in the sample r(498), maxlag() that is not a whole number r(126), more lags than the sample holds r(2000) or r(2001), several panels r(459), an unknown option r(198). A sample too small for the lags is refused.",
            ),
        ],
    },
    Guide {
        command: "corrgram",
        about: "Autocorrelations, partial autocorrelations and Q statistics",
        points: &[
            p(
                "As Stata 19",
                "corrgram y, lags(#) prints the autocorrelation (sum of products about the mean over the sum of squares, over the pairs of times both present), the partial autocorrelation (the last coefficient of y on its first k lags, or Yule-Walker with yw), Ljung and Box's Q and its p-value, and the bars (not with noplot). The default is min(floor(n/2) - 2, 40) lags. A gap in time or a missing value is noted. r() holds lags, the scalars ac#, pac#, q# and p# for the first ten lags and the matrices AC, PAC, Q and P with all of them.",
            ),
            p(
                "Errors",
                "Without tsset r(111), a second variable r(103), no variable r(100), lags() of 0 or as long as the sample r(498), a string variable r(109), several panels r(459), an unknown option r(198).",
            ),
        ],
    },
    Guide {
        command: "wntestq",
        about: "Portmanteau (Q) test for white noise",
        points: &[
            p(
                "As Stata 19",
                "wntestq y, lags(#) prints Ljung and Box's Q at the last lag with its chi-squared p-value (the default lags are those of corrgram). r() holds df, p and stat.",
            ),
            p(
                "Errors",
                "As corrgram, except that a string variable gives r(2000).",
            ),
        ],
    },
    Guide {
        command: "xtunitroot",
        about: "Panel unit-root tests",
        points: &[
            p(
                "As Stata 19",
                "xtunitroot ips, ht, breitung, fisher, hadri and llc on strongly balanced panels, with if and in. ips: the mean of the panels' Dickey-Fuller t statistics (t-bar with Table 2's exact critical values, Z-t-tilde-bar), or W-t-bar with lags(0 to 8). ht: the pooled AR coefficient (altt shortens T by one). breitung: lambda, with noconstant, trend or demean. fisher: Choi's P, Z, L* and Pm from each panel's dfuller (lags(), trend, drift) or pperron p-values. hadri: the LM z statistic, robust or with a Bartlett or Parzen kernel of a given length. llc: Levin, Lin and Chu's adjusted t* (unadjusted t, delta, sbar, with lags(#) and kernel(bartlett # or parzen #, or ... llc) and the adjustment table by the number of periods; the default bandwidth follows each panel's rows in the data). demean removes the cross-sectional means. When the sample starts after the data do, the first period's lag is read from the period before (but not for a demeaned series). r() holds N, N_t, N_g, the statistics and p-values, and the texts test, deterministics, demean, urtest and options.",
            ),
            p(
                "Errors",
                "No test or an unknown one, an unknown option, noconstant with ips, hadri and trend, lags() or a test missing for fisher r(198); no variable r(100), two variables r(103); without xtset r(459); a name the data lack r(111); a string variable r(109). Unbalanced panels with llc r(498). Refused: llc with lags(aic or bic or hqic) or kernel(... nwest), breitung with robust or lags(), ips with lags() above 8 or lags(aic), hadri kernels other than bartlett # and parzen #, fisher with noconstant, a panel whose p-value is 0 or 1, the Phillips-Perron form with trend, demean or a sample that starts late, ht with trend on a sample that starts late, panels that are not strongly balanced or have gaps in time, fewer than 4 periods.",
            ),
        ],
    },
    Guide {
        command: "varsoc",
        about: "Lag-order selection statistics for a vector autoregression",
        points: &[
            p(
                "As Stata 19",
                "varsoc y w, maxlag(#) (the default is 4) fits lag orders 0 to # on the sample the longest one leaves and prints each one's log likelihood, the likelihood-ratio test against the order before (a star on the last significant at 5%), FPE, AIC, HQIC and SBIC (a star on each one's smallest). exog() adds variables to every equation. r() holds N, mlag, tmin, tmax, N_gaps, the texts endog and exog and the matrix stats (lag, LL, LR, df, p, FPE, AIC, HQIC, SBIC; lag 0 has missing LR, df and p).",
            ),
            p(
                "Errors",
                "No variable, an unknown option and maxlag() that is not a positive whole number r(198), a name the data lack r(111), a string variable r(109), weights r(101), without tsset r(111). Refused: noconstant, varsoc after xtset, a name list that is not plain variables, and after a var (the lags then come from the model).",
            ),
        ],
    },
    Guide {
        command: "xtset",
        about: "Declares panel data",
        points: &[p(
            "Time",
            "With a time variable it is tsset: the times are checked, the data sorted by panel \
             and time, and L., F., D. and S. work as under tsset. A time unit (daily, yearly) or \
             format() makes Stata show the time as dates and gives it that format; here it shows \
             as a number. The lags are the same. A .dta file Stata saved after xtset or tsset \
             keeps the settings, and Stata sets them again when it reads the file: when the \
             do-file runs, RCraft reads them from the file too (the delta included).",
        )],
    },
    Guide {
        command: "coefplot",
        about: "Plots stored models' coefficients and intervals (SSC)",
        points: &[
            p(
                "In R",
                "stata_coefplot_rows works out the rows as coefplot does, from Stata's names in \
                 e(b): _cons included, base levels with baselevels and omitted terms with omitted \
                 (both 0), in the order they first appear with _cons last, keep(), drop() and \
                 order() with its empty rows; intervals with the model's residual df (the normal \
                 without one) at levels() (or ci() of numbers); several plots 1/(k+1) apart, offset() moving one; \
                 headings(); subgraphs at ||. gen() leaves those rows, as floats, as coefplot does.",
            ),
            p(
                "Several equations",
                "A model with several equations (margins posted after cmclogit) is drawn alone \
                 and with noeqlabels: its rows in e(b)'s order, only the first equation's unless \
                 keep() says otherwise; keep(), headings() and coeflabels() read eq: names as \
                 Stata 19 does (the equation kept for the names after it, eq: alone all its \
                 names). Equation labels, order() and several plots of such models are refused.",
            ),
            p(
                "Look",
                "The graph is drawn by ggplot2, so it looks like R's; colours, markers and the \
                 like are not carried over. Labels are the variable's label or name, a factor \
                 level's value label or g=2.",
            ),
        ],
    },
    Guide {
        command: "cluster2",
        about: "Least squares with errors clustered two ways (Petersen's add-on)",
        points: &[
            p(
                "As Stata 19",
                "Matched to the add-on's results, not its code: the variance clustered by \
                 fcluster() plus the one clustered by tcluster() less the one clustered by their \
                 cells, each with Stata's G/(G-1) (N-1)/(N-K); with one row per cell the last is \
                 the robust one. e(F) and e(df_r) are the cell-clustered fit's (the robust one's \
                 with one row per cell); a variance below 0 is kept and its standard error is \
                 missing; Stata's header and footer, and no e(sample), as the add-on leaves.",
            ),
            p(
                "Differences",
                "A missing value in a cluster variable stops the R script: the add-on then fits \
                 its parts on different rows, which RCraft does not reproduce. Collinear \
                 variables, factor variables and predict after it are not translated yet; the \
                 coefficient table is in R's layout.",
            ),
        ],
    },
    Guide {
        command: "medeff",
        about: "Causal mediation by simulation (Hicks and Tingley's mediation package, SSC)",
        points: &[
            p(
                "As Stata 19",
                "Worked out from medeff's output, not its code: the mediator model and the \
                 outcome model (refitted with the treatment first, so e() is Stata's) on the rows \
                 complete in both; seed() then sims rows of each model's coefficients from \
                 Stata's own generator (b + L z, z = invnormal(runiform()) a row at a time, the \
                 seed set again before the outcome model), the ACME, direct and total effects \
                 stored as floats, their means and _pctile's 2.5 and 97.5 percentiles, the share \
                 mediated as the median of mean ACME over each total effect; Stata's table and \
                 r(), and the generator left where medeff leaves it.",
            ),
            p(
                "Not yet",
                "Models other than regress (probit, logit), interactions, treatment values other \
                 than 0 and 1, medeff without sims() or seed(), its other options, and medsens.",
            ),
        ],
    },
    Guide {
        command: "marginsplot",
        about: "Plots the margins margins left",
        points: &[
            p(
                "In R",
                "After margins with one at() of lists and no terms or effects (not posted): \
                 Stata 19's line naming the variables at() sets to several values (in at()'s \
                 order), the margins drawn against the first with their intervals, a line for \
                 each value of the others, by ggplot2 (so the look is R's). r() stays margins', \
                 as in Stata.",
            ),
            p(
                "Not yet",
                "marginsplot after margins with terms, dydx() and the other effects, several \
                 at(), or posted, and its options other than the titles and noci.",
            ),
        ],
    },
    Guide {
        command: "mvreg",
        about: "Multivariate regression",
        points: &[
            p(
                "As Stata 19",
                "mvreg y1 y2 = x1 x2 fits each equation by least squares on the same rows; e(Sigma) \
                 = E'E/(N - k), V = Sigma kron (X'X)^-1, t tests on N - k degrees of freedom, and \
                 e(F), e(rmse), e(p_F), e(r2) as space-led lists, one per equation. The equation \
                 table, the coefficient table, corr (the correlation matrix of the residuals and \
                 the Breusch-Pagan test), noheader, notable, noconstant, level(), fweights and \
                 aweights (scaled to sum to N) are Stata's; pweights and iweights stop with \
                 `not allowed`, r(101). predict (xb, residuals, equation(name or #k)) and test \
                 across equations ([y1]x1 = [y2]x1) work after it.",
            ),
            p(
                "Not yet",
                "Factor variables and operators, a single dependent variable, constraints, and \
                 predict's other statistics.",
            ),
        ],
    },
    Guide {
        command: "sureg",
        about: "Seemingly unrelated regression",
        points: &[
            p(
                "As Stata 19",
                "sureg (y1 x1 x2) (y2 x1 x3) is two-step feasible GLS: Sigma from each equation's \
                 least-squares residuals (cross products over N, or over sqrt((Ni - ki)(Nj - kj)) \
                 with dfk), b = (X'(Sigma^-1 kron I)X)^-1 X'(Sigma^-1 kron I)y, V the inverse. isure \
                 (with nolog) goes round again with the residuals of the last fit until no \
                 coefficient moves by 1e-6; small gives F and t on N*M - k degrees of freedom; dfk \
                 prints Stata's note. Per equation: R-squared (centred), RMSE, the Wald chi2 of \
                 its slopes; e(ll), e(Sigma), e(b), e(V) and Stata's e() names. corr, noheader, \
                 notable, level(), fweights and aweights, predict (xb, residuals, equation()) and \
                 test across equations are taken; noconstant, pweights and iweights stop as Stata \
                 stops (r(198), r(101)).",
            ),
            p(
                "Not yet",
                "isure without nolog (Stata's iteration log), equation names (name: y x), the \
                 y1 y2 = x form, constraints(), factor variables, one equation, the same dependent \
                 variable twice, and predict's other statistics. e(dfk2_adj) is N (Stata's has \
                 rounding noise).",
            ),
        ],
    },
    Guide {
        command: "nlsur",
        about: "Systems of nonlinear equations",
        points: &[
            p(
                "As Stata 19",
                "nlsur (y1 = <expression>) (y2 = <expression>) with {name} and {name = #} parameters (in order of first \
                 appearance, shared between equations by name; unset starts are 0) is Gauss-Newton on the stacked \
                 equations, as Stata takes it: forward-difference derivatives, each step halved while it raises the \
                 criterion, done when no parameter moves by more than eps() (1e-5, relative to |b|); a parameter whose \
                 derivative column is zero or a combination of earlier ones' stays put and shows (constrained). NLS \
                 comes first (log: Residual SS), then FGNLS with Sigma from its residuals (Scaled RSS); ifgnls goes \
                 round again until the parameters and Sigma each change by at most 1e-5 (the Parameter change and \
                 Covariance matrix change lines). The iteration log, the equation table (RMSE, R-sq with the equation's \
                 constant, or uncentered with a star), the z table and e() (rss, mss, r2, rmse and k per equation, ll, \
                 rank, converged, params, constants, sexp, b, V, init, Sigma) follow Stata's; nolog, level(), iterate(), \
                 eps(), noconstant, initial(), if and in are taken, and predict gives yhat and residuals with \
                 equation(#k). Failed runs stop as Stata does (an unevaluable start, missing values on the right-hand \
                 side: r(480)).",
            ),
            p(
                "Not yet",
                "Weights, vce(), variables(), parameters() and the function-evaluator form, delta(), {name: varlist} \
                 linear combinations, nls when a parameter is shared by two equations (Stata's estimates there are not \
                 the stacked least squares), ifgnls with iterate(), and the table of a run that has not converged \
                 (RCraft stops with a message after the log). The cap on iterations (100) is an assumption. test, \
                 lincom and margins after nlsur are not translated; _b[a:_cons] works.",
            ),
        ],
    },
    Guide {
        command: "areg",
        about: "Linear regression absorbing the indicators of one variable",
        points: &[
            p(
                "As Stata 19",
                "areg y x..., absorb(g) takes the slopes from the within fit (each group's weighted means removed, the \
                 grand means put back, so the constant is ybar - xbar b) with N - k - G residual degrees of freedom, \
                 and R-squared, rss and mss of the model with the indicators; R-squared adjusted divides by df_r and \
                 F is the Wald test of the slopes. vce(robust), vce(cluster v) and the old cluster() are taken (robust \
                 times N/(N - k - G); clusters G/(G - 1) (N - 1)/(N - k - G), t and F on G - 1), and \
                 fweights, aweights and pweights (which imply robust). The `F test of absorbed indicators` \
                 (the pooled fit against this one) is printed unless the errors are robust; e() holds Stata's \
                 scalars (e(F_absorb), e(ll_0) the within data's constant-only likelihood, e(rank) that of \
                 the slopes and constant, one less after clusters) and e(b) and e(V) with a zero column for a \
                 factor's base level. i.g factor variables are taken. predict gives xb (the default), \
                 residuals, d (the absorbed effect of the group) and xbd; test works after it.",
            ),
            p(
                "Not yet",
                "Regressors the absorbed groups make collinear (Stata omits them with a note), \
                 interactions and ib. bases, iweights (stopped as Stata stops, r(101)), e(datasignature) \
                 and predict's other statistics.",
            ),
        ],
    },
    Guide {
        command: "qreg",
        about: "Quantile regression",
        points: &[
            p(
                "As Stata 19",
                "qreg y x..., quantile(#) nolog (quantile(75) means .75) minimises the sum of q|r| above and \
                 (1 - q)|r| below by descending the edges of the linear programme's vertices, the iid variance \
                 as Stata's (sparsity from fits at q +/- h with Hall and Sheather's bandwidth, z from level()), \
                 the header with the raw and minimum sums of deviations, the pseudo R2 and e() (rank, sparsity, \
                 bwidth, df_m, df_r, f_r, q_v, sum_rdev, sum_adev, r2_p, b, V). fweights, factor variables, \
                 if, in and level() are taken; aweights, pweights and iweights stop as Stata stops (r(101)). \
                 predict gives xb and residuals.",
            ),
            p(
                "Not yet",
                "Fits where the minimum is not unique (often with dummy regressors at the median): Stata ends at \
                 one of several optimal vertices by its own path, so RCraft stops with a message instead of \
                 printing a vertex that might differ. Stata's iteration log (use nolog), vce(), other \
                 density and bandwidth methods, interactions and collinear regressors.",
            ),
        ],
    },
    Guide {
        command: "logistic",
        about: "Logistic regression, reporting odds ratios",
        points: &[
            p(
                "As Stata 19",
                "logistic y x... is logit that shows odds ratios (coef shows the coefficients) and prints no \
                 iteration log, with `Note: _cons estimates baseline odds.`; the estimates, e() and the header are \
                 logit's, with e(cmd) logistic and e(predict) logistic_p. level(), vce(), robust, if, in and \
                 weights are logit's; predict and test work after it.",
            ),
            p(
                "Not yet",
                "noconstant (Stata leaves e(ll_0) missing and a Wald chi2 there) and the options logit has \
                 not been translated with.",
            ),
        ],
    },
    Guide {
        command: "xi",
        about: "Expand i. terms into dummy variables",
        points: &[
            p(
                "As Stata 19",
                "xi: regress y i.g x (and any command with i. terms), xi i.g and xi, prefix(_D): ... make a byte \
                 variable _Ig_# for each level of g but the omitted one (the lowest; the most frequent with char \
                 _dta[omit] prevalent; # with char g[omit] #), labelled g==#, and run the command on them. \
                 i.g*i.h adds both sets and _IgXh_#_# (g==2 & h==1), i.g*x the dummies and the float _IgXx_# \
                 ((g==2)*x), i.g|x the same with the dummies left out of the command. Each run drops the variables \
                 an earlier run made with the same prefix, prints a line per term (`i.g  _Ig_1-3  (naturally coded; \
                 _Ig_1 omitted)`) and leaves e(cmdline) with the stubs (`regress y _Ig_* x`). The levels are asked \
                 of R when the do-file runs, so a dry run shows the line as known only when run.",
            ),
            p(
                "Not yet",
                "A factor in two terms, three or more parts in a term, | between two factors, noomit and the other \
                 options, noninteger or negative levels, names over 32 characters and tied most-frequent levels.",
            ),
        ],
    },
    Guide {
        command: "cnsreg",
        about: "Constrained linear regression",
        points: &[
            p(
                "As Stata 19",
                "cnsreg y x..., constraints(numlist) reads each defined constraint as a linear \
                 equation in the coefficients (`x1 = x2`, `2*x1 - x2 = x3 + 1`, `_cons = 2`, \
                 `_b[x1] = 1`), counts equal ones once, and fits restricted least squares by \
                 projection: b = b0 + T a, b0 the smallest b meeting R b = r, T a basis of what \
                 the constraints leave free. Degrees of freedom N - (k - m), s^2 = RSS/df. noconstant, \
                 vce(robust) and vce(cluster v) (HC1, G/(G-1) with the clusters), level(), \
                 fweights, aweights and pweights (which imply robust) are taken. The F test is that \
                 of the slopes given the constraints, and goes with its Prob > F when it \
                 contradicts them (a slope fixed at a value); e(p) keeps the residual degrees of \
                 freedom even with clusters, as Stata does. A constraint that does not parse, or names \
                 a variable the model lacks, is dropped with Stata's note; none left is r(111), \
                 repeated or contradicting ones r(412), a missing constraints() r(198). The header, \
                 the constraint lines, `(constrained)` and the table print as Stata's, and e() \
                 (b, V, Cns, V_modelbased) is Stata's. predict (xb, residuals) and test work after it.",
            ),
            p(
                "Not yet",
                "Factor variables and operators in the model, iweights, collinear, mse1, and \
                 constraints with more than one =, products or powers of coefficients (Stata: r(131)) \
                 beyond the error, or the [eq]x form beyond its r(303).",
            ),
        ],
    },
    Guide {
        command: "constraint",
        about: "Define and list linear constraints",
        points: &[
            p(
                "As Stata 19",
                "constraint [define] # exp keeps the text as typed (1 to 1999), without checking it \
                 until a command uses it; constraint dir and list [numlist] print `     1:  x1 = x2`; \
                 get # sets r(defined) and r(contents); free sets r(free), the highest number not in \
                 use; drop takes a numlist or _all. Only get and free change r(). cnsreg reads them.",
            ),
            p(
                "Not yet",
                "Numbers above 1999 (Stata accepts them silently), quoted text, and the \
                 constraints() option of other models (logit, poisson, sspace and the rest).",
            ),
        ],
    },
    Guide {
        command: "rreg",
        about: "Robust regression (Huber then biweight iterations)",
        points: &[
            p(
                "In R",
                "stata_rreg fits it as Stata 19 does, worked out from its logs and results: least \
                 squares first, rows of Cook's distance above 1 left out (silently, as Stata); \
                 Huber weights min(1, 2M/|r|), M the residuals' median absolute deviation, until \
                 the largest change in a weight is below 0.05; then biweight weights with 4.685 \
                 times M/0.6745 until it is below 0.01, each a weighted least-squares fit. The \
                 variance is the pseudovalues' rmse^2 (X'X)^-1, F the Wald test of the slopes, \
                 e() as Stata's (r2, mss, rss and beta from them). The iteration log, header and \
                 table print as Stata's.",
            ),
            p(
                "Not yet",
                "Options (tune(), tolerance(), genwt(), level() and the rest), weights, factor \
                 variables, and predict, test and margins after it are refused.",
            ),
        ],
    },
    Guide {
        command: "cmset",
        about: "Declares choice-model data: the case and alternatives variables",
        points: &[p(
            "In R",
            "Prints Stata 19's two lines and keeps the case and alternatives variables for \
             cmclogit after it. Options of cmset (a panel's time variable, noalternatives) are \
             refused.",
        )],
    },
    Guide {
        command: "cmclogit",
        about: "Conditional logit choice model (McFadden's), with margins after it",
        points: &[
            p(
                "In R",
                "stata_cmclogit fits the conditional logit by Newton-Raphson in base R: each \
                 alternative-specific term (x, i.x, ib#.x) one column, each case-specific term \
                 of casevars() one coefficient for each alternative but the base one, which is \
                 the one most frequent in the data (the first of a tie), as in Stata; each such \
                 equation named by the alternative's value, a constant in each unless \
                 noconstant. vce(cluster v) is H^-1 (sum of the clusters' scores) H^-1 times \
                 G / (G - 1). e() and the table are Stata 19's; its iteration log is not shown.",
            ),
            p(
                "margins",
                "margins, dydx(i.x ...) outcome(#) alternative(#) asobserved post after it: for \
                 each level the mean over cases of Pr(outcome) with the alternative's x at that \
                 level less with it at the base, with the analytic gradient; vce(unconditional) \
                 clusters each case's term as Stata does, the default is the delta method. e(b) \
                 has an equation a level (1.x:_cons, the base 3b.x:o._cons), as Stata's.",
            ),
            p(
                "Not yet",
                "Weights, vce(robust), interactions, margins without post or of continuous \
                 terms, and predict, test and lincom after cmclogit are refused.",
            ),
        ],
    },
    Guide {
        command: "clogit",
        about: "Conditional (fixed-effects) logistic regression",
        points: &[
            p(
                "In R",
                "stata_clogit fits the exact conditional likelihood, as Stata does, with R's \
                 survival::coxph (one stratum per group, method = \"exact\"); several positive \
                 outcomes in a group are fine. Groups that are all positive or all negative are \
                 dropped with Stata's note, and so is a variable that does not vary within any \
                 group. With one positive outcome a group, the estimates are where Stata's \
                 Newton-Raphson stops (from an ordinary logit's slopes, Stata's steps and \
                 stopping rule), which can be a few parts in 1e7 short of the maximum.",
            ),
            p(
                "Robust errors",
                "survival gives no robust variance with the exact method, so the scores are \
                 worked out from each group's probabilities. vce(robust) clusters on the group, \
                 as in Stata; with vce(cluster v) the groups must be nested in the clusters \
                 (r(459)). Both carry G / (G - 1).",
            ),
            p(
                "predict",
                "pc1 (the default: the probability of being the positive one within the group, \
                 on every row), pu0 and xb.",
            ),
            p(
                "Not yet",
                "Weights, offsets and the other predict statistics; margins after clogit.",
            ),
        ],
    },
    Guide {
        command: "esttab",
        about: "Tables of stored models (estout package, SSC)",
        points: &[
            p(
                "In R",
                "stata_esttab lays the table out as esttab prints and writes it, matched to its \
                 output: Stata's names in e(b) in order of first appearance with _cons last, base \
                 levels as 0 over (.), t statistics (se, p or none) in parentheses, esttab's own \
                 number format (three significant digits) or the format given, stars at 0.05, 0.01 \
                 and 0.001 or star(), labels with label, N and r2, ar2, aic, bic and scalars(); \
                 the screen table and .txt in fixed columns, .csv with =\"...\" cells, .tex with \
                 sym stars, booktabs or plain rules, lines ending CR LF.",
            ),
            p(
                "More options",
                "stats() with fmt() and labels() (only the rows it names, in b()'s format without \
                 fmt()), varlabels() (pairs in turn; it also takes away the blank lines between the \
                 coefficients), mgroups() with pattern(), and prefix() suffix() span in .tex, \
                 starlevels() (the stars as typed), compress, nogaps, and tex on the screen, each \
                 as Stata 19 printed and wrote them. After estpost summarize, cells() of its \
                 statistics with fmt() (a statistic without one takes the first one's).",
            ),
            p(
                "Head and foot",
                "prehead() posthead() prefoot() postfoot() put their lines in place of a part of the \
                 table (@title in prehead() is the title), fragment leaves out the .tex head and \
                 foot, mgroups(..., erepeat()) follows the group row, mlabels() with span prefix() \
                 suffix() or without them, alignment() sets .tex's columns, and addnotes() goes \
                 between the first note and the stars', as Stata 19 printed and wrote them.",
            ),
            p(
                "matrix()",
                "esttab matrix(M, fmt()) lays a matrix out as Stata 19 does: its row and column \
                 names, a row of the matrix's name (mlabels(\" \") blank, mlabels(none) none), \
                 each cell %9.0g or fmt()'s decimals (one for all or one a column), . for a \
                 missing one; on the screen and in .txt between rules, in .tex a tabular unless \
                 fragment with posthead() prefoot() postfoot() in their places, in .csv =\"\" \
                 cells. fmt() with %formats, other options and names over 12 characters are \
                 refused.",
            ),
            p(
                "Not yet",
                "wide, cells() of other models, mlabels() with mtitles(), other @ \
                 substitutions, fragment with prehead() or postfoot(), .rtf (it carries the time \
                 it was written) and not with a file are refused.",
            ),
        ],
    },
    Guide {
        command: "factor",
        about: "Factor analysis (factor varlist, pf, pcf, ipf, factors(#))",
        points: &[
            p(
                "In R",
                "stata_factor works on the correlation matrix of the rows with every variable \
                 present: principal factors (the squared multiple correlations as communalities), \
                 principal-component factors, or iterated principal factors (until the squared \
                 changes in the communalities sum below 1e-8, as Stata 19 stops, or citerate() \
                 rounds), keeping the factors with an eigenvalue above mineigen() (0, or 1 with \
                 pcf), at most factors(). Stata 19's tables and the test of independence, each \
                 factor signed so its loadings sum to more than 0; e() as Stata's but e(rngstate).",
            ),
            p(
                "Not yet",
                "ml, weights, and what follows factor (predict, estat) are not translated yet; \
                 rotate is.",
            ),
        ],
    },
    Guide {
        command: "pca",
        about: "Principal components (pca varlist, components(#), covariance)",
        points: &[
            p(
                "In R",
                "stata_pca takes the eigenvalues and eigenvectors of the correlation (or with \
                 covariance the covariance) matrix of the rows with every variable present, each \
                 eigenvector signed so that its elements sum to more than 0, as Stata's manual \
                 gives it, and prints Stata 19's two tables (the eigenvalues, their differences \
                 and proportions; the eigenvectors kept with each variable's unexplained \
                 variance). e() is pca's: e(Ev), e(L), e(Psi), e(C), e(components), e(rho), \
                 e(trace), e(cond), e(lndet), e(N).",
            ),
            p(
                "Not yet",
                "Weights, the options other than components() and covariance, and what follows \
                 pca (predict, estat loadings, screeplot) are not translated yet; rotate is.",
            ),
        ],
    },
    Guide {
        command: "kap",
        about: "Interrater agreement (kap rater1 rater2, kap r1 r2 r3, wgt(w), wgt(w2), tab, absolute)",
        points: &[
            p(
                "In R",
                "stata_kap gives Cohen's kappa for two raters over the ratings either used, \
                 unweighted or weighted by w (1 - |i-j|/(k-1)) or w2 (1 - ((i-j)/(k-1))^2), on \
                 the ratings' ranks or with absolute their values, with frequency weights, and its \
                 standard error under no agreement beyond chance; for three or more raters, \
                 Fleiss's kappa for each outcome and combined with their z tests. Stata 19's \
                 tables (tab's two-way table of the ratings' values) and r(); Stata's errors.",
            ),
            p(
                "Not yet",
                "kapwgt and its weights, kappa (the frequency form), raters per subject that vary \
                 (missing ratings with three or more raters), a chance agreement that leaves no \
                 test (Stata's dots), tab with more than five ratings, and weights with three or \
                 more raters are not translated. r() can differ from Stata's in the 16th digit.",
            ),
        ],
    },
    Guide {
        command: "tetrachoric",
        about: "Tetrachoric correlations of 0/1 variables (tetrachoric b1 b2, stats(rho se p obs), edwards)",
        points: &[
            p(
                "In R",
                "stata_tetrachoric takes each pair's thresholds from its margins and the rho that \
                 maximizes the 2x2 table's likelihood (the bivariate normal by Sheppard's integral), \
                 its standard error the full model's inverse information; edwards gives Edwards and \
                 Edwards's (a^(pi/4) - 1)/(a^(pi/4) + 1) of the odds ratio with its delta-method \
                 error; the p-value is Fisher's exact test, Bonferroni's or Sidak's. Listwise or pw, \
                 frequency weights, Stata 19's two-variable report and matrix (stats(), print(), \
                 star(), matrix, notable) and r().",
            ),
            p(
                "Not yet",
                "A 2x2 table with an empty cell (and zeroadjust), correlations within .001 of -1 or \
                 1, posdef's adjustment of a matrix that is not positive definite, and weights other \
                 than a frequency-weight variable are not translated. r(rho) and r(se_rho) can \
                 differ from Stata's in the 16th digit.",
            ),
        ],
    },
    Guide {
        command: "icc",
        about: "Intraclass correlations (icc y target [rater], mixed, consistency, absolute, testvalue(#))",
        points: &[
            p(
                "In R",
                "stata_icc gives the one-way and two-way random-effects and two-way mixed-effects \
                 ICCs (individual and average) from the ANOVA mean squares, absolute agreement or \
                 consistency, with McGraw and Wong's confidence intervals (Satterthwaite's df for \
                 absolute agreement) and F tests of testvalue(), level() and format(), Stata 19's \
                 table, note and r(); a target not rated by all raters (or, one-way, with fewer \
                 ratings) is left out as Stata does, with its message (one or several), and \
                 Stata's errors: weights, mixed with one-way models, a target rated twice by a \
                 rater, no observations, too few targets or raters.",
            ),
            p(
                "Not yet",
                "r()'s bounds and p-values can differ from Stata's past the 13th digit (Stata's F tail with \
                 fractional df far out by about 1e-6 of itself).",
            ),
        ],
    },
    Guide {
        command: "mhodds",
        about: "Mantel-Haenszel odds ratios (mhodds case exposure [controls], by(), compare())",
        points: &[
            p(
                "In R",
                "stata_mhodds gives the odds ratio of a two-valued exposure (or compare()'s two \
                 values), Mantel and Haenszel's across the controls' strata, with its chi2 and an \
                 interval on the variance sum(V)/(R*S); more values give the score test for \
                 trend, exp(U/V) with exp(U/V +/- z/sqrt(V)). by() (one or two variables) adds \
                 each group's estimate, the estimate over all strata and the approximate test of \
                 homogeneity; frequency weights, level(), Stata 19's notes, r() and errors.",
            ),
            p(
                "Not yet",
                "by() with three or more variables, with value labels, with a control stratum \
                 carrying no information, or with groups without information beside two or \
                 more with it; levels that are not whole. r() can differ from Stata's in the \
                 last digit or two.",
            ),
        ],
    },
    Guide {
        command: "lroc",
        about: "Area under the ROC curve after logit or probit (lroc [depvar], nograph all beta())",
        points: &[
            p(
                "In R",
                "stata_lroc sorts the predicted probabilities (a float, as predict makes them) and \
                 sums the trapezoids under sensitivity against 1 - specificity, each point a float \
                 as in Stata 19, so r(area) carries Stata's residue in its last digits. The rows \
                 are e(sample), or those if and in choose (they replace it), or every row with \
                 all. beta() scores the first equation of a row vector: variables, factor levels \
                 (o. columns add nothing) and _cons, ignoring the fit's weights. The fit's \
                 frequency weights or iweights (which weigh the curve while N counts rows), or \
                 lroc's own; r(N) and r(area), and Stata's errors (after other models, weights, \
                 by, nograph with other options, beta()'s matrix).",
            ),
            p(
                "Graph",
                "Stata draws the ROC curve unless nograph is given; RCraft prints the area and does \
                 not draw the curve. Options that only change its look (titles, labels, scales, \
                 legend, scheme, regions, sizes, nodraw) are taken and change nothing; name(), \
                 saving(), the curve's line and marker options and addplot() are not, and graph \
                 export after lroc is refused.",
            ),
            p(
                "Not yet",
                "lroc after logistic (not translated); bysort and by (...); beta() columns with \
                 interactions or c., or a string variable; negative iweights. matrix colnames with \
                 factor or equation names is refused by the matrix translation, so such columns \
                 reach beta() only from e(b).",
            ),
        ],
    },
    Guide {
        command: "rotate",
        about: "Rotating factor's or pca's loadings (rotate, varimax, quartimax, equamax, promax(#), normalize, blanks(#))",
        points: &[
            p(
                "In R",
                "stata_rotate does what Stata 19 was seen to do: varimax by Kaiser's angle for two \
                 factors and his SVD iteration for more (stopped when the singular values' sum \
                 grows by less than 1e-8 of itself), quartimax and equamax (weight p/2) by the \
                 gradient projection algorithm (stopped when T changes by less than 1e-6 and the \
                 gradient is below 1e-6), promax(k) from the varimax loadings, Kaiser's \
                 normalization with normalize or horst. Each factor is signed so its loadings sum \
                 to more than 0 and the factors are ordered by variance. Stata 19's tables, \
                 blanks(), e(r_L), e(r_T), e(r_Phi), e(r_Ev) and the texts, r(rtext) and Stata's \
                 errors; rotate, clear.",
            ),
            p(
                "Not yet",
                "The other criteria (oblimin, quartimin, parsimax, ...), factors(), random starts, \
                 the maximize options, blanks() below 0, rotate after pca with every component \
                 kept, e(r_fmin), matrix list e(r_Phi) after an orthogonal rotation, and estat \
                 rotatecompare are not translated yet.",
            ),
        ],
    },
    Guide {
        command: "lasso",
        about: "Lasso for linear, logit, probit and Poisson outcomes, elasticnet linear, sqrtlasso, dsregress, poregress, xporegress, dslogit, and lassoselect, lassoknots, lassocoef, lassoinfo, predict",
        points: &[
            p(
                "In R",
                "stata_lasso_run standardises the predictors (divisor N), centres the outcome and \
                 solves the lasso along Stata's grid by cyclic coordinate descent with Stata's \
                 stopping rule (for logit, probit and Poisson by Newton steps with coordinate \
                 descent, to the optimum), then selects lambda as Stata 19 does: none, BIC, \
                 cross-validation (folds from Stata's own random numbers, rseed()), the one-s.e. \
                 rule, adaptive steps or the plug-in penalty, with the deviance stop() and the \
                 'minimum found' rule. elasticnet runs each alpha on Stata's shared grid with the \
                 same folds and picks the alpha and lambda with the smallest BIC or CV error; \
                 sqrtlasso solves the square-root lasso by Stata's own coordinate rule. \
                 dsregress, poregress, xporegress and dslogit run those lassos for the outcome \
                 and each variable of interest, then double selection, partialing out or \
                 cross-fit (Stata's folds and resamples) with Stata's variances. The \
                 log, the table, e() (e(rngstate) included) and e(sample) are Stata's; \
                 lassoknots, lassocoef, lassoinfo and lassoselect print Stata's tables, and \
                 predict gives the penalized or postselection prediction (Pr(y) or the number of \
                 events after logit, probit and Poisson).",
            ),
            p(
                "Differs",
                "Stata's descent stops short of the optimum; for the linear lasso, elasticnet and \
                 sqrtlasso RCraft follows its rule, which reproduces the probes to about 1e-15, but on \
                 some data the L1 and L2 norms at the path's end differ in the 8th digit, and the \
                 plug-in penalty's fit agrees to about 2e-9. For logit, probit and Poisson \
                 Stata's test is not decoded: RCraft solves to the optimum (Stata's own results \
                 at tolerance(1e-12)), so coefficients can differ from Stata's default ones by \
                 about 1e-7 relative, the log likelihood and deviance ratio by about 1e-11. The \
                 standardised constant Stata prints as -1.39e-17 (rounding) is 0 here.",
            ),
            p(
                "Not yet",
                "lasso cox, elasticnet logit, probit and poisson, elasticnet with alpha 0 \
                 (ridge) or several alphas and selection(none), pologit with other than the \
                 plug-in lassos, vce(robust) and no always-in controls, xpologit, dspoisson, \
                 popoisson and xpopoisson, selection(cv) or lasso() inside xporegress, \
                 poregress with vce(cluster), lassocoef after the inference commands, cvplot, \
                 factor variables, \
                 weights, cluster(), tolerance() other than the default and dtolerance() are \
                 refused; so are noconstant with cross-validation and predict after offset(), \
                 exposure() or noconstant for logit, probit and Poisson, and test, lincom and \
                 margins after lasso.",
            ),
        ],
    },
    Guide {
        command: "pdslasso",
        about: "Post-double-selection lasso (pdslasso, SSC, with lassopack's rigorous lasso)",
        points: &[
            p(
                "In R",
                "stata_pdslasso selects controls for the dependent variable and for the regressor \
                 of interest by the rigorous lasso with heteroskedastic loadings (c = 1.1, gamma = \
                 0.1/log(n), two loading passes from OLS on the five most correlated controls), \
                 then fits OLS on their union with robust (HC0) standard errors as e(b) and e(V), \
                 and prints Stata 19's output: the selections, the specification, the CHS and PDS \
                 tables. On ZNMIXH's data it selects what Stata does and matches its estimates.",
            ),
            p(
                "Not yet",
                "Without robust (homoskedastic loadings), several regressors of interest, \
                 instruments, partial(), pnotpen(), clusters, fixed effects, weights and ranges or \
                 patterns in the controls are refused; predict, test and margins after it too.",
            ),
        ],
    },
    Guide {
        command: "sdtest",
        about: "Tests of standard deviations (sdtest y == 1, sdtest y == x, sdtest y, by(g))",
        points: &[
            p(
                "In R",
                "stata_sdtest prints Stata 19's table and tests: one standard deviation against a \
                 value by chi2 = (n - 1)s^2/sd0^2, two (two variables, each over its own rows, or \
                 the two groups of by(), named by their value labels) by the F ratio of their \
                 variances, with the Combined row of every value; the one-sided and two-sided p, \
                 and r() as Stata's (N, df or df_1 and df_2, chi2 or F, sd, sd_1, sd_2, p_l, p_u, p).",
            ),
            p(
                "Not yet",
                "Options other than by() and level() are refused; weights are Stata's own error.",
            ),
        ],
    },
    Guide {
        command: "sdtesti",
        about: "sdtest from numbers typed in (sdtesti 50 . .9 1.5, sdtesti 30 2 1.3 25 1 .7)",
        points: &[p(
            "In R",
            "As sdtest's, from #obs #mean #sd and a value, or two groups' #obs #mean #sd (named x \
             and y, their Combined row pooled from them); a mean of . is shown as ., as in Stata.",
        )],
    },
    Guide {
        command: "cs",
        about: "Risk table of a cohort study (cs case exposed, csi 20 15 80 120)",
        points: &[
            p(
                "In R",
                "stata_epi prints Stata 19's table: cases and noncases of the exposed and the \
                 unexposed (a nonzero value counts as one; the exposure's label heads the \
                 columns), the risks, the risk difference with its Wald interval, the risk ratio \
                 with its log interval, the attributable fraction among the exposed and in the \
                 population (prevented fractions when the ratio is below 1), Pearson's chi2, or \
                 with exact Fisher's one- and two-sided p; r() as Stata's.",
            ),
            p(
                "Strata",
                "stata_epi_by prints each stratum's risk ratio with its log interval and \
                 Mantel-Haenszel weight, the crude ratio, the Mantel-Haenszel combined one with \
                 Greenland and Robins's interval and the test of homogeneity (worked out in float \
                 precision, as Stata keeps it); istandard the internally standardized ratio (the \
                 exposed as the standard); or woolf the odds ratios with Woolf's intervals, the \
                 Robins-Breslow-Greenland interval of the combined one and the Mantel-Haenszel \
                 test. Strata are named by their value labels.",
            ),
            p(
                "Not yet",
                "or (its Cornfield interval comes from Stata's own search, which R's root of \
                 Cornfield's equation matches only to about 5 digits; with by() or woolf is \
                 translated), rd, binomial(), estandard, ustandard(), weights and the other \
                 options are refused.",
            ),
        ],
    },
    Guide {
        command: "csi",
        about: "cs from the four counts typed in (csi #a #b #c #d)",
        points: &[p(
            "In R",
            "As cs's, from the exposed and unexposed cases and noncases.",
        )],
    },
    Guide {
        command: "cc",
        about: "Odds table of a case-control study (cc case exposed, woolf)",
        points: &[
            p(
                "In R",
                "With woolf or tb: stata_epi prints Stata 19's table of cases and controls with \
                 the proportion exposed, the odds ratio with Woolf's interval or the test-based \
                 one (on the Mantel-Haenszel chi2, (N - 1)/N Pearson's, as Stata 19 takes it), \
                 the attributable or prevented fractions and Pearson's chi2; r() as Stata's. \
                 by() (with woolf or tb): each stratum's odds ratio and Mantel-Haenszel weight, \
                 the crude and the Mantel-Haenszel combined ratio (Robins-Breslow-Greenland's \
                 interval, or test-based on the Mantel-Haenszel chi2), the test of homogeneity \
                 and the Mantel-Haenszel test.",
            ),
            p(
                "Not yet",
                "cc without woolf or tb: its default exact interval of the odds ratio comes from \
                 Stata's own search, which stops short of the exact bound (by about 1e-4), so it \
                 is refused (with by() too), as are cornfield, exact, bd, tarone and weights.",
            ),
        ],
    },
    Guide {
        command: "cci",
        about: "cc from the four counts typed in (cci #a #b #c #d, woolf)",
        points: &[p(
            "In R",
            "As cc's, from the exposed and unexposed cases and controls; woolf or tb is needed, \
             as for cc.",
        )],
    },
    Guide {
        command: "ir",
        about: "Incidence-rate table (ir case exposed time, iri 20 15 2000 3500)",
        points: &[p(
            "In R",
            "stata_epi prints Stata 19's table: cases and person-time of the exposed and the \
             unexposed (rows named by the variables' labels), the rates, the rate difference \
             with its Wald interval, the rate ratio with the exact interval (R's poisson.test, \
             the same as Stata's), the attributable or prevented fractions, and the mid-p and \
             exact tests; r() as Stata's. by(): each stratum's rate ratio with its exact \
             interval and Mantel-Haenszel weight, the crude ratio, the Mantel-Haenszel combined \
             one (Greenland and Robins's interval), the pooled (inverse-variance) one in r() and \
             the test of homogeneity. ird, tb and the other options are refused.",
        )],
    },
    Guide {
        command: "tabodds",
        about: "Odds of failure by level of an exposure (tabodds case dose)",
        points: &[
            p(
                "In R",
                "stata_tabodds prints Stata 19's table of cases, controls and odds with Woolf's \
                 interval for each level (named by its value label, abbreviated to 9), the test \
                 of equal odds ((N - 1)/N Pearson's chi2) and the score test for trend on the \
                 levels' values; r() as Stata's. or gives each level's Mantel-Haenszel odds \
                 ratio against base()'s level (its place, 1 to k), with its chi2 and the interval \
                 on V/(R*S), as mhodds does; frequency weights count. adjust() gives the same \
                 across the adjusting variables' strata with the stratified score test for trend; \
                 binomial(n) reads case as the cases of n trials on each record.",
            ),
            p(
                "Not yet",
                "The graphs (a later step), and binomial() counts that are missing, negative, not \
                 whole or above the trials. r() can differ from Stata's in the last digit or two.",
            ),
        ],
    },
    Guide {
        command: "iri",
        about: "ir from the counts typed in (iri #a #b #t1 #t0)",
        points: &[p(
            "In R",
            "As ir's, from the exposed and unexposed cases and person-time.",
        )],
    },
    Guide {
        command: "power",
        about: "Power and sample size (power onemean 0 0.5, power twoproportions 0.3 0.5, n(150))",
        points: &[
            p(
                "In R",
                "stata_power works out the sample size, the power, or (with one value, n() and \
                 power()) the target value for onemean, twomeans, pairedmeans, oneproportion, \
                 twoproportions, onevariance, twovariances, onecorrelation, twocorrelations, \
                 pairedproportions, oneway, logrank, cox, exponential, trend, oneslope, rsquared and pcorr, from the [PSS] manual's power \
                 functions (logrank: Freedman's or Schoenfeld's events; cox: Hsieh and Lavori's): the \
                 noncentral t (or z with knownsd), Satterthwaite's t for unequal sds, the score or \
                 Wald test of a proportion, the pooled test of two with Fleiss's continuity \
                 correction, the chi-squared test of a variance, the F test of two, Fisher's z test \
                 of correlations, the large-sample McNemar test and the F test of a group effect \
                 (its noncentral F summed to full precision; R's pf() agrees to about 1e-9). Sample sizes are the smallest whole numbers \
                 reaching the power (the second group nratio() times the first, rounded up); a \
                 total that does not split is cut to Stata's actual sample sizes. Stata 19's \
                 printout and r(), but for its search's own r(init), r(iter), r(deltax) and \
                 r(function).",
            ),
            p(
                "As Stata",
                "Two variances are tested with F on n1 - 1 and n2 - 1 degrees of freedom, scaled by \
                 v2/v1, as Stata 19 does. One-sided, pairedproportions' power is the upper tail even \
                 when p21 is below p12 (its sample size uses the size of the difference), as in \
                 Stata 19.",
            ),
            p(
                "Digits",
                "R's noncentral t, chi-squared and F and its root finding agree with Stata's to \
                 about 1e-12; what is printed is the same.",
            ),
            p(
                "Not yet",
                "Graphs, parallel, table() with columns, return list after power oneway (Stata \
                 lists its r() in an order that changes with the number of groups), oneway's \
                 lists and contrasts, logrank's and cox's lists, wdprob() and simulation, the \
                 other methods (twoway, repeated, mcc, cmh, \
                 ...), the exact tests' sample size and effect size (Stata refuses them too), \
                 one group's size that leaves the other not whole, \
                 pairedproportions' corr(), diff() and ratio() \
                 forms, and the other options are refused.",
            ),
            p(
                "Tables",
                "Lists of values print Stata's table, each row worked out as its own power line, in \
                 Stata's order; r() is the last row's, with r(columns), r(labels), r(widths), \
                 r(formats) and r(pss_table). return list after a table is refused: Stata lists \
                 the table's r(separator) and r(divider) in an order RCraft does not follow.",
            ),
            p(
                "As Stata's limits",
                "Stata's noncentral t takes noncentralities up to 1000: past them two means stop \
                 with r(498) (in a table, a missing power and Stata's warning) and one mean shows a \
                 missing power, in RCraft as in Stata 19. After cox given b1 or worked out, \
                 r(hratio) can be read but return list leaves it out, as Stata keeps it. \
                 test(binomial) and test(fisher) give the power and the actual alpha only (their \
                 power is saw-toothed in n, so Stata offers no sample size or effect size); Fisher's \
                 two-sided p-value is twice the smaller tail.",
            ),
        ],
    },
    Guide {
        command: "mvtest",
        about: "Multivariate tests (mvtest means x1 x2 x3, by(g); mvtest normality x1 x2, stats(all))",
        points: &[
            p(
                "In R",
                "stata_mvtest works out Hotelling's T2 that the means are equal (or zero), the \
                 four MANOVA statistics by() with Rao's, Pillai's, Lawley-Hotelling's and Roy's F, \
                 the LR test allowing for heterogeneity (the common mean by maximum likelihood), \
                 Box's M with its F and chi2, the adjusted LR tests that the covariance matrix is \
                 diagonal, spherical or compound symmetric, Lawley's and Jennrich's tests of \
                 correlations, and the Doornik-Hansen, Henze-Zirkler and Mardia tests of \
                 normality with the univariate and bivariate tables. Rows with any of the \
                 variables missing are left out; missing makes the missing by() values a group. \
                 Stata 19's printout, its errors (r(198), and r(506) for a singular matrix) and \
                 r(), matrices included.",
            ),
            p(
                "Digits",
                "The statistics agree with Stata's to about 1e-13; r(M) after lr, the common \
                 mean, about 1e-10 (Stata stops its iterations sooner).",
            ),
            p(
                "Not yet",
                "heterogeneous (for more than two groups Stata takes the p-value of James' \
                 approximation from its own search, a few 1e-6 off the root), protect(), \
                 weights, missing without by(), fewer rows than contrasts in means, and \
                 matrix list r(Btest) (its rows are named by pairs) are refused.",
            ),
        ],
    },
    Guide {
        command: "ranksum",
        about: "Wilcoxon rank-sum (Mann-Whitney) test (ranksum x, by(g))",
        points: &[
            p(
                "In R",
                "stata_ranksum prints Stata 19's table of the two groups' observed and expected \
                 rank sums (named by their value labels), the unadjusted variance, the ties' \
                 adjustment and the adjusted variance, the H0 line, z and Prob > |z|, and the \
                 exact p when there are 200 observations or fewer, worked out over every way of \
                 sharing the midranks between the groups; r() as Stata's (N_1, N_2, z, Var_a, \
                 sum_obs, sum_exp, p, p_l, p_u, p_exact, p_l_exact, p_u_exact).",
            ),
            p(
                "Not yet",
                "Options other than by() (porder, exact) and weights are refused.",
            ),
        ],
    },
    Guide {
        command: "signrank",
        about: "Wilcoxon matched-pairs signed-rank test (signrank x = w, signrank x = 0)",
        points: &[
            p(
                "In R",
                "stata_signrank prints Stata 19's table (positive, negative and zero differences \
                 with their observed and expected rank sums), the unadjusted variance with the \
                 ties' and the zeros' adjustments, z, Prob > |z| and the exact p (over every sign \
                 of the nonzero ranks); r() as Stata's.",
            ),
            p(
                "Not yet",
                "Options, weights, and an expression on the right other than a variable or a \
                 number are refused.",
            ),
        ],
    },
    Guide {
        command: "signtest",
        about: "Sign test of matched pairs (signtest x = w, signtest x = 0)",
        points: &[p(
            "In R",
            "stata_signtest prints Stata 19's table of positive, negative and zero signs, and the \
             one-sided and two-sided binomial tests with their hypotheses; r() as Stata's \
             (N_pos, N_neg, N_tie, p, p_l, p_u).",
        )],
    },
    Guide {
        command: "kwallis",
        about: "Kruskal-Wallis equality-of-populations rank test (kwallis x, by(g))",
        points: &[
            p(
                "In R",
                "stata_kwallis prints Stata 19's table of each group's observations and rank sum, \
                 chi2 with its degrees of freedom and probability, and chi2 with the ties' \
                 correction; r() as Stata's (df, chi2, p, chi2_adj, p_adj). chi2 agrees with \
                 Stata's to about 2e-7.",
            ),
            p("Not yet", "Weights are refused."),
        ],
    },
    Guide {
        command: "ktau",
        about: "Kendall's rank correlations (ktau x y, ktau x y z, stats(taub p))",
        points: &[
            p(
                "In R",
                "stata_ktau prints Stata 19's two-variable report (tau-a, tau-b, the score, its \
                 standard error corrected for ties and the continuity-corrected p) or, for three \
                 or more (or with matrix), the lower triangle of tau-a or of the stats() asked \
                 for, with its Key, print() and star(), listwise or pw, Bonferroni's or Sidak's \
                 adjustment; r() as Stata's, the matrices included. As Stata 19 counts them, the \
                 score takes values that agree to 15 significant digits as tied, the ties' \
                 corrections only values exactly equal.",
            ),
            p(
                "Digits",
                "p-values near 1e-14 differ in their second digit (Stata's normal tail is \
                 less precise there); what is printed is the same.",
            ),
            p("Not yet", "Weights are refused."),
        ],
    },
    Guide {
        command: "spearman",
        about: "Spearman's rank correlations (spearman x w z)",
        points: &[
            p(
                "In R",
                "With three or more variables, the matrix of rank correlations as Stata 19 \
                 prints it: the lower triangle, over the rows with none of them missing.",
            ),
            p(
                "Not yet",
                "Two variables are refused: Stata 19's p-value comes from its own approximation \
                 to the permutation distribution, not R's, and is not matched yet. The options \
                 (stats(), pw, star()), and r() after the matrix, too.",
            ),
        ],
    },
    Guide {
        command: "sktest",
        about: "Skewness and kurtosis tests for normality (sktest x, noadjust)",
        points: &[
            p(
                "In R",
                "stata_sktest computes D'Agostino, Belanger and D'Agostino's skewness and \
                 kurtosis tests from summarize's skewness and kurtosis (with frequency or analytic \
                 weights), and the joint chi2 with Royston's adjustment unless noadjust, as \
                 Stata's manual gives them; Stata 19's table and r() (N, p_skew, p_kurt, chi2, \
                 p_chi2 and r(table)). Fewer than 8 observations give missing tests, as in \
                 Stata.",
            ),
            p(
                "Not yet",
                "Patterns in the varlist that the data's names do not settle are refused.",
            ),
        ],
    },
    Guide {
        command: "nl",
        about: "Nonlinear least squares (nl (y = {b0 = 1} * exp({b1} * x)))",
        points: &[
            p(
                "In R",
                "stata_nl fits it as Stata 19 does, worked out from its output on probes: the \
                 starting values as %9.0g writes them, Gauss-Newton with exact derivatives and \
                 ml's step search, ml's stopping rule on the residual SS, so its iteration log \
                 and e(ic) are Stata's; the parameters in byte order (Mid before alpha); a \
                 parameter whose derivative is the same on every row taken as the constant \
                 (centered R2, Stata's note); e(V) s2 (J'J)^-1, t tests on N - k.",
            ),
            p(
                "Not yet",
                "The function-program form, linear combinations ({xb: x1 x2}), weights, vce(), \
                 hasconstant(), initial() and the other options, and predict after nl.",
            ),
        ],
    },
    Guide {
        command: "nlcom",
        about: "Nonlinear combinations of the coefficients (nlcom _b[x]/_b[z], nlcom (name: exp) ...)",
        points: &[
            p(
                "In R",
                "stata_nlcom works each expression out at the coefficients and its standard error \
                 by the delta method, with the derivatives taken by central differences \
                 (extrapolated) on Stata mode's own translation of the expression; z and the \
                 normal interval, as Stata's, after regress too. The table and r() are Stata \
                 19's: r(b), r(V), r(table), r(N), r(level).",
            ),
            p(
                "post",
                "The combinations become e(b) and e(V) under their names (r, _nl_1), with Stata \
                 19's e(N), e(rank), e(cmd) nlcom, e(depvar), e(predict) and e(properties); test, \
                 lincom and nlcom then use them, with z.",
            ),
            p(
                "Not yet",
                "nlcom after models other than regress, logit, probit, poisson, nbreg, ologit, \
                 oprobit, xtreg, ivregress, clogit, stcox and mlogit ([2]_b[x] for an equation) \
                 or nlcom's own posted results (after margins, post it is refused), and \
                 expressions of the data's variables are refused. After the likelihood models the \
                 results are as close to Stata's as the model's own estimates (6 digits or more).",
            ),
        ],
    },
    Guide {
        command: "testnl",
        about: "Wald test of nonlinear restrictions (testnl _b[x]/_b[z] = 1, testnl (exp = exp) (exp = exp))",
        points: &[
            p(
                "In R",
                "stata_testnl lists the equations, then Wald's chi2 of g(b) = 0 by the delta \
                 method, its derivatives taken as nlcom's; after regress too it is chi2, as \
                 Stata's. mtest prints each equation's test as well, unadjusted or Bonferroni's \
                 (r(mtest) then gains adjustp). r() as Stata 19's: p, chi2, df, mtmethod, \
                 r(mtest), r(G) (the derivatives, in Stata's order with _cons last) and r(R). \
                 The derivatives agree with Stata's own to about 1e-11.",
            ),
            p(
                "Not yet",
                "mtest's Holm and Sidak adjustments, testnl after the models nlcom refuses, and \
                 expressions of the data's variables are refused.",
            ),
        ],
    },
    Guide {
        command: "centile",
        about: "Centiles with confidence intervals (centile x, centile(25 50 75))",
        points: &[
            p(
                "In R",
                "stata_centile gives Stata 19's centiles, at rank (n + 1)q/100 between the order \
                 statistics (R's quantile type 6), and its default interval: where the binomial \
                 cdf passes each tail, interpolated between the order statistics, held at the \
                 sample's minimum or maximum (and marked) where that order statistic is not \
                 there. Each variable uses its own rows; r() is the last variable's (N, n_cent, \
                 c_#, lb_#, ub_#, centiles).",
            ),
            p(
                "Other intervals",
                "cci gives the conservative interval (the order statistics themselves), normal \
                 the centile's standard error from a normal density at it, meansd the centile of \
                 a normal with the sample's mean and standard deviation, all as Stata's manual \
                 gives them and checked against Stata 19.",
            ),
            p(
                "Not yet",
                "centile() lists that are not plain numbers are refused; weights are Stata's own \
                 error (weights not allowed, r(101)).",
            ),
        ],
    },
    Guide {
        command: "cii",
        about: "Confidence intervals from numbers typed in (cii means 50 10 3, cii proportions 60 18)",
        points: &[
            p(
                "In R",
                "stata_cii works ci's intervals out from the numbers typed (cii means #obs #mean \
                 #sd, cii means #exposure #events, poisson, cii proportions #obs #succ with each \
                 method, cii variances #obs #variance [#kurtosis] with sd and bonett) and prints \
                 Stata 19's table with an empty Variable column; where an interval was cut at 0 or \
                 1 it says so as Stata's cii does (Note: The Wald CI was truncated at 0 at the lower \
                 endpoint.). r() is ci's.",
            ),
            p(
                "Not yet",
                "Values that are not numbers written out are refused. A proportion or rate typed in \
                 place of the count (cii proportions 60 .31) is the count Stata 19 takes from it, \
                 rounded half up (19).",
            ),
        ],
    },
    Guide {
        command: "ci",
        about: "Confidence intervals for means, proportions and variances (ci means x, ci proportions b, ci variances x)",
        points: &[
            p(
                "In R",
                "stata_ci prints Stata 19's table and leaves the last variable's r(). Means: the \
                 t interval, with frequency and analytic weights as summarize takes them. \
                 Proportions: the exact (Clopper-Pearson, from qbeta), Wald, Wilson, \
                 Agresti-Coull and Jeffreys intervals, with Stata's marks and notes where an \
                 interval is one-sided or clipped at 0 or 1, and only the 0/1 variables (a note \
                 says so). Variances: the chi2 interval, or Bonett's (bonett: the kurtosis about \
                 a trimmed mean, as Stata's manual gives it), and with sd the standard deviation. \
                 Means, poisson (or exposure()): the exact Poisson interval of the total, which \
                 Stata rounds to a whole number, as rates over the exposure.",
            ),
            p(
                "Not yet",
                "Patterns in the varlist and exposure() with weights are refused. Frequency \
                 weights are taken everywhere and analytic ones for normal means; analytic weights \
                 elsewhere are Stata's own error (aweights not allowed, r(101)). cii has a page of \
                 its own.",
            ),
        ],
    },
    Guide {
        command: "mean",
        about: "Means with their standard errors (mean y x, over(g))",
        points: &[
            p(
                "In R",
                "stata_meanest estimates as Stata 19 does and prints its table: the rows with \
                 every variable (and over()'s) present, each mean's standard error s/sqrt(n) \
                 with frequency weights counting rows and analytic ones scaled to the rows (in \
                 each group of over()), and with probability weights the linearized (sandwich) \
                 variance n/(n-1) sum z z', each group of over() on its own. e(b) and e(V) are \
                 named as Stata names them (y, c.y@0bn.b), with e(N), e(df_r), e(sd), e(_N) and \
                 r(table); test and lincom read them, their lines naming the levels as Stata's \
                 do (c.y@0bn.b - c.y@1.b = 0).",
            ),
            p(
                "Not yet",
                "vce() and the display options are refused. Several variables in over() make \
                 cells of their values (c.y@1bn.g#0bn.b), iweights count as frequency weights, as \
                 Stata 19's, and _b[] and _se[] read over()'s names (_b[c.y@0.b], with or without \
                 bn).",
            ),
        ],
    },
    Guide {
        command: "total",
        about: "Totals with their standard errors (total y, over(g))",
        points: &[p(
            "In R",
            "As mean's (stata_meanest), each total the sum of the weights times the mean, its \
             variance that sum squared times the mean's, as Stata 19 gives it; frequency and \
             probability weights (iweights as frequency weights), over(); analytic weights are \
             Stata's error (aweights not allowed).",
        )],
    },
    Guide {
        command: "proportion",
        about: "Proportions of each value with logit intervals (proportion g, over(h))",
        points: &[p(
            "In R",
            "stata_meanest gives each value's share of the rows (every variable present), its \
             variance (diag(p) - pp')/n, and Stata 19's logit interval; with probability weights \
             the linearized variance; over() each group on its own. e(b) is named 1bn.g 2.g ... \
             (0bn.b@1bn.g with over()), with e(freq) and e(_N), and lincom's lines name the \
             levels so (1bn.g - 2.g = 0). prop is proportion, as in Stata (propo is not); \
             analytic weights are Stata's error (aweights not allowed), and iweights count as \
             frequency weights.",
        )],
    },
    Guide {
        command: "ratio",
        about: "Ratios of means with linearized standard errors (ratio y/x, ratio (r: y/x))",
        points: &[p(
            "In R",
            "stata_meanest gives sum w y / sum w x and its linearized variance, n/(n-1) sum z z' \
             with z = w (y - R x) / sum w x, each group of over() on its own, as Stata 19's \
             (frequency weights counting rows); named ratios and _ratio_#, the lines naming \
             each ratio before the table. Analytic weights are Stata's error (aweights not \
             allowed); iweights are refused.",
        )],
    },
    Guide {
        command: "oneway",
        about: "One-way analysis of variance (oneway y g, tabulate)",
        points: &[
            p(
                "In R",
                "stata_oneway prints Stata 19's tables: the groups' means, standard deviations and \
                 counts with tabulate (labels cut at 9), the analysis of variance (between and \
                 within groups, F and its p-value), and Bartlett's test of equal variances on the \
                 groups with a positive variance, with Stata's note when single-observation or \
                 equal-valued groups are left out; the Bonferroni, Scheffe and Sidak comparisons \
                 of each pair of means; frequency and analytic weights (with analytic weights \
                 Bartlett's test counts each group's rows, as Stata 19 does). nomeans, nostandard \
                 and nofreq drop the summary table's columns, noobs its Obs column; missing makes \
                 the rows with a missing group a group of their own (.), and wrap changes nothing \
                 here. r() is Stata's: N, mss, df_m, rss, df_r, F, chi2bart, df_bart and the \
                 matrix r(ANOVA).",
            ),
            p(
                "Not yet",
                "The comparisons of more groups than the line holds are refused. With weights and tabulate the summary table gains Stata's Obs column \
                 (analytic weights: the weight sums as Freq., the rows as Obs, summarize's standard \
                 deviation).",
            ),
        ],
    },
    Guide {
        command: "testparm",
        about: "Joint test of the coefficients a varlist names (testparm varlist, equal)",
        points: &[
            p(
                "In R",
                "stata_testparm finds the model's coefficients whose terms the varlist matches \
                 (i.g, c.x, x, i.g#c.x in either order, i.g##i.h as its main effects and \
                 interaction, x* against the data's names), base levels left out, in e(b)'s order, \
                 and runs test's Wald test on them: F after regress, xtreg fe and ivregress, \
                 small; chi2 after xtreg re, ivregress and stcox, and after logit, probit, poisson, \
                 nbreg, ologit, oprobit and clogit, whose lines name the equation ([y]x) as \
                 Stata's do. equal tests each against the first (- x1 + x2 = 0). r() is test's.",
            ),
            p(
                "Not yet",
                "testparm after the other models, interactions of two continuous variables after \
                 mlogit (after mlogit each coefficient is tested in every equation, the base \
                 outcome's dropped, as Stata does, factor-by-continuous interactions too), a level \
                 of its own (2.g), lists in parentheses (i.(g h)) and the options other than equal \
                 are refused. No \
                 coefficient matched is Stata's r(111), equal with one coefficient r(102).",
            ),
        ],
    },
    Guide {
        command: "tabstat",
        about: "Table of summary statistics (tabstat varlist, statistics() by())",
        points: &[
            p(
                "In R",
                "stata_tabstat computes summarize's statistics (Stata's percentile rule, its \
                 frequency and analytic weights, skewness and kurtosis over the whole sample) and \
                 prints Stata 19's table: 10-wide columns in %9.0g or format(), the statistics' or \
                 variables' stub, the groups by their value labels cut to labelwidth(), rules \
                 between groups of more than one row, the table cut into blocks at the line size. \
                 Rows with a missing group are left out of the groups and the total unless \
                 missing; each variable uses its own rows unless casewise. save leaves r() as \
                 Stata's: StatTotal, Stat1, ... (statistics by variables) and name1, ...",
            ),
            p(
                "Not quite",
                "A string group's stub is as wide as its longest value; Stata's is the string's \
                 storage width (str#), wider when the variable was made wider than its values.",
            ),
            p(
                "Not yet",
                "format alone takes each variable's own display format: one set by format, read \
                 with the data from a .dta, or the default of the storage type generate made \
                 (float %9.0g, double %10.0g, long %12.0g); a variable whose format is none of \
                 these stops with that reason. Formats wider than 30, widths above 32 \
                 and patterns in the varlist that the data's names do not settle are refused; \
                 an unknown statistic, a string variable and pweights or iweights are Stata's \
                 errors (r(198), r(109), r(101)).",
            ),
        ],
    },
    Guide {
        command: "prtest",
        about: "Two-sample test of proportions (prtest var, by(g))",
        points: &[
            p(
                "In R",
                "stata_prtest prints Stata 19's table: each group's proportion with its standard \
                 error sqrt(p(1 - p)/n) and normal interval, the difference with sqrt(se1^2 + \
                 se2^2), the z test under H0 with the pooled proportion and its three p-values; \
                 the groups in sorted order, named by their value labels cut at 12 characters; \
                 r() as Stata's (N1 ... p_u, level). Other than two groups is r(450), a mean \
                 outside [0,1] r(198).",
            ),
            p(
                "Not yet",
                "The one-sample form (prtest x == #), two variables (prtest x == y) and prtesti \
                 are refused.",
            ),
        ],
    },
    Guide {
        command: "anovaplot",
        about: "Fitted values after anova, by two factors (anovaplot, Stata Journal gr0009_2)",
        points: &[
            p(
                "In R",
                "After anova, the model's fitted values over its own rows, joined along the first \
                 variable for each value of the second (marked at each cell, labelled with its \
                 value label), the data as points (hidden by scatter(ms(i)), which still set the \
                 y axis), the first variable's values as ticks with their labels, the titles \
                 Stata 19 gives, and Profiles by the second variable above; drawn by ggplot2.",
            ),
            p(
                "Not yet",
                "anovaplot after other models, of variables that are not the model's categorical \
                 terms, with if or in, or after the data changed are refused.",
            ),
        ],
    },
    Guide {
        command: "table",
        about: "Tables of frequencies and statistics (Stata 17's table)",
        points: &[
            p(
                "In R",
                "stata_table lays the table out as Stata 19 does: rows (one variable, or two \
                 nested) and at most one column variable, frequency (Sum of weights with \
                 fweights), percent, mean, sd, median and p#, min, max, sum, count and iqr, each \
                 column as wide as its widest entry, a blank cell where there are no rows and . \
                 where there are no values, totals unless nototals, missing as a level of its \
                 own, nformat() for some statistics or all.",
            ),
            p(
                "Not yet",
                "A third dimension, several statistics of several variables, and the options for \
                 styles, totals() and the collect system are refused; contents() is Stata's own \
                 error since Stata 17.",
            ),
        ],
    },
    Guide {
        command: "tabm",
        about: "One table of several variables' values (tabm, tab_chi from SSC)",
        points: &[
            p(
                "In R",
                "stata_tabm stacks the variables and prints tabulate's table of variable (a row \
                 each, in the varlist's order) by values (a column each, labelled with the first \
                 variable's value label, cut at 9 characters), split into blocks as Stata 19 does; \
                 if and missing; r(N), r(r) and r(c).",
            ),
            p(
                "Not yet",
                "in, weights, string variables, missing with .a to .z in the data, and tabm's \
                 other options are refused.",
            ),
        ],
    },
    Guide {
        command: "revrs",
        about: "Reverses a variable's coding (revrs, SSC)",
        points: &[
            p(
                "In R",
                "stata_revrs does what Stata 19 does with revrs, matched to its output: with a \
                 value label, the labelled values in the data take the codes k to 1 in order and \
                 the label set rev<name> keeps their texts (a value without a label is missing); \
                 without one, (max + 1) - x with missing values kept. rev<name> is new, or with \
                 replace the variable itself. Stata's errors come through capture as r(111), \
                 r(109) and r(110).",
            ),
            p(
                "Not yet",
                "Values that are not whole numbers or halves (Stata's float arithmetic there was \
                 not matched), value labels that name missing values, if and in are refused.",
            ),
        ],
    },
    Guide {
        command: "frame",
        about: "Several datasets at once (frame create, change, drop, pwf)",
        points: &[
            p(
                "In R",
                "Each frame's data is kept in the list stata_frames while another is current; \
                 frame change puts the current one there and takes the other out, so each keeps \
                 its own data, labels, settings and pending preserve, and e() stays, as in Stata \
                 19. A frame that is \
                 there already is r(110), one that is not r(111), dropping the current one r(119), \
                 each stopping in R so capture gives Stata's _rc.",
            ),
            p(
                "Not yet",
                "frame name: command, frame copy, put, rename, dir, frlink and frget are refused.",
            ),
        ],
    },
    Guide {
        command: "which",
        about: "Whether a command is installed (capture which, as do-files check their add-ons)",
        points: &[
            p(
                "In R",
                "capture which name (or quietly) sets _rc as Stata 19 does, 0 when found and 111 \
                 when not, printing nothing. Found means RCraft translates the command (the estout \
                 package for its esttab, eststo, estadd and estpost); an add-on it does not \
                 translate counts as not installed, so a do-file's own install step runs as on a \
                 machine without it.",
            ),
            p(
                "Not yet",
                "which without capture or quietly (it lists the command's file and its version \
                 lines), several names and options are refused.",
            ),
        ],
    },
    Guide {
        command: "estpost",
        about: "Posts summary statistics for esttab (estpost summarize, the SSC estout package)",
        points: &[
            p(
                "In R",
                "stata_estpost_summarize posts e(count), e(sum_w), e(mean), e(Var), e(sd), e(min), \
                 e(max) and e(sum) for each variable, e(N) (the rows where any is nonmissing) and \
                 e(k), and prints its table as Stata 19 does, split to the linesize.",
            ),
            p(
                "Not yet",
                "Its other subcommands (tabulate, correlate, ttest, ...), weights, options and \
                 string variables are refused.",
            ),
        ],
    },
    Guide {
        command: "eststo",
        about: "Stores results and adds to them (eststo and estadd, the SSC estout package)",
        points: &[
            p(
                "In R",
                "eststo keeps the active results as estimates store does (stata_stored), under its \
                 name or est1, est2, ... by position, with e(_estimates_name); eststo name: \
                 command fits and then stores. eststo clear and drop work as estimates clear and \
                 drop.",
            ),
            p(
                "estadd",
                "estadd scalar name = exp and estadd local name text add to e() (stata_e), and to \
                 the stored copy when the active results are stored, by eststo or estimates store, \
                 as the package does.",
            ),
            p(
                "Not yet",
                "esttab and estout, which write the tables; the other estadd subcommands \
                 (ysumm, matrix, ...) and eststo's other options.",
            ),
        ],
    },
    Guide {
        command: "fre",
        about: "Frequency tables (the SSC package)",
        points: &[
            p(
                "In R",
                "stata_fre prints fre's table as the package prints it, matched to its output \
                 (not its code): the variable's name and label, each value with its value label, \
                 Valid, Missing and Total rows, Freq., Percent, Valid and Cum.; a string's empty \
                 value is missing. It leaves r(N), r(N_valid), r(N_missing), r(r), r(r_valid), \
                 r(r_missing) and r(depvar).",
            ),
            p(
                "Missing values",
                "Stata's extended missing values (.a to .z) are one missing value in R, so they \
                 are one Missing row.",
            ),
            p("Not yet", "fre's options and weights; r(lab_valid) and the matrices."),
        ],
    },
    Guide {
        command: "outreg2",
        about: "Writes a table of models to a file (the SSC package)",
        points: &[
            p(
                "In R",
                "stata_outreg2 writes the same .txt, .xls and .tex files as the package, byte for \
                 byte, matched to the files it writes (not its code), for tstat or se with bdec(#) \
                 tdec(#), parenthesis(), adjr2, r2, keep() (a##b included) and e(); tex(fragment); \
                 label (a LABELS column, the labels in the .tex); ctitle(); addtext() and \
                 addstat(): a column per model added to the table already in f.txt unless \
                 replace, a variable a later model adds placed before Constant, the statistics \
                 rows in Stata's order (a column alone in its model's order, xtreg's Number of \
                 panels included; Observations and R-squared first once columns are added; e() \
                 rows sorted by name), standard errors with 3 decimals, Robust in the footer \
                 after a robust or clustered first column.",
            ),
            p(
                "Not yet",
                "outreg2 without tstat or se, bdec() and tdec() (its own number format), label \
                 with an .xls file, other files (.doc), stored models by name, or after factor \
                 variables that keep() does not leave out.",
            ),
        ],
    },
    Guide {
        command: "parmest",
        about: "Saves the estimates as a dataset (the SSC package)",
        points: &[
            p(
                "In R",
                "stata_parmest saves the dataset as the package does, matched to the files it \
                 saves (not its code): one row per column of e(b), with idnum, eq (when e(b) has \
                 equation names), parm, estimate, stderr, z, p, min95 and max95 and its labels; a \
                 base level is 0 with z and p missing. The data in memory stay as they are.",
            ),
            p(
                "Not yet",
                "parmest without saving(), after a model with t tests, and its other options.",
            ),
        ],
    },
    Guide {
        command: "reghdfe",
        about: "Regression absorbing fixed effects (the SSC package)",
        points: &[
            p(
                "In R",
                "stata_reghdfe fits reghdfe as the package does, matched to its results (its code \
                 was not read): singletons dropped (again until none are left) unless \
                 keepsingletons, the absorbed means taken out, _cons with the grand means added \
                 back, and its degrees of freedom: an absorbed variable counts its levels, the \
                 second one less one for each connected group the two make, and one nested in a \
                 cluster variable counts none.",
            ),
            p(
                "Errors",
                "Unadjusted, vce(robust) (times N / (N - K)), clustered (times (N - 1) / (N - K) \
                 G / (G - 1), t tests on G - 1) and two-way clustered (the two less their \
                 intersection, G the smaller count, with Cameron, Gelbach and Miller's fix when \
                 that is not positive). As reghdfe makes the fix: on the slopes alone, each in \
                 units of its standard deviation, and the constant's row from the fix of the \
                 whole matrix; F is missing when the slopes' variance falls short of full rank, \
                 and a variance with a covariance larger than its variances allow is posted as \
                 zero (errors missing), as Stata 19 does.",
            ),
            p(
                "Rounding",
                "With one slope and the fix applied, Stata posts e(V) or zeroes it as its own \
                 rounding of a correlation of exactly 1 falls (22 of 30 such models kept their \
                 errors in one replication package); RCraft always keeps them, so Stata may show \
                 missing errors where RCraft shows numbers.",
            ),
            p(
                "Not yet",
                "A third absorbed variable not nested in a cluster variable, interactions in absorb(), savefe and resid, \
                 weights, and predict after reghdfe.",
            ),
        ],
    },
    Guide {
        command: "robreg",
        about: "Robust regression (the SSC package): robreg m",
        points: &[
            p(
                "In R",
                "stata_robreg fits robreg m as the package does, matched to its results (its code \
                 was not read): least absolute deviations for the start, the scale from those \
                 residuals (their normalized median, the fit's p zero residuals left out), then \
                 Huber's M estimate with that scale fixed (k = 1.3449975 for 95% efficiency), \
                 robust standard errors with t tests on n - p degrees of freedom.",
            ),
            p(
                "predict",
                "xb (the default), residuals and weights (the Huber weights).",
            ),
            p(
                "Indicator variables",
                "When the LAD starting fit is not unique, as indicator variables often make it, \
                 robreg starts from a point inside the LAD solutions; RCraft finds the same one \
                 with quantreg's interior-point solver (method fn), so the scale and the \
                 estimates agree there too.",
            ),
            p(
                "Not yet",
                "The other robreg estimators (s, mm, lts, lqs, q), bisquare, weights and the \
                 vce() options.",
            ),
        ],
    },
    Guide {
        command: "xpose",
        about: "Turns the data so observations become variables",
        points: &[p(
            "In R",
            "stata_xpose makes variables v1, v2, ... from the observations and observations \
             from the variables; a text variable is a row of missing values, as in Stata. The \
             new variables are rounded to float, as Stata stores them, unless promote keeps \
             them double; varname adds _varname with the old names. Labels and formats go.",
        )],
    },
    Guide {
        command: "xtile",
        about: "Groups a variable by its quantiles",
        points: &[
            p(
                "In R",
                "stata_xtile gives each value 1 plus the number of cut points below it, as Stata \
                 does (a cut point repeated by ties counts each time). The cut points are \
                 Stata's percentiles of the rows if and in keep (R's quantile type 2; altdef is \
                 type 6), or every value of cutpoints()'s variable whatever if keeps. Rows left \
                 out and missing values are missing.",
            ),
            p(
                "Label",
                "The new variable is labelled as in Stata (\"4 quantiles of x\", \"x categorized \
                 by c\"); it holds whole numbers, as Stata's byte does.",
            ),
            p("Not yet", "Weights."),
        ],
    },
    Guide {
        command: "tsset",
        about: "Declares time-series data, for L., F., D. and S.",
        points: &[
            p(
                "In R",
                "stata_tsset checks the times (whole numbers, none repeated within a panel, none \
                 closer than delta()), sorts the data by panel and time and prints Stata's lines. \
                 L2.x is stata_ts(): x at time t - 2 delta in the same panel, missing when no row \
                 has that time, worked out over every row whatever if keeps. A chain is one sum \
                 over x, as in Stata (D.F.x is F.x - x).",
            ),
            p(
                "Models",
                "In a model the operators become columns of its data named as Stata names the \
                 coefficients (LD.x, L2.x); L(1/2).x and L.(x y) are written out first. predict \
                 adds the same columns.",
            ),
            p(
                "Not sorted",
                "When the data are not in panel and time order, Stata stops with \"not sorted\" \
                 (r(5)); so does the R. Under by, the operators are translated only when by \
                 names the panel variable.",
            ),
            p(
                "tsfill",
                "A row for each time missing between a panel's first and last (from the data's \
                 first to its last for every panel with full), its other variables missing and \
                 text \"\", the data in panel and time order, as Stata 19 does.",
            ),
            p(
                "Not yet",
                "replace that reads its own variable at other times (replace x = L.x + 1) runs \
                 row by row in Stata and is not translated yet; nor are the operators on a string \
                 variable, two seasonal differences (S.S.x), or a time unit's dates in the output.",
            ),
        ],
    },
];

/// The page a command has (`reg` is `regress`; `probit` shares `logit`'s,
/// `tab1` and `tab2` `tabulate`'s, `scatter` and `line` `twoway`'s).
pub fn guide(command: &str) -> Option<&'static Guide> {
    let full = full(command);
    let page = match full.as_str() {
        "probit" => "logit",
        "lassoselect" | "lassoknots" | "lassocoef" | "lassoinfo" | "elasticnet" | "sqrtlasso"
        | "dsregress" | "poregress" | "xporegress" | "dslogit" | "pologit" => "lasso",
        "oprobit" => "ologit",
        "xtabond" | "xtdpd" => "xtdpdsys",
        "tsfill" => "tsset",
        "tab1" | "tab2" => "tabulate",
        "prop" => "proportion",
        "rm" => "erase",
        "rmdir" => "mkdir",
        "note" => "notes",
        "fdause" => "fdasave",
        "args" | "marksample" => "program",
        "scatter" | "line" | "connected" => "twoway",
        "bysort" => "by",
        "svyset" | "svydescribe" => "svy",
        "saveold" => "save",
        "return" => "ereturn",
        "run" | "include" => "do",
        "global" | "macro" | "tempvar" | "tempname" | "tempfile" | "unab" | "tokenize"
        | "gettoken" => "local",
        "restore" => "preserve",
        "post" | "postclose" | "postutil" => "postfile",
        "estadd" => "eststo",
        "else" => "if",
        "forvalues" | "while" | "continue" => "foreach",
        "cmdlog" => "log",
        other => other,
    };
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
