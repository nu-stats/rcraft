Guide {
    command: "xtreg",
    about: "Panel regression",
    points: &[
        p(
            "Estimators",
            "re is Swamy-Arora random effects with z tests, as Stata's default; fe is the \
             within regression with the constant as the grand mean's; be the regression on \
             the panel means; mle and pa as Stata's (pa iterated until b changes by less \
             than 1e-6, where Stata stops).",
        ),
        p(
            "Robust errors",
            "Clustered on the panel (or the cluster variable), scaled by G / (G - 1) and \
             (N - 1) / (N - k), as Stata's. With fewer clusters than slopes the variance is \
             not of full rank, and the Wald test is missing, as in Stata.",
        ),
        p(
            "Collinearity",
            "A variable the constant and the ones before it explain is omitted, the later one \
             of a collinear set, as Stata omits it; fe also omits one that does not vary \
             within the panels. When re has more columns than panels, sigma_u is 0 and re is \
             least squares, as in Stata. A variable named twice is kept once, the second an \
             omitted column after it (o.x), as Stata keeps it.",
        ),
        p(
            "Differences",
            "fe with variables collinear only within the panels (age with time dummies): one \
             must be left out, and the data cannot tell which. RCraft leaves out the one \
             Stata leaves out: the later one, or, clustered on another variable than the \
             panel, the one its largest-pivot sweep leaves with nothing ('collinearity with \
             the absorbed variable'). Where two columns are left with the same (time \
             dummies of one year, say), Stata's rounding decides between them; RCraft then \
             leaves out the earlier, and a note names the coefficients that may differ \
             (those of the set and _cons). Every other result is the same whichever is \
             left out: the other coefficients and their standard errors, N, the \
             R-squareds, sigma_u, sigma_e, rho, F and the predictions.",
        ),
        MISSING_IN_MODELS,
        LAYOUT_R,
    ],
}
