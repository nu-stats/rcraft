Guide {
    command: "coefplot",
    about: "Plots stored models' coefficients and intervals (SSC)",
    points: &[
        p(
            "In R",
            "stata_coefplot_rows works out the rows as coefplot does, from Stata's names in \
             e(b): _cons included, base levels with baselevels and omitted terms with omitted \
             (both 0), in the order they first appear with _cons last, keep(), drop() and \
             order() with its empty rows; intervals with the model's residual df (the normal \
             without one) at levels() (or ci() of numbers); several plots 1/(k+1) apart, offset() moving one; \
             headings(); subgraphs at ||. gen() leaves those rows, as floats, as coefplot does.",
        ),
        p(
            "Several equations",
            "A model with several equations (margins posted after cmclogit) is drawn alone \
             and with noeqlabels: its rows in e(b)'s order, only the first equation's unless \
             keep() says otherwise; keep(), headings() and coeflabels() read eq: names as \
             Stata 19 does (the equation kept for the names after it, eq: alone all its \
             names). Equation labels, order() and several plots of such models are refused.",
        ),
        p(
            "Look",
            "The graph is drawn by ggplot2, so it looks like R's; colours, markers and the \
             like are not carried over. Labels are the variable's label or name, a factor \
             level's value label or g=2.",
        ),
    ],
}
