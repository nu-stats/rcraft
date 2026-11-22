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
}
