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
}
