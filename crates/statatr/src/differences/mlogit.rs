Guide {
    command: "mlogit",
    about: "Multinomial logistic regression",
    points: &[
        p(
            "Base outcome",
            "The most frequent outcome, as in Stata, unless baseoutcome() says; R's \
             multinom() takes the first level.",
        ),
        p(
            "Names",
            "Each coefficient is named outcome:variable; the base outcome's equation is \
             zeros, as in Stata's e(b).",
        ),
        ML_EXACT,
        ML_TABLE,
        ML_ROBUST,
        ML_DIGITS,
        MISSING_IN_MODELS,
    ],
}
