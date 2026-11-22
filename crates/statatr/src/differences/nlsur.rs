Guide {
    command: "nlsur",
    about: "Systems of nonlinear equations",
    points: &[
        p(
            "As Stata 19",
            "nlsur (y1 = <expression>) (y2 = <expression>) with {name} and {name = #} parameters (in order of first \
             appearance, shared between equations by name; unset starts are 0) and {name: varlist} linear \
             combinations (parameters name_x1, name_x2) is Gauss-Newton on the stacked equations, as Stata takes \
             it: forward-difference derivatives with the step delta (|b| + delta) (delta() 4e-7), each step halved \
             while it raises the criterion, done when no parameter moves by more than eps() (1e-5, relative to |b|), \
             at most iterate() (300) steps a stage; a parameter whose derivative column is zero or a combination of \
             earlier ones' stays put and shows (constrained). NLS comes first (log: Residual SS), then FGNLS with \
             Sigma from its residuals (Scaled RSS); ifgnls goes round again until the parameters and Sigma each \
             change by at most 1e-5; nls ends with a pass, not logged, that weights each equation by its variance \
             (it moves the estimates only when a parameter is shared). fweights, aweights and pweights (sum of wgt \
             line, pweights robust; iweights as fweights, (obs = N)), vce(robust) and vce(cluster v) (scores at \
             the final estimates; clusters times G/(G-1) (N-1)/(N-k); a row with v missing left out) and \
             variables() are taken; a linear combination written twice is the same parameters. A run that runs \
             out of steps prints its table and keeps e(), then stops with r(430). The function-evaluator form \
             (nlsur name @ varlist, parameters() or nparameters(), neq(), hasconstants(), weights, variables(), \
             noconstant) runs the do-file's program nlsurname at each step, as Stata calls it, and again for \
             predict and margins. The iteration log, the equation table, the z table and e() follow Stata's; \
             predict gives yhat and residuals with equation(#k); test, lincom and nlcom read /a, [a]_cons and \
             _b[/a]; margins gives the predictive margin of an equation's fitted values, by over() groups, and \
             post makes them e() as Stata's (terms, dydx() and at() are Stata's errors, the model having no \
             covariates); estat ic, estat summarize and estat vce are translated.",
        ),
        p(
            "Not yet",
            "An evaluator program with a line that is not translated, or that needs a value from R while it is \
             translated, refuses the fit. margins, over() with value labels or missing values, and post after \
             vce(cluster), are refused. Where a parameter is near zero the log can take other steps than \
             Stata's (the estimates agree to about 1e-8).",
        ),
    ],
}
