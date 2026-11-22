Guide {
    command: "contrast",
    about: "Contrasts of marginal linear predictions (contrast g, contrast r.g#h, contrast g@h)",
    points: &[
        p(
            "In R",
            "stata_contrast takes the balanced margins (the other factors averaged with equal \
             weights, covariates at zero) from R's own model matrix and tests plain, r., ar., a., \
             gw. and g. contrasts, g#h and g@h, with F after the linear models and chi2 after \
             the others, a generalized inverse for the rank, mcompare(bonferroni, sidak, \
             scheffe), effects and level(); a contrast the design cannot estimate is (not \
             testable). Tables and r() are Stata 19's: r(b), r(V), r(table), r(F) or r(chi2), \
             r(df), r(p), r(df2).",
        ),
        p(
            "Not yet",
            "After models other than regress, anova, xtreg, logit, probit and poisson (areg, \
             xtregar among them), post, noestimcheck, pveffects, cieffects, tukey and the \
             other multiple-comparison methods, gw. after a weighted model, r(L).",
        ),
    ],
}
