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
                 The extended missing values .a to .z all become NA, with a note.",
            ),
            p(
                "Float and double",
                "generate without a type makes a float in Stata (about 7 digits); R keeps every \
                 number as a double. The translation rounds a new variable to float as Stata \
                 stores it, unless the do-file says double, so x == 0.1 behaves as in Stata. \
                 replace keeps a float a float in Stata; R keeps full precision for a variable \
                 the do-file did not make, with a note.",
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
                 row as Stata draws. rpoisson(), rbeta() and rt(), rbinomial(), rnbinomial() and \
                 rhypergeometric() past the ranges whose method is known (they stop with the \
                 reason), simulate and permute, two draws in one expression and version 13 or \
                 earlier are refused.",
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
                "Not yet",
                "Statistics are _b after regress, logit or probit, or r() and e() values; other \
                 options (strata(), saving(), ...), predict and estat after it are refused with \
                 the reason.",
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
                "Not yet",
                "strata(), weight(), and if or in together with a sample size or cluster() are \
                 refused with the reason.",
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
        points: &[p(
            "Precision",
            "Stata stores means and the like of float variables as floats; R keeps full \
             precision, so the 8th digit can differ.",
        )],
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
            LAYOUT_R,
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
            LAYOUT_R,
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
        command: "ereturn",
        about: "ereturn list",
        points: &[p(
            "What is listed",
            "Stata's scalars, macros and matrices in Stata's layout; what Stata also lists and \
             RCraft does not keep (the optimizer's settings, e(ilog), ...) is named in a note.",
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
            p("generate()", "The new variable is 0 for the rows there before and 1 for the copies; Stata also gives 0 and 1 value labels, which RCraft does not."),
        ],
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
        points: &[p(
            "Information criteria",
            "estat ic counts the coefficients as the degrees of freedom, as Stata does; R's \
             AIC() on a linear model also counts the variance.",
        )],
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
                "Not yet",
                "estimates dir, table, replay, save and use, and options, are not translated. A \
                 name stored by a line that was not translated (eststo, a program) is not known, \
                 so a restore of it is refused rather than guessed.",
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
                "A row the if leaves out is missing (.), or empty text for a string, as in Stata.",
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
        points: &[p(
            "Tests",
            "t on the model's degrees of freedom after regress, z after a likelihood model, as \
             Stata's; with or, irr or rrr the estimate and its interval are exponentiated and \
             the standard error is the delta method's.",
        )],
    },
    Guide {
        command: "list",
        about: "Lists rows",
        points: &[LAYOUT_R],
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
                 omitted with the rows it decides, or the fit stops with Stata's message. R's \
                 glm() would go on with huge estimates.",
            ),
            p(
                "Collinear regressors",
                "The later of a collinear set, in the order written, is omitted with Stata's note \
                 and tolerance, as for poisson, nbreg, ologit, oprobit and mlogit (in every \
                 equation). R's glm() checks far more tightly, so a variable stored as float that \
                 is nearly another plus a constant stays in and the fit fails; polr() and \
                 multinom() do not leave a column out at all.",
            ),
            ML_SE,
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
            LAYOUT_R,
        ],
    },
    Guide {
        command: "matrix",
        about: "matrix list",
        points: &[p(
            "Columns",
            "e(b) and e(V) have Stata's columns (base levels as zeros, o. for omitted, the \
             equations), printed in Stata's layout at a linesize of 80.",
        )],
    },
    Guide {
        command: "merge",
        about: "Joins another file",
        points: &[p(
            "_merge",
            "_merge is made as Stata makes it (1, 2, 3 with Stata's labels) and the rows are \
             in Stata's order; R's merge() sorts them and makes no _merge.",
        )],
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
            ML_ROBUST,
            ML_DIGITS,
            MISSING_IN_MODELS,
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
            p(
                "Standard errors",
                "From R's numerical Hessian, so the last digit or two can differ from Stata's.",
            ),
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
        command: "poisson",
        about: "Poisson regression",
        points: &[ML_SE, ML_ROBUST, ML_DIGITS, MISSING_IN_MODELS],
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
            LAYOUT_R,
        ],
    },
    Guide {
        command: "recode",
        about: "Changes values by rules",
        points: &[p(
            "Rules",
            "The rules are taken in Stata's order, the first that matches a value deciding it; \
             missing values are kept unless a rule names them, as in Stata.",
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
        points: &[p(
            "Storage type",
            "replace keeps a float a float in Stata; R keeps full precision unless the \
             do-file made the variable, with a note.",
        )],
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
                "Stata's own table without detail or weights, checked character for character; \
                 detail and weights keep R's layout.",
            ),
        ],
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
                "Stata's own tables for counts, checked character for character; percentages, \
                 analytic weights and string variables keep R's layout.",
            ),
        ],
    },
    Guide {
        command: "test",
        about: "Wald tests after a model",
        points: &[p(
            "Statistic",
            "F on the model's degrees of freedom after regress, chi2 after a likelihood \
             model, as Stata's.",
        )],
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
        about: "t tests: one sample, paired, two samples",
        points: &[
            p(
                "Variances",
                "Two groups are taken to have the same variance unless unequal, as Stata does; \
                 R's t.test() assumes unequal variances by default (Welch). welch itself is \
                 refused, since R offers only Satterthwaite's degrees of freedom.",
            ),
            p(
                "p-values",
                "R prints the two-sided p-value; Stata prints the one-sided ones beside it. \
                 r() holds all three.",
            ),
            LAYOUT_R,
        ],
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
                 (N - 1) / (N - k), as Stata's.",
            ),
            MISSING_IN_MODELS,
            LAYOUT_R,
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
             as a number. The lags are the same.",
        )],
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
        "oprobit" => "ologit",
        "tab1" | "tab2" => "tabulate",
        "scatter" | "line" | "connected" => "twoway",
        "bysort" => "by",
        "saveold" => "save",
        "return" => "ereturn",
        "run" | "include" => "do",
        "global" | "macro" | "tempvar" | "tempname" | "tempfile" | "unab" | "tokenize" => "local",
        "restore" => "preserve",
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
        assert_eq!(page("xtlogit y x"), None);
    }
}
