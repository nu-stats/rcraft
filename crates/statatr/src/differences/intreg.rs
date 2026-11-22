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
        p("Not yet", "aweights, from(), constraints(), het(), interactions, estat other than ic."),
    ],
}
