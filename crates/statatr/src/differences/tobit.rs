Guide {
    command: "tobit",
    about: "Censored (tobit) regression",
    points: &[
        p(
            "As Stata 19",
            "tobit y x..., ll(#) ul(#) as Stata fits it: its gsem-style log (Refining starting values, Grid node 0, \
             Fitting full model), the least squares start with sigma^2 as the parameter, ml's Newton steps, the \
             header with the censoring counts and limits, the LR test (F with robust, pweights or clusters, on N - k \
             degrees of freedom), var(e.y) with its interval, e(). fweights, pweights, iweights, vce(robust), \
             vce(cluster), level(), nolog, factor variables, replay, offset(), iterate() (the log, `convergence not \
             achieved` and the warning), from(matrix) naming the parameters (skip; copy stops as Stata stops), \
             stepwise and nestreg (the F and LR block tables, the Wald and LR searches). predict gives xb, stdp, \
             stdf, pr(a,b), e(a,b), ystar(a,b).",
        ),
        p(
            "As Stata 19 (estat)",
            "estat ic (r(S), n(), df()), estat vce (covariance and correlation, format(), r(V)) and estat summarize \
             (a varlist, equation, labels, noheader, the weights' row, clusters, r(stats)) in Stata's layout; every \
             other estat subcommand stops as Stata stops it (r(321)).",
        ),
        p(
            "Not yet",
            "aweights (each row's own variance), noconstant (the estimates, log likelihood and F test match, but \
             Stata's starting values do not, so its log), constraints(), interactions and ib. bases, stepwise and \
             nestreg with weights other than fweights or with robust variances in stepwise, estat vce and \
             summarize with factor variables, margins.",
        ),
    ],
}
