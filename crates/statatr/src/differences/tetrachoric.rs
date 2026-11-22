Guide {
    command: "tetrachoric",
    about: "Tetrachoric correlations of 0/1 variables (tetrachoric b1 b2, stats(rho se p obs), edwards)",
    points: &[
        p(
            "In R",
            "stata_tetrachoric takes each pair's thresholds from its margins and the rho that \
             maximizes the 2x2 table's likelihood (the bivariate normal by Sheppard's integral), \
             its standard error the full model's inverse information; edwards gives Edwards and \
             Edwards's (a^(pi/4) - 1)/(a^(pi/4) + 1) of the odds ratio with its delta-method \
             error; the p-value is Fisher's exact test, Bonferroni's or Sidak's. Listwise or pw, \
             frequency weights, Stata 19's two-variable report and matrix (stats(), print(), \
             star(), matrix, notable) and r().",
        ),
        p(
            "Not yet",
            "A 2x2 table with an empty cell (and zeroadjust), correlations within .001 of -1 or \
             1, posdef's adjustment of a matrix that is not positive definite, and weights other \
             than a frequency-weight variable are not translated. r(rho) and r(se_rho) can \
             differ from Stata's in the 16th digit.",
        ),
    ],
}
