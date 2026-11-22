Guide {
    command: "correlate",
    about: "Correlations and covariances",
    points: &[
        p(
            "Missing values",
            "A row with a missing value in any of the variables is dropped (casewise), as \
             Stata does; R's cor() would stop or need use = \"complete.obs\".",
        ),
        p(
            "Weights",
            "Frequency weights count rows; analytic weights are scaled to sum to N; r() \
             is Stata's (sum_w with analytic weights).",
        ),
        p(
            "As Stata 19",
            "Stata's matrix, checked character for character: the names cut to 8 over the \
             columns and 12 beside the rows, %8.4f correlations or %8.0g covariances, seven \
             columns to a block at a linesize of 80; means' table above it; (sum of wgt is \
             ...) with analytic weights; no observations as Stata's r(2000).",
        ),
    ],
}
