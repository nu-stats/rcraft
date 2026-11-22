Guide {
    command: "pnorm",
    about: "Standardized normal probability plot",
    points: &[
        p(
            "Points",
            "The normal probability of each standardized value against i/(N+1), with the \
             line y = x and Stata's axes from 0 to 1, as Stata 19 plots them. grid is not \
             translated yet.",
        ),
        GRAPH,
    ],
}
