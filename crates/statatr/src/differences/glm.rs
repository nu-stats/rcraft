Guide {
    command: "glm",
    about: "Generalized linear models (maximum likelihood)",
    points: &[
        p(
            "In R",
            "stata_glm fits glm as Stata 19 does by default, matched to its output (its code \
             was not read): one IRLS step from mu = (y + mean(y)) / 2 (binomial: n(y + 0.5) / \
             (n + 1)), then Stata's Newton-Raphson on the observed information, which is \
             the iteration log; OIM standard errors times the scale (1, or Pearson chi2 / df \
             for gaussian and with scale(x2)), robust ones with N / (N - 1), clustered with \
             G / (G - 1). The header (deviance, Pearson, AIC, BIC), the OIM table, \
             offset()/exposure() rows, eform labels and notes, and e() are Stata 19's.",
        ),
        p(
            "Translated",
            "family(gaussian) with the identity link; poisson with log; binomial with logit, \
             probit or cloglog (binomial n or #: logit); nbinomial # with log. fweights, \
             pweights, iweights, aweights (gaussian), robust and clustered errors (without \
             weights), eform, noconstant, scale(x2). predict: mu (the default), xb, eta.",
        ),
        p(
            "Not yet",
            "irls (Stata's IRLS stops by its own rule on the deviance), gamma and inverse \
             Gaussian (Stata's default scale there is not Pearson / df exactly), gaussian with \
             other links, other links and families, scale(dev) and numeric scales. \
             Where the likelihood is very flat (a near-perfect probit), Stata's own steps end \
             a little elsewhere: the last digits can differ.",
        ),
    ],
}
