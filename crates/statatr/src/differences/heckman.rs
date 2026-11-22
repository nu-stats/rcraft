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
            "Not yet",
            "factor variables (also in select()), mills() and the other options, from(), noconstant by maximum likelihood (twostep is translated; Stata's start differs), constraints().",
        ),
    ],
}
