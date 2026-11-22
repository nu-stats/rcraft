Guide {
    command: "tsset",
    about: "Declares time-series data, for L., F., D. and S.",
    points: &[
        p(
            "In R",
            "stata_tsset checks the times (whole numbers, none repeated within a panel, none \
             closer than delta()), sorts the data by panel and time and prints Stata's lines. \
             L2.x is stata_ts(): x at time t - 2 delta in the same panel, missing when no row \
             has that time, worked out over every row whatever if keeps. A chain is one sum \
             over x, as in Stata (D.F.x is F.x - x).",
        ),
        p(
            "Models",
            "In a model the operators become columns of its data named as Stata names the \
             coefficients (LD.x, L2.x); L(1/2).x and L.(x y) are written out first. predict \
             adds the same columns.",
        ),
        p(
            "Not sorted",
            "When the data are not in panel and time order, Stata stops with \"not sorted\" \
             (r(5)); so does the R. Under by, the operators are translated only when by \
             names the panel variable.",
        ),
        p(
            "tsfill",
            "A row for each time missing between a panel's first and last (from the data's \
             first to its last for every panel with full), its other variables missing and \
             text \"\", the data in panel and time order, as Stata 19 does.",
        ),
        p(
            "Not yet",
            "replace that reads its own variable at other times (replace x = L.x + 1) runs \
             row by row in Stata and is not translated yet; nor are the operators on a string \
             variable, two seasonal differences (S.S.x), or a time unit's dates in the output.",
        ),
    ],
}
