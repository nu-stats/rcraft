Guide {
    command: "boxcox",
    about: "Box-Cox regression",
    points: &[
        p(
            "As Stata 19",
            "boxcox y [x...], model(lhsonly|rhsonly|lambda|theta), notrans(), lrtest, nolog, nologlr, level(): the \
             comparison and full fits and their logs, the transform parameter's table, the scale-variant parameters \
             (with the LR tests per regressor under lrtest), the restricted fits at -1, 0 and 1, e(b), e(V). predict \
             gives yhat (with smearing), residuals and btransform. Weights stop as Stata stops.",
        ),
        p(
            "Not yet",
            "Stata's numerical Hessian moves its path in the seventh digit, so estimates agree to about 1e-6 and the \
             iteration log can differ in its last digits; factor variables.",
        ),
    ],
}
