Guide {
    command: "histogram",
    about: "Histogram",
    points: &[
        p(
            "Bins",
            "Stata's: int(min(sqrt(N), 10 log10 N)) bins from the minimum, unless bin(), \
             width() or discrete say otherwise. R's hist() uses Sturges' rule.",
        ),
        GRAPH,
    ],
}
