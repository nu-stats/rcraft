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
        p("Not yet", "factor variables, from(), offset, noconstant."),
    ],
}
