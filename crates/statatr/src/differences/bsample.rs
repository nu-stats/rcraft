Guide {
    command: "bsample",
    about: "Replaces the data with a sample drawn with replacement",
    points: &[
        p(
            "Draws",
            "The rows are drawn with Stata's own generator after set seed, so the same seed \
             gives the rows Stata's bsample gives, in its order, and the draws after it go on \
             as Stata's do. R's sample() would draw other rows.",
        ),
        p(
            "strata()",
            "The data sorted by stratum, then Stata's draws with the bins taken within each \
             stratum and a third uniform a row ordering them, so the rows come in Stata's \
             order; with a size, the first that many of each stratum.",
        ),
        p(
            "Not yet",
            "weight(), strata() with cluster(), if or in, and if or in together with a \
             sample size or cluster() are refused with the reason.",
        ),
    ],
}
