Guide {
    command: "mprobit",
    about: "Multinomial probit regression",
    points: &[
        p(
            "As Stata 19",
            "mprobit y x1 x2 [, baseoutcome(#) probitparam nolog]: the likelihood is the integral over one standard \
             normal of the product of normal probabilities, evaluated to 1e-10 (Stata's 15 points are about as \
             exact), maximized at its optimum. Coefficients, covariance, e(ll), the Wald test and e() agree with \
             Stata 19 to about 1e-7; predict p1 p2 p3 (one variable per outcome) and predict p, outcome(#).",
        ),
        p(
            "Differences",
            "Stata stops where its own iterations do, so digits beyond the 7th can differ; iteration logs, e(ic) and \
             e(gradient) are not reproduced.",
        ),
        p(
            "Not yet",
            "Weights, factor variables, vce(), constraints(), intpoints(), noconstant and predict, xb.",
        ),
    ],
}
