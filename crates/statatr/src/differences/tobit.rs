Guide {
    command: "tobit",
    about: "Censored (tobit) regression",
    points: &[
        p(
            "As Stata 19",
            "tobit y x..., ll(#) ul(#) as Stata fits it: its gsem-style log (Refining starting values, Grid node 0, \
             Fitting full model), the least squares start with sigma^2 as the parameter, ml's Newton steps, the \
             header with the censoring counts and limits, the LR test (F with robust, pweights or clusters, on N - k \
             degrees of freedom), var(e.y) with its interval, e(). fweights, pweights, iweights, vce(robust), \
             vce(cluster), level(), nolog, factor variables, replay. predict gives xb, stdp, stdf, pr(a,b), e(a,b), \
             ystar(a,b); estat ic is in R's layout.",
        ),
        p(
            "Not yet",
            "aweights (each row's own variance), from(), iterate(), noconstant, interactions and ib. bases, \
             stepwise and nestreg with tobit, estat other than ic, margins.",
        ),
    ],
}
