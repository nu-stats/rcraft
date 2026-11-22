Guide {
    command: "pwcompare",
    about: "Pairwise comparisons of marginal linear predictions (pwcompare g, effects)",
    points: &[
        p(
            "In R",
            "The same machinery as contrast: every pair of cells of g or g#h, with effects, \
             cimargins, level() and mcompare(bonferroni, sidak, scheffe); (not estimable) rows \
             where the design cannot tell the margins apart; r(b_vs), r(V_vs), r(table_vs) for \
             the comparisons and r(b), r(V), r(table) for the margins.",
        ),
        p(
            "Not yet",
            "tukey, snk, duncan, dunnett, groups, sort, post; r(groups#), r(L), r(M).",
        ),
    ],
}
