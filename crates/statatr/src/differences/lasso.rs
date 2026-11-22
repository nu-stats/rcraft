Guide {
    command: "lasso",
    about: "Lasso for linear, logit, probit and Poisson outcomes, elasticnet linear, sqrtlasso, dsregress, poregress, xporegress, dslogit, and lassoselect, lassoknots, lassocoef, lassoinfo, predict",
    points: &[
        p(
            "In R",
            "stata_lasso_run standardises the predictors (divisor N), centres the outcome and \
             solves the lasso along Stata's grid by cyclic coordinate descent with Stata's \
             stopping rule (for logit, probit and Poisson by Newton steps with coordinate \
             descent, to the optimum), then selects lambda as Stata 19 does: none, BIC, \
             cross-validation (folds from Stata's own random numbers, rseed()), the one-s.e. \
             rule, adaptive steps or the plug-in penalty, with the deviance stop() and the \
             'minimum found' rule. elasticnet runs each alpha on Stata's shared grid with the \
             same folds and picks the alpha and lambda with the smallest BIC or CV error; \
             sqrtlasso solves the square-root lasso by Stata's own coordinate rule. \
             dsregress, poregress, xporegress and dslogit run those lassos for the outcome \
             and each variable of interest, then double selection, partialing out or \
             cross-fit (Stata's folds and resamples) with Stata's variances. The \
             log, the table, e() (e(rngstate) included) and e(sample) are Stata's; \
             lassoknots, lassocoef, lassoinfo and lassoselect print Stata's tables, and \
             predict gives the penalized or postselection prediction (Pr(y) or the number of \
             events after logit, probit and Poisson).",
        ),
        p(
            "Differs",
            "Stata's descent stops short of the optimum; for the linear lasso, elasticnet and \
             sqrtlasso RCraft follows its rule, which reproduces the probes to about 1e-15, but on \
             some data the L1 and L2 norms at the path's end differ in the 8th digit, and the \
             plug-in penalty's fit agrees to about 2e-9. For logit, probit and Poisson \
             Stata's test is not decoded: RCraft solves to the optimum (Stata's own results \
             at tolerance(1e-12)), so coefficients can differ from Stata's default ones by \
             about 1e-7 relative, the log likelihood and deviance ratio by about 1e-11. The \
             standardised constant Stata prints as -1.39e-17 (rounding) is 0 here.",
        ),
        p(
            "Not yet",
            "lasso cox, elasticnet logit, probit and poisson, elasticnet with alpha 0 \
             (ridge) or several alphas and selection(none), pologit with other than the \
             plug-in lassos, vce(robust) and no always-in controls, xpologit, dspoisson, \
             popoisson and xpopoisson, selection(cv) or lasso() inside xporegress, \
             poregress with vce(cluster), lassocoef after the inference commands, cvplot, \
             factor variables, \
             weights, cluster(), tolerance() other than the default and dtolerance() are \
             refused; so are noconstant with cross-validation and predict after offset(), \
             exposure() or noconstant for logit, probit and Poisson, and test, lincom and \
             margins after lasso.",
        ),
    ],
}
