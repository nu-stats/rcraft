Guide {
    command: "predict",
    about: "Predictions after a model",
    points: &[
        PREDICT_ROWS,
        p(
            "Defaults",
            "One new variable and no outcome() after mlogit or ologit is the first \
             outcome's probability, as in Stata.",
        ),
    ],
}
