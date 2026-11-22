Guide {
    command: "npregress",
    about: "Nonparametric kernel regression",
    points: &[
        p(
            "As Stata 19",
            "npregress kernel with one continuous covariate: the local-linear or \
             local-constant fit, the mean's bandwidth by leave-one-out cross-validation and \
             the derivative's by Stata's local-cubic proxy, both found by Stata's Nelder-Mead \
             search; every kernel; given bandwidths, iterate(), the identification rule, the \
             table, e() and the two variables it leaves (StatCraft's decoding of Stata 19).",
        ),
        p(
            "Differences",
            "Stored bandwidths agree with Stata to about 1e-15, not always to the last bit. \
             Several covariates, factor variables, imaic, vce(bootstrap), bandwidths given as \
             a matrix, e(datasignature), and predict or margins afterwards are not \
             translated.",
        ),
    ],
}
