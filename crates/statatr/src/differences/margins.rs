Guide {
    command: "margins",
    about: "Margins and marginal effects",
    points: &[
        p(
            "Rows averaged",
            "Averaged over the estimation sample, Stata's e(sample); if the data changed \
             since the model, the current values of its rows are used, as Stata does.",
        ),
        p(
            "Standard errors",
            "The delta method from the model's own variance, as Stata's.",
        ),
        p(
            "r() and at()",
            "r(table) (b se z or t pvalue ll ul df crit eform) and r(b) and r(V) with Stata's \n                 names (the first level of a term 1bn.: 1bn._at 2._at). at() over several \n                 variables is numbered as Stata 19 numbers it: the variables in the model's \n                 order, the last changing fastest. After xtreg the statistics are z, as in \n                 Stata.",
        ),
        p(
            "After clogit",
            "predict(pu0) (the default: invlogit(xb), the fixed effect taken as zero); \n                 vce(unconditional) from the rows' scores of the exact likelihood, clustered as \n                 the model was. ar. compares each level with the one before, r. with the first, \n                 term by term, with Stata's chi2 tests. mcomp(bonferroni) adjusts the printed \n                 p-values and intervals, not e(b) and e(V); post keeps the base levels in e(b) \n                 as zeros.",
        ),
        LAYOUT_R,
    ],
}
