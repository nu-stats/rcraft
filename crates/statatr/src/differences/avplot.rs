Guide {
    command: "avplot",
    about: "Added-variable plot",
    points: &[
        p(
            "Points and note",
            "After regress: the residuals of the outcome on the other regressors against \
             those of the one named, with the line of its coefficient and Stata's note of \
             the coefficient, its standard error (robust when the model's is) and t.",
        ),
        GRAPH,
    ],
}
