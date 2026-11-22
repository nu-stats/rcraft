Guide {
    command: "zip",
    about: "Zero-inflated Poisson regression",
    points: &[
        p(
            "As Stata 19",
            "zip y x..., inflate(z...) with Stata's logs (the constant-only model, then the full model), irr, level(), \
             if and in, fweights, pweights, iweights, vce(robust), vce(cluster), tolerance options, e(). aweights stop \
             as Stata stops.",
        ),
        p(
            "Not yet",
            "predict after zip, probit inflation, offset() and exposure(), inflate() with options, factor variables in \
             inflate().",
        ),
    ],
}
