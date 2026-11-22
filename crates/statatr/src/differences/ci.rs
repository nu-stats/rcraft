Guide {
    command: "ci",
    about: "Confidence intervals for means, proportions and variances (ci means x, ci proportions b, ci variances x)",
    points: &[
        p(
            "In R",
            "stata_ci prints Stata 19's table and leaves the last variable's r(). Means: the \
             t interval, with frequency and analytic weights as summarize takes them. \
             Proportions: the exact (Clopper-Pearson, from qbeta), Wald, Wilson, \
             Agresti-Coull and Jeffreys intervals, with Stata's marks and notes where an \
             interval is one-sided or clipped at 0 or 1, and only the 0/1 variables (a note \
             says so). Variances: the chi2 interval, or Bonett's (bonett: the kurtosis about \
             a trimmed mean, as Stata's manual gives it), and with sd the standard deviation. \
             Means, poisson (or exposure()): the exact Poisson interval of the total, which \
             Stata rounds to a whole number, as rates over the exposure.",
        ),
        p(
            "Not yet",
            "Patterns in the varlist and exposure() with weights are refused. Frequency \
             weights are taken everywhere and analytic ones for normal means; analytic weights \
             elsewhere are Stata's own error (aweights not allowed, r(101)). cii has a page of \
             its own.",
        ),
    ],
}
