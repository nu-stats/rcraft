Guide {
    command: "ratio",
    about: "Ratios of means with linearized standard errors (ratio y/x, ratio (r: y/x))",
    points: &[p(
        "In R",
        "stata_meanest gives sum w y / sum w x and its linearized variance, n/(n-1) sum z z' \
         with z = w (y - R x) / sum w x, each group of over() on its own, as Stata 19's \
         (frequency weights counting rows); named ratios and _ratio_#, the lines naming \
         each ratio before the table. Analytic weights are Stata's error (aweights not \
         allowed); iweights are refused.",
    )],
}
