Guide {
    command: "centile",
    about: "Centiles with confidence intervals (centile x, centile(25 50 75))",
    points: &[
        p(
            "In R",
            "stata_centile gives Stata 19's centiles, at rank (n + 1)q/100 between the order \
             statistics (R's quantile type 6), and its default interval: where the binomial \
             cdf passes each tail, interpolated between the order statistics, held at the \
             sample's minimum or maximum (and marked) where that order statistic is not \
             there. Each variable uses its own rows; r() is the last variable's (N, n_cent, \
             c_#, lb_#, ub_#, centiles).",
        ),
        p(
            "Other intervals",
            "cci gives the conservative interval (the order statistics themselves), normal \
             the centile's standard error from a normal density at it, meansd the centile of \
             a normal with the sample's mean and standard deviation, all as Stata's manual \
             gives them and checked against Stata 19.",
        ),
        p(
            "Not yet",
            "centile() lists that are not plain numbers are refused; weights are Stata's own \
             error (weights not allowed, r(101)).",
        ),
    ],
}
