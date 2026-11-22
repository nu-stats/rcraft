Guide {
    command: "robreg",
    about: "Robust regression (the SSC package): robreg m",
    points: &[
        p(
            "In R",
            "stata_robreg fits robreg m as the package does, matched to its results (its code \
             was not read): least absolute deviations for the start, the scale from those \
             residuals (their normalized median, the fit's p zero residuals left out), then \
             Huber's M estimate with that scale fixed (k = 1.3449975 for 95% efficiency), \
             robust standard errors with t tests on n - p degrees of freedom.",
        ),
        p(
            "predict",
            "xb (the default), residuals and weights (the Huber weights).",
        ),
        p(
            "Indicator variables",
            "When the LAD starting fit is not unique, as indicator variables often make it, \
             robreg starts from a point inside the LAD solutions; RCraft finds the same one \
             with quantreg's interior-point solver (method fn), so the scale and the \
             estimates agree there too.",
        ),
        p(
            "Not yet",
            "The other robreg estimators (s, mm, lts, lqs, q), bisquare, weights and the \
             vce() options.",
        ),
    ],
}
