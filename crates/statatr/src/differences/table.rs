Guide {
    command: "table",
    about: "Tables of frequencies and statistics (Stata 17's table)",
    points: &[
        p(
            "In R",
            "stata_table lays the table out as Stata 19 does: rows (one variable, or two \
             nested) and at most one column variable, frequency (Sum of weights with \
             fweights), percent, mean, sd, median and p#, min, max, sum, count and iqr, each \
             column as wide as its widest entry, a blank cell where there are no rows and . \
             where there are no values, totals unless nototals, missing as a level of its \
             own, nformat() for some statistics or all.",
        ),
        p(
            "Not yet",
            "A third dimension, several statistics of several variables, and the options for \
             styles, totals() and the collect system are refused; contents() is Stata's own \
             error since Stata 17.",
        ),
    ],
}
