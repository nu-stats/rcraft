Guide {
    command: "pdslasso",
    about: "Post-double-selection lasso (pdslasso, SSC, with lassopack's rigorous lasso)",
    points: &[
        p(
            "In R",
            "stata_pdslasso selects controls for the dependent variable and for the regressor \
             of interest by the rigorous lasso with heteroskedastic loadings (c = 1.1, gamma = \
             0.1/log(n), two loading passes from OLS on the five most correlated controls), \
             then fits OLS on their union with robust (HC0) standard errors as e(b) and e(V), \
             and prints Stata 19's output: the selections, the specification, the CHS and PDS \
             tables. On ZNMIXH's data it selects what Stata does and matches its estimates.",
        ),
        p(
            "Not yet",
            "Without robust (homoskedastic loadings), several regressors of interest, \
             instruments, partial(), pnotpen(), clusters, fixed effects, weights and ranges or \
             patterns in the controls are refused; predict, test and margins after it too.",
        ),
    ],
}
