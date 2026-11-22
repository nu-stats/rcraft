Guide {
    command: "nlcom",
    about: "Nonlinear combinations of the coefficients (nlcom _b[x]/_b[z], nlcom (name: exp) ...)",
    points: &[
        p(
            "In R",
            "stata_nlcom works each expression out at the coefficients and its standard error \
             by the delta method, with the derivatives taken by central differences \
             (extrapolated) on Stata mode's own translation of the expression; z and the \
             normal interval, as Stata's, after regress too. The table and r() are Stata \
             19's: r(b), r(V), r(table), r(N), r(level).",
        ),
        p(
            "post",
            "The combinations become e(b) and e(V) under their names (r, _nl_1), with Stata \
             19's e(N), e(rank), e(cmd) nlcom, e(depvar), e(predict) and e(properties); test, \
             lincom and nlcom then use them, with z.",
        ),
        p(
            "Not yet",
            "nlcom after models other than regress, logit, probit, poisson, nbreg, ologit, \
             oprobit, xtreg, ivregress, clogit, stcox and mlogit ([2]_b[x] for an equation) \
             or nlcom's own posted results (after margins, post it is refused), and \
             expressions of the data's variables are refused. After the likelihood models the \
             results are as close to Stata's as the model's own estimates (6 digits or more).",
        ),
    ],
}
