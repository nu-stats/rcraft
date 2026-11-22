Guide {
    command: "hetprobit",
    about: "Heteroskedastic probit",
    points: &[
        p(
            "As Stata 19",
            "hetprobit y x..., het(z...): its probit fit first (Fitting probit model), then the full model from it, \
             the Wald test of the slopes and the LR (Wald with robust) test of lnsigma = 0, fweights, pweights, \
             iweights, robust, clustered, replay, predict pr, xb, stdp and sigma.",
        ),
        p("As Stata 19 (options)", "noconstant, offset(), iterate() (the probit fit always runs to convergence)."),
        p("Not yet", "factor variables, from(), constraints()."),
    ],
}
