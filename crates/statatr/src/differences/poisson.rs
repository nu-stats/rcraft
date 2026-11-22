Guide {
    command: "poisson",
    about: "Poisson regression",
    points: &[
        p(
            "exposure() and offset()",
            "ln(exposure) or the offset enters with coefficient 1, kept in e(offset) as Stata \
             keeps it (as for nbreg); Stata's errors stop it as in Stata: an exposure not \
             above 0, both options, a variable not there, two variables, a string.",
        ),
        ML_CNS,
        p(
            "iterate(#)",
            "logit, probit and poisson take iterate(#) as Stata 19 does: its Newton-Raphson path stops \
             after # steps (iterate(0) shows the start), the log ends with `convergence not achieved` \
             (also under nolog), the estimates, variance, likelihood and LR or Wald test are those where \
             the path stopped, `Warning: Convergence not achieved.` follows the table and e(converged) \
             is 0; a path that has converged inside the cap prints as usual. Not translated: with \
             weights for poisson, noconstant or an offset, with constraints(), and from(), trace and \
             showstep.",
        ),
        ML_SE,
        ML_TABLE,
        ML_ROBUST,
        ML_DIGITS,
        MISSING_IN_MODELS,
    ],
}
