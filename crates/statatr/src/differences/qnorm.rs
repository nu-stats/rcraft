Guide {
    command: "qnorm",
    about: "Normal quantile plot",
    points: &[
        p(
            "Points",
            "The sorted values against the mean plus the standard deviation times the \
             normal quantile of i/(N+1), with the line y = x, as Stata 19 plots them. R's \
             qqnorm() uses other plotting positions. grid is not translated yet.",
        ),
        GRAPH,
    ],
}
