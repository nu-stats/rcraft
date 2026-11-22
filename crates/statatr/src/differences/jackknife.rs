Guide {
    command: "jackknife",
    about: "Repeats a command with each observation (or cluster) left out",
    points: &[
        p(
            "Method",
            "The command runs on the data, then once for each observation of its estimation \
             sample (each cluster with cluster()) left out. The replicates are kept as floats, \
             as Stata keeps them; the variance is (n - 1)/n times the sum of products of the \
             deviations, with t on n - 1 degrees of freedom. e(b_jk) is the bias-corrected \
             value, n times the observed minus (n - 1) times the mean of the replicates.",
        ),
        p(
            "Forms",
            "jackknife exp_list: command with r() or e() values, or _b after regress, logit or \
             probit; regress, logit and probit with vce(jackknife) are the same run. \
             cluster(), level(), nodots and notable are translated; r(table) is left.",
        ),
        p(
            "Not yet",
            "Other options, a command with if or in that sets no e(sample), a missing value of \
             the cluster variable, and a replication that stops with an error are refused with \
             the reason.",
        ),
    ],
}
