Guide {
    command: "pcorr",
    about: "Partial and semipartial correlations of one variable with the others",
    points: &[
        p(
            "As Stata 19",
            "pcorr y x1 x2 ... fits y on all the others over the rows where every variable is \
             present. With t a coefficient's t statistic and df the residual degrees of \
             freedom, the partial correlation is t / sqrt(t^2 + df), the semipartial one is \
             t sqrt((1 - R2) / df), and the significance is the coefficient's p value. The \
             table, r(N), r(df), r(sp_corr) and r(p_corr) are Stata's. Frequency weights count \
             N as their sum (r(N) and df follow), analytic weights keep it as the rows. A \
             variable named twice counts once, so pcorr y x x is too few variables.",
        ),
        p(
            "Errors",
            "No variable r(100), fewer than three r(102), a name the data lack r(111), no \
             rows or a string variable r(2000), pweights and iweights r(101).",
        ),
        p(
            "Not yet",
            "A regressor collinear with the others stops with a message: Stata drops the \
             first of the collinear variables and prints (dropped) in its row, and what it \
             then leaves in the two matrices is not known here. by: and patterns in the \
             varlist are refused.",
        ),
    ],
}
