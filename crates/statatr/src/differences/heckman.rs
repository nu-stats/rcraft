Guide {
    command: "heckman",
    about: "Heckman selection model",
    points: &[
        p(
            "As Stata 19",
            "heckman y x..., select([s =] z...) by maximum likelihood from the two-step start, or twostep: the \
             iteration log, the equations, /athrho, /lnsigma, rho, sigma, lambda, the LR (Wald with robust) test of \
             independence, fweights, pweights, iweights, robust, clustered, replay. predict gives xb, xbsel, psel, \
             mills, ycond and yexp.",
        ),
        p("As Stata 19 (options)", "offset() in the outcome equation, iterate(), noconstant with twostep."),
        p(
            "As Stata 19 (estat)",
            "estat ic (r(S), n(), df(); after twostep Stata's r(321)), estat vce (covariance and correlation, format(), \
             r(V)) and estat summarize (a varlist, equation, labels, noheader, the weights' row, clusters, r(stats)) in \
             Stata's layout; every other estat subcommand stops as Stata stops it (r(321)).",
        ),
        p(
            "Not yet",
            "factor variables (also in select()), mills() and the other options, from(), noconstant by maximum likelihood (twostep is translated; Stata's start differs), constraints().",
        ),
    ],
}
