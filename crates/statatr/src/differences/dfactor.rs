Guide {
    command: "dfactor",
    about: "Dynamic-factor models, and predict after them",
    points: &[
        p(
            "As Stata 19",
            "dfactor (y1 y2 y3 = , noconstant) (f = , ar(1)): one factor with unit variance, an AR(1) factor or none, \
             diagonal observation errors; dfactor (y1 y2 = , noconstant ar(1)) with autoregressive errors and no \
             factor. The exact likelihood, observed-information standard errors, Wald test, e() and \
             _b[/observable:var(e.y1)]. predict has factors (smethod(onestep|filter)) and xb (the regression on \
             the exogenous variables alone: 0 here).",
        ),
        p(
            "Differences",
            "As for sspace: Stata's iteration log and e(ic) are not reproduced; estimates, standard errors and \
             the log likelihood agree to about 1e-6. Constants and exogenous variables in the equations, \
             more than one factor, ar() above 1, a factor with ar() on the errors, covariance(), constraints(), \
             estat and smoothing are not translated yet.",
        ),
    ],
}
