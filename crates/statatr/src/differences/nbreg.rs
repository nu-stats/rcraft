Guide {
    command: "nbreg",
    about: "Negative binomial regression",
    points: &[
        p(
            "Alpha",
            "alpha is 1 / theta of MASS::glm.nb; the standard errors come from the whole \
             likelihood, alpha included, as Stata's (glm.nb's own leave theta out).",
        ),
        ML_TABLE,
        ML_ROBUST,
        ML_DIGITS,
        MISSING_IN_MODELS,
    ],
}
