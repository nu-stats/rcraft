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
                 The extended missing values .a to .z are kept as haven's tagged NAs and \
                 compare in Stata's order (. < .a < ... < .z).",
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
                 row as Stata draws. rt() is Stata's draw to within its last bit or so, the \
                 draws after it exactly Stata's; rbeta() is Stata's draw to its last bits or so, \
                 its stream exactly Stata's. rpoisson(), rbeta() with equal shapes above 1, \
                 rbinomial(), rnbinomial() and \
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
        points: &[p(
            "Information criteria",
            "estat ic counts the coefficients as the degrees of freedom, as Stata does; R's \
             AIC() on a linear model also counts the variance.",
        )],
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
        command: "program",
        about: "Defines a program the do-file then runs by name",
        points: &[
            p(
                "In R",
                "There is no R function for a program: RCraft runs its lines where it is called, \
                 each translated as any line is, with locals of its own (`0', `1' ... and args), \
                 temporary variables dropped when it returns, exit returning from it, and an \
                 r-class program's return scalar and return local as r() afterwards.",
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
        command: "recode",
        about: "Changes values by rules",
        points: &[p(
            "Rules",
            "The rules are taken in Stata's order, the first that matches a value deciding it; \
             missing values are kept unless a rule names them, as in Stata. Labels in the \
             rules (`1 \"low\"`) make a label set for each new variable, named after it or \
             label(), as Stata does; they need generate() or prefix().",
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
        about: "Survey estimation (svyset, svy: clogit)",
        points: &[
            p(
                "As Stata 19",
                "svyset psu [pweight=w] (one stage, no strata, no finite population \
                 correction) and svy: clogit: the weighted estimates (one weight a group, on \
                 Stata's Newton-Raphson path), the variance linearized over the PSUs, G / (G - 1) \
                 times their score sums; t tests on the design df (PSUs less 1), Stata's adjusted \
                 Wald F, e() as Stata's (N_pop the groups' weights). predict pc1 and pu0 after it, \
                 and margins with vce(unconditional): weighted means, linearized over the PSUs, on \
                 the design df.",
            ),
            p(
                "Not yet",
                "Strata, several stages, fpc, subpop(), other models under svy, and groups with \
                 more than one positive outcome.",
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
                "Not yet",
                "Ranges, matrix(), etable, formula(), pictures, the formatting options (bold, \
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
                "Stata's own table without detail or weights, checked character for character; \
                 detail and weights keep R's layout.",
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
                 must be left out, and the data cannot tell which. RCraft leaves out the later \
                 one, as Stata does when the collinearity is exact; where rounding leaves a \
                 residual, Stata's choice rests on it ('collinearity with the absorbed \
                 variable') and may be another of the set. A note names the coefficients that \
                 then differ (those of the set and _cons); every other result is the same \
                 whichever is left out: the other coefficients and their standard errors, N, \
                 the R-squareds, sigma_u, sigma_e, rho, F and the predictions.",
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
                 without one) at levels(); several plots 1/(k+1) apart, offset() moving one; \
                 headings(); subgraphs at ||. gen() leaves those rows, as floats, as coefplot does.",
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
                "Not yet",
                "wide, cells(), matrix(), mlabels() with its suboptions, varlabels(), .rtf (it \
                 carries the time it was written) and not with a file are refused.",
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
                 that is not positive).",
            ),
            p(
                "Not yet",
                "More than two absorbed variables, interactions in absorb(), savefe and resid, \
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
        "svyset" => "svy",
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
