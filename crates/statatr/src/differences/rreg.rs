Guide {
    command: "rreg",
    about: "Robust regression (Huber then biweight iterations)",
    points: &[
        p(
            "In R",
            "stata_rreg fits it as Stata 19 does, worked out from its logs and results: least \
             squares first, rows of Cook's distance above 1 left out (silently, as Stata); \
             Huber weights min(1, 2M/|r|), M the residuals' median absolute deviation, until \
             the largest change in a weight is below 0.05; then biweight weights with 4.685 \
             times M/0.6745 until it is below 0.01, each a weighted least-squares fit. The \
             variance is the pseudovalues' rmse^2 (X'X)^-1, F the Wald test of the slopes, \
             e() as Stata's (r2, mss, rss and beta from them). The iteration log, header and \
             table print as Stata's.",
        ),
        p(
            "Not yet",
            "Options (tune(), tolerance(), genwt(), level() and the rest), weights, factor \
             variables, and predict, test and margins after it are refused.",
        ),
    ],
}
