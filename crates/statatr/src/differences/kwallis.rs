Guide {
    command: "kwallis",
    about: "Kruskal-Wallis equality-of-populations rank test (kwallis x, by(g))",
    points: &[
        p(
            "In R",
            "stata_kwallis prints Stata 19's table of each group's observations and rank sum, \
             chi2 with its degrees of freedom and probability, and chi2 with the ties' \
             correction; r() as Stata's (df, chi2, p, chi2_adj, p_adj). chi2 agrees with \
             Stata's to about 2e-7.",
        ),
        p("Not yet", "Weights are refused."),
    ],
}
