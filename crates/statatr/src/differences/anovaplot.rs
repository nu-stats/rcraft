Guide {
    command: "anovaplot",
    about: "Fitted values after anova, by two factors (anovaplot, Stata Journal gr0009_2)",
    points: &[
        p(
            "In R",
            "After anova, the model's fitted values over its own rows, joined along the first \
             variable for each value of the second (marked at each cell, labelled with its \
             value label), the data as points (hidden by scatter(ms(i)), which still set the \
             y axis), the first variable's values as ticks with their labels, the titles \
             Stata 19 gives, and Profiles by the second variable above; drawn by ggplot2.",
        ),
        p(
            "Not yet",
            "anovaplot after other models, of variables that are not the model's categorical \
             terms, with if or in, or after the data changed are refused.",
        ),
    ],
}
