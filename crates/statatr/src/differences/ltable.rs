Guide {
    command: "ltable",
    about: "Life tables for individual-level or aggregate data",
    points: &[
        p(
            "Method",
            "Actuarial intervals [kw, (k + 1)w): those lost count for half the interval, the \
             survivor function is the product of 1 - d / n', Greenwood's variance gives the \
             standard error, the interval is on the log-log scale. failure, hazard, noadjust, \
             level(), intervals(), by(), fweight and test (the likelihood-ratio statistic of \
             the exponential model, then the log-rank test; r(chi2) and r(df) are the log-rank's).",
        ),
        p(
            "Not yet",
            "graph, overlay, tvid() and saving() are refused with the reason.",
        ),
        LAYOUT_R,
    ],
}
