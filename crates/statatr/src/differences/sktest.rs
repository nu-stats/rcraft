Guide {
    command: "sktest",
    about: "Skewness and kurtosis tests for normality (sktest x, noadjust)",
    points: &[
        p(
            "In R",
            "stata_sktest computes D'Agostino, Belanger and D'Agostino's skewness and \
             kurtosis tests from summarize's skewness and kurtosis (with frequency or analytic \
             weights), and the joint chi2 with Royston's adjustment unless noadjust, as \
             Stata's manual gives them; Stata 19's table and r() (N, p_skew, p_kurt, chi2, \
             p_chi2 and r(table)). Fewer than 8 observations give missing tests, as in \
             Stata.",
        ),
        p(
            "Not yet",
            "Patterns in the varlist that the data's names do not settle are refused.",
        ),
    ],
}
