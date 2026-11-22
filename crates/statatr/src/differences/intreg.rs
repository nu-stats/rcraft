Guide {
    command: "intreg",
    about: "Interval regression",
    points: &[
        p(
            "As Stata 19",
            "intreg lo hi x...: missing lo is left-censored, missing hi right-censored, equal values exact. Olsen's \
             parameters with both logs (constant-only and full), LR or Wald test, /lnsigma and sigma, fweights, \
             pweights, iweights, robust, clustered, factor variables, replay, predict (xb, stdp, stdf, pr, e, ystar).",
        ),
        p("As Stata 19 (options)", "noconstant (a Wald test, no constant-only fit), offset(), iterate()."),
        p(
            "As Stata 19 (estat)",
            "estat ic (r(S), n(), df()), estat vce (covariance and correlation, format(), r(V)) and estat summarize \
             (a varlist, equation, labels, noheader, the weights' row, clusters, r(stats)) in Stata's layout; every \
             other estat subcommand stops as Stata stops it (r(321)).",
        ),
        p("Not yet", "aweights, from(), constraints(), het(), interactions, estat vce and summarize with factor variables, margins."),
    ],
}
