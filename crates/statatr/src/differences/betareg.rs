Guide {
    command: "betareg",
    about: "Beta regression",
    points: &[
        p(
            "As Stata 19",
            "betareg y x..., scale(), link() (logit, probit, cloglog, loglog) and slink() (log, root, identity): \
             Stata's own path (the Initial, Rescale and Rescale eq lines, ten BHHH iterations, then Newton-Raphson on \
             a Hessian from central differences of the analytic scores), the LR (Wald with robust) test, the link \
             lines, vce(robust), vce(cluster), level(), if and in, fweights, pweights, iweights, e(). predict gives \
             cmean, cvariance, xb, xbscale and stdp.",
        ),
        p(
            "Not yet",
            "e(ll_0) differs from Stata's from the seventh digit (Stata's constant-only fit stops short of the \
             maximum), so the LR test's p-value agrees to 6 digits; factor variables in scale(), from().",
        ),
    ],
}
