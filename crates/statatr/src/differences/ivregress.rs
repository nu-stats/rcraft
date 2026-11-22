Guide {
    command: "ivregress",
    about: "Instrumental-variables regression",
    points: &[
        p(
            "Degrees of freedom",
            "Without small the variance divides by N, with z tests, as in Stata; small \
             gives N - k and t tests.",
        ),
        p(
            "Clusters",
            "Clustered errors without small carry no G / (G - 1), as Stata's ivregress \
             (unlike its regress).",
        ),
        p(
            "GMM",
            "The weight matrix from the 2SLS residuals, robust unless wmatrix() says \
             otherwise, and the variance of the same kind unless vce() says, as Stata's.",
        ),
        MISSING_IN_MODELS,
        LAYOUT_R,
    ],
}
