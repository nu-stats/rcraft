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
        p("Not yet", "factor variables, from(), noconstant, other predict statistics."),
    ],
}
