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
}
