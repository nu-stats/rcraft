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
        ML_SE,
        ML_TABLE,
        ML_ROBUST,
        ML_DIGITS,
        MISSING_IN_MODELS,
    ],
}
