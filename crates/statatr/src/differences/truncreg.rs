Guide {
    command: "truncreg",
    about: "Truncated regression",
    points: &[
        p(
            "As Stata 19",
            "truncreg y x..., ll(#) ul(#): the rows outside the limits are dropped and counted, sigma is a parameter, \
             the Wald test, fweights, aweights (rescaled to the rows), pweights, iweights, robust, clustered, replay, \
             predict xb and stdp. Stata's derivatives are numerical and ours exact, so estimates agree to about 1e-7.",
        ),
        p("As Stata 19 (options)", "offset(), iterate() when it converges within the limit (an unconverged search differs from Stata's numerical derivatives and stops with a message)."),
        p("Not yet", "factor variables, noconstant (the variances differ from Stata's by about 1e-3), from(), constraints(), other predict statistics."),
    ],
}
