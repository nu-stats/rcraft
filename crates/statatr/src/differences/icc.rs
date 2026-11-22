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
}
