Guide {
    command: "total",
    about: "Totals with their standard errors (total y, over(g))",
    points: &[p(
        "In R",
        "As mean's (stata_meanest), each total the sum of the weights times the mean, its \
         variance that sum squared times the mean's, as Stata 19 gives it; frequency and \
         probability weights (iweights as frequency weights), over(); analytic weights are \
         Stata's error (aweights not allowed).",
    )],
}
