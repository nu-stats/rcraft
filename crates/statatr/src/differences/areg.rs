Guide {
    command: "areg",
    about: "Linear regression absorbing the indicators of one variable",
    points: &[
        p(
            "As Stata 19",
            "areg y x..., absorb(g) takes the slopes from the within fit (each group's weighted means removed, the \
             grand means put back, so the constant is ybar - xbar b) with N - k - G residual degrees of freedom, \
             and R-squared, rss and mss of the model with the indicators; R-squared adjusted divides by df_r and \
             F is the Wald test of the slopes. vce(robust), vce(cluster v) and the old cluster() are taken (robust \
             times N/(N - k - G); clusters G/(G - 1) (N - 1)/(N - k - G), t and F on G - 1), and \
             fweights, aweights and pweights (which imply robust). The `F test of absorbed indicators` \
             (the pooled fit against this one) is printed unless the errors are robust; e() holds Stata's \
             scalars (e(F_absorb), e(ll_0) the within data's constant-only likelihood, e(rank) that of \
             the slopes and constant, one less after clusters) and e(b) and e(V) with a zero column for a \
             factor's base level. i.g factor variables are taken. A regressor the absorbed groups make collinear (constant within each group), wherever it stands, and then the later of a collinear set are omitted as Stata omits them: `note: u omitted because of collinearity.`, a `0  (omitted)` row and an o. column of zeros in e(b) and e(V), the residual degrees of freedom counting only the slopes kept. predict gives xb (the default), \
             residuals, d (the absorbed effect of the group) and xbd; test works after it.",
        ),
        p(
            "Not yet",
            "A factor level omitted for collinearity, every regressor omitted, \
             powers (I(x^2)), iweights (stopped as Stata stops, r(101)), e(datasignature) \
             and predict's other statistics. Interactions (i.g#c.z, c.z##i.h, i.g#i.h, c.x#c.z) \
             and ib. bases are taken, with e(b) as Stata names it (1b.g#c.z, 2b.g).",
        ),
    ],
}
