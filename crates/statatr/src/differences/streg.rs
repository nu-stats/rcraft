Guide {
    command: "streg",
    about: "Parametric survival regression after stset",
    points: &[
        p(
            "Method",
            "The exponential, Weibull, lognormal, loglogistic and Gompertz distributions, fitted \
             by maximum likelihood with Newton's method and exact derivatives to its limit. The \
             log likelihood is that of log time (as Stata reports it); late entry (time0(), \
             enter()) takes the log survivor function at the entry time off. Stata stops at its \
             own tolerance, so its last digits can differ unless tolerance() is set tight.",
        ),
        p(
            "Metrics",
            "Hazard ratios by default for the proportional-hazards models (nohr gives the \
             coefficients), time (and tr, the time ratios) for the accelerated-failure-time \
             metric of the exponential and Weibull; the ancillary parameter is /ln_p, \
             /lnsigma, /lngamma or /gamma, with p and 1/p, sigma or gamma below.",
        ),
        p(
            "Variances",
            "robust and vce(cluster v) scale the sandwich by N / (N - 1) or G / (G - 1); with \
             id() the clusters are the subjects. e() holds what Stata's does (b, V, ll, ll_0, \
             chi2, N_sub, N_fail, risk ...).",
        ),
        p(
            "Not yet",
            "Other distributions (generalized gamma), factor variables, strata(), ancillary(), \
             frailty(), offset(), vce(opg), and predict, test, lincom, margins and estat after \
             it are refused with the reason.",
        ),
        LAYOUT_R,
    ],
}
