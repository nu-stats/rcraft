Guide {
    command: "predictnl",
    about: "Nonlinear predictions with delta-method errors (predictnl p = invlogit(xb()), se(s))",
    points: &[
        p(
            "In R",
            "stata_predictnl works the expression out at each row (data variables, _b[], xb(), \
             predict()), with the standard error by the delta method on numerical derivatives \
             of the coefficients, and se(), variance(), wald(), p(), ci() and level(); new \
             variables are float unless double; Stata's notes and warnings.",
        ),
        p(
            "Not yet",
            "After models other than regress, logit, probit and poisson, predict() types other \
             than xb, pr and n, force, iterate() and g().",
        ),
    ],
}
