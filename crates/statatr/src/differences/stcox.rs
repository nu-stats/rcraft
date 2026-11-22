Guide {
    command: "stcox",
    about: "Cox regression",
    points: &[
        p(
            "Ties",
            "Breslow's method by default, as Stata's; R's coxph() defaults to Efron's. \
             efron and exactp are translated as asked.",
        ),
        p(
            "Robust errors",
            "Robust errors carry N / (N - 1) and clustered ones G / (G - 1), as Stata's.",
        ),
        LAYOUT_R,
    ],
}
