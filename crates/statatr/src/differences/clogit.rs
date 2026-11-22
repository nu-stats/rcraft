Guide {
    command: "clogit",
    about: "Conditional (fixed-effects) logistic regression",
    points: &[
        p(
            "In R",
            "stata_clogit fits the exact conditional likelihood, as Stata does, with R's \
             survival::coxph (one stratum per group, method = \"exact\"); several positive \
             outcomes in a group are fine. Groups that are all positive or all negative are \
             dropped with Stata's note, and so is a variable that does not vary within any \
             group. With one positive outcome a group, the estimates are where Stata's \
             Newton-Raphson stops (from an ordinary logit's slopes, Stata's steps and \
             stopping rule), which can be a few parts in 1e7 short of the maximum.",
        ),
        p(
            "Robust errors",
            "survival gives no robust variance with the exact method, so the scores are \
             worked out from each group's probabilities. vce(robust) clusters on the group, \
             as in Stata; with vce(cluster v) the groups must be nested in the clusters \
             (r(459)). Both carry G / (G - 1).",
        ),
        p(
            "predict",
            "pc1 (the default: the probability of being the positive one within the group, \
             on every row), pu0 and xb.",
        ),
        p(
            "Not yet",
            "Weights, offsets and the other predict statistics; margins after clogit.",
        ),
    ],
}
