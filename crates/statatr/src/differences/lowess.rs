Guide {
    command: "lowess",
    about: "Lowess smoothing",
    points: &[
        p(
            "Smoother",
            "Stata's lowess: a local line with tricube weights over the nearest 80% of the \
             data, with no robustness steps. R's lowess() adds three robustness steps.",
        ),
        GRAPH,
    ],
}
