Guide {
    command: "lrtest",
    about: "Likelihood-ratio test of nested models (lrtest full small)",
    points: &[
        p(
            "In R",
            "stata_lrtest takes twice the difference of e(ll) on the difference of e(rank) for \
             two stored fits (one name: the other is the active fit) of logit, probit, poisson \
             or regress, with Stata's stops for the same model twice and for equal degrees of \
             freedom. r(p), r(chi2), r(df).",
        ),
        p(
            "Not yet",
            "Robust or weighted fits, stats, force, using, and models of other commands.",
        ),
    ],
}
