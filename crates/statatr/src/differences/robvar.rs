Guide {
    command: "robvar",
    about: "Tests that the variances of groups are equal, robustly",
    points: &[
        p(
            "As Stata 19",
            "robvar x, by(g) prints the summary of x in each group, then Levene's W0 and Brown \
             and Forsythe's W50 and W10: the one-way F of the absolute deviations from each \
             group's mean, median and 10% trimmed mean (the integer part of 0.1 n values \
             dropped from each end, as R's mean(trim = 0.1)), with df(k - 1, N - k) and the \
             p value. r() holds N, w10, p_w10, w50, p_w50, w0, p_w0, df_1 and df_2; with one \
             group or constant data they are missing, as in Stata. All agree to 9 digits.",
        ),
        p(
            "Errors",
            "Weights r(101), no by() r(198), two by variables r(103), a string variable \
             r(109). A sample with no rows is an empty table in Stata; here R stops.",
        ),
    ],
}
