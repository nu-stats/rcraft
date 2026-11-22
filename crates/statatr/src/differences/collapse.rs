Guide {
    command: "collapse",
    about: "Makes a dataset of summary statistics",
    points: &[
        p(
            "Precision",
            "Stata stores means and the like of float variables as floats; R keeps full \
             precision, so the 8th digit can differ.",
        ),
        p(
            "Order and formats",
            "As Stata 19: the by() variables come first in the data's order (not by()'s), \
             the rows sorted as by() names them, and each result keeps its source's display \
             format.",
        ),
        p(
            "Weights",
            "aweights and fweights as Stata 19 uses them: the mean weighted; aweights scaled \
             to each group's rows, so (sum) is the weighted mean times the rows with a value \
             and (count) is those rows (with Stata's note); fweights as copies of the row, \
             percentiles included; (sd) over the weights' sum less one; rawsum, min, max, \
             first and last unweighted.",
        ),
        p(
            "Not yet",
            "pweights and iweights; percentiles with aweights; missing or zero weights (the \
             R stops and says so).",
        ),
    ],
}
