Guide {
    command: "survwgt",
    about: "Raking a weight to control totals (survwgt rake, SSC)",
    points: &[
        p(
            "As Stata 19",
            "survwgt rake w, by() totvars() generate() [maxrep()]: each pass adjusts the \
             dimensions in order to their totals; it stops after the first pass in which no \
             category was 0.001 or more from its total (maxrep() passes at most, 10 by \
             default, with Stata's warning when they are reached). The new variable is a \
             double labelled (raked); a missing weight stays missing, a missing by() value is \
             a category of its own; the add-on's errors for totals not constant in a category \
             and for dimensions whose totals differ. The data are left sorted by the last \
             dimension; rows that tie keep their order in R, which Stata does not promise.",
        ),
        p(
            "Not yet",
            "With several dimensions and totals in the thousands Stata can stop a pass or more \
             sooner, by a rule not pinned down yet: R stops with a message there rather than \
             guess (one dimension is exact either way). The add-on's other subcommands, if, in, \
             and the values it leaves in r().",
        ),
    ],
}
