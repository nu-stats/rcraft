Guide {
    command: "stepwise",
    about: "Stepwise selection of terms for regress, logit, probit and poisson",
    points: &[
        p(
            "As Stata 19",
            "Backward, forward, hierarchical, lockterm1 and lr searches over variables and \
             groups of variables: the trace, the final model with its terms in the order they \
             were entered, the rows the full model uses, e(cmdline) and e(stepwise), and \
             Stata's errors with their return codes.",
        ),
        p(
            "Differences",
            "Weights, factor variables, other options, a collinear full model, and a search \
             that ends with no term are stopped when they run, saying so; so is a search with \
             both pr() and pe() that could take the other kind of step, whose order is not \
             decoded. The likelihood models agree with Stata to about 7 digits.",
        ),
    ],
}
