Guide {
    command: "estat",
    about: "Statistics after a model",
    points: &[
        p(
            "Information criteria",
            "estat ic counts the coefficients as the degrees of freedom, as Stata does; R's \
             AIC() on a linear model also counts the variance.",
        ),
        p(
            "After regress",
            "vif (largest first unless the model has factor variables; uncentered with the \
             constant's), imtest (Cameron and Trivedi's decomposition, with white; its \
             statistics agree with Stata's to about 3e-6), and on tsset data bgodfrey and \
             durbinalt (lags(), nomiss0) and dwatson, with the gaps in the sample counted: \
             Stata 19's tables and r(), its matrices named lags:1 ... as Stata's. As Stata \
             keeps them, r(vif_#) are floats.",
        ),
        p(
            "After logit and probit",
            "classification (cutoff(): the table, the rates and r(ctable)) after logit and \
             probit; gof after logit, Pearson's over the covariate patterns or, with group(), \
             Hosmer and Lemeshow's over the quantiles of the predicted probabilities (the mean \
             of two neighbours where a quantile falls between them, as Stata's), with table.",
        ),
        p(
            "Not yet",
            "On panel data bgodfrey, durbinalt and dwatson are Stata's error (sample may not \
             include multiple panels), and after regress, robust bgodfrey and durbinalt are \
             too; small is refused. estat vce prints Stata 19's lower triangle (format(), \
             correlation), and estat sd after mean the means and standard deviations; estat \
             vce of factor variables or several equations keeps R's print.",
        ),
    ],
}
