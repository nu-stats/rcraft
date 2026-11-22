Guide {
    command: "ologit",
    about: "Ordered logistic regression (ologit and oprobit alike)",
    points: &[
        p(
            "Cut points",
            "R's zeta are Stata's /cut1, /cut2, ... with the same sign.",
        ),
        ML_EXACT,
        ML_TABLE,
        ML_ROBUST,
        ML_DIGITS,
        MISSING_IN_MODELS,
    ],
}
