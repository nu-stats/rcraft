Guide {
    command: "ucm",
    about: "Unobserved-components models (local level, random walk, local linear trend), and predict after them",
    points: &[
        p(
            "As Stata 19",
            "ucm y [x], model(llevel | rwalk | lltrend): the components' variances and the noise by the diffuse \
             Kalman filter, regressors as coefficients, e(), _b[/:var(level)] and the Wald test of the regressors. \
             predict has xb (the one-step prediction, regressors included) and trend (the level, with \
             smethod(onestep) or smethod(filter)); the first period, or the first two for lltrend, is missing \
             where the filter is diffuse.",
        ),
        p(
            "Differences",
            "As for sspace: Stata's own iteration log and e(ic) are not reproduced; estimates, standard errors \
             and the log likelihood agree to about 1e-6. Other models, cycles, seasonals, vce(), from(), \
             model() left out, estat and smoothing are not translated yet.",
        ),
    ],
}
