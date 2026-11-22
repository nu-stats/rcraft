Guide {
    command: "manova",
    about: "Multivariate analysis of variance: the four statistics for each term",
    points: &[
        p(
            "As Stata 19",
            "manova y1 y2 y3 = g##h c.x: a variable is a factor unless written c.x, each term tested by the four \
             statistics (Wilks with Rao's F, Pillai, Lawley-Hotelling, Roy as an upper bound unless s is 1) on the \
             Type III change in the residual cross products. e() is Stata's, including b, V (reference coding, base \
             levels as zeros), E, H_#, B, xpxinv, stat_#, aux_# and eigvals_#; checked against Stata 19 to 1e-9.",
        ),
        p(
            "Differences",
            "The table is printed in R's layout of Stata's. Weights, options, nesting and bases (ib#.) are not \
             translated; manovatest reads this e().",
        ),
    ],
}
