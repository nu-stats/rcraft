Guide {
    command: "slogit",
    about: "Stereotype logistic regression",
    points: &[
        p(
            "As Stata 19",
            "slogit y x1 x2 [, dimension(#) baseoutcome(#) nolog]: the stereotype model with Stata's constraints on \
             phi, started from the multinomial logit's singular value decomposition; e(b) names (dim#:x, phi#_j:_cons, \
             theta#:_cons), e(V), e(ll), e(ll_0), the Wald test; predict p1 p2 ... , predict v, outcome(#) and \
             predict v, xb. A dimension above the limit stops as Stata does (r(198)).",
        ),
        p(
            "Differences",
            "Stata stops where its own iterations do, so digits beyond the 7th can differ; iteration logs, e(ic) and \
             e(gradient) are not reproduced.",
        ),
        p(
            "Not yet",
            "Weights, factor variables, vce(), constraints(), noconstant and the other options.",
        ),
    ],
}
