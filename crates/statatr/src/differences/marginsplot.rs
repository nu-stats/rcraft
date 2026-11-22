Guide {
    command: "marginsplot",
    about: "Plots the margins margins left",
    points: &[
        p(
            "In R",
            "After margins with one at() of lists and no terms or effects (not posted): \
             Stata 19's line naming the variables at() sets to several values (in at()'s \
             order), the margins drawn against the first with their intervals, a line for \
             each value of the others, by ggplot2 (so the look is R's). r() stays margins', \
             as in Stata.",
        ),
        p(
            "Not yet",
            "marginsplot after margins with terms, dydx() and the other effects, several \
             at(), or posted, and its options other than the titles and noci.",
        ),
    ],
}
