Guide {
    command: "cloglog",
    about: "Complementary log-log regression",
    points: &[
        p(
            "As Stata 19",
            "cloglog y x...: Newton steps from the logit fit one iteration before its end (Stata's own start), \
             the iteration count, LR or Wald test, e(ll_0), eform, fweights, pweights, iweights (aweights stop as \
             Stata stops, r(101)), robust, clustered, replay, predict pr and xb.",
        ),
        p("As Stata 19 (options)", "noconstant (a Wald test), offset(), iterate()."),
        p("As Stata 19 (estat)", "estat ic (r(S), n(), df()), estat vce (covariance and correlation, format(), r(V)) and estat summarize (a varlist, equation, labels, noheader, the weights' row, clusters, r(stats)) in Stata's layout; every other estat subcommand stops as Stata stops it (r(321))."),
        p("Not yet", "factor variables, from(), constraints()."),
    ],
}
