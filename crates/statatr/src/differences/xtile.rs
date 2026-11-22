Guide {
    command: "xtile",
    about: "Groups a variable by its quantiles",
    points: &[
        p(
            "In R",
            "stata_xtile gives each value 1 plus the number of cut points below it, as Stata \
             does (a cut point repeated by ties counts each time). The cut points are \
             Stata's percentiles of the rows if and in keep (R's quantile type 2; altdef is \
             type 6), or every value of cutpoints()'s variable whatever if keeps. Rows left \
             out and missing values are missing.",
        ),
        p(
            "Label",
            "The new variable is labelled as in Stata (\"4 quantiles of x\", \"x categorized \
             by c\"); it holds whole numbers, as Stata's byte does.",
        ),
        p("Not yet", "Weights."),
    ],
}
