Guide {
    command: "canon",
    about: "Canonical correlation analysis",
    points: &[
        p(
            "As Stata 19",
            "canon (y1 y2) (x1 x2 x3) [, lc(#) first(#) stdcoef]: the canonical correlations, raw and standardized \
             coefficients, loadings and the four tests of significance; lc() and first() cut e() as well as the \
             printout. Checked against Stata 19 to 1e-9.",
        ),
        p(
            "Not yet",
            "Variable lists that are not written out, weights, and the other options (notests, test(), format) \
             are not translated.",
        ),
    ],
}
