Guide {
    command: "cmclogit",
    about: "Conditional logit choice model (McFadden's), with margins after it",
    points: &[
        p(
            "In R",
            "stata_cmclogit fits the conditional logit by Newton-Raphson in base R: each \
             alternative-specific term (x, i.x, ib#.x) one column, each case-specific term \
             of casevars() one coefficient for each alternative but the base one, which is \
             the one most frequent in the data (the first of a tie), as in Stata; each such \
             equation named by the alternative's value, a constant in each unless \
             noconstant. vce(cluster v) is H^-1 (sum of the clusters' scores) H^-1 times \
             G / (G - 1). e() and the table are Stata 19's; its iteration log is not shown.",
        ),
        p(
            "margins",
            "margins, dydx(i.x ...) outcome(#) alternative(#) asobserved post after it: for \
             each level the mean over cases of Pr(outcome) with the alternative's x at that \
             level less with it at the base, with the analytic gradient; vce(unconditional) \
             clusters each case's term as Stata does, the default is the delta method. e(b) \
             has an equation a level (1.x:_cons, the base 3b.x:o._cons), as Stata's.",
        ),
        p(
            "Not yet",
            "Weights, vce(robust), interactions, margins without post or of continuous \
             terms, and predict, test and lincom after cmclogit are refused.",
        ),
    ],
}
