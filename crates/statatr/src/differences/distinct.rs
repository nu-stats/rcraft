Guide {
    command: "distinct",
    about: "Counts the distinct values of each variable (distinct, SSC)",
    points: &[
        p(
            "As Stata 19",
            "For each variable (all of them without a varlist) the rows and the distinct values \
             among the rows of if and in, a missing value or an empty string left out unless \
             missing; minimum() and maximum() choose the variables shown (swapped, with the \
             add-on's message, when the minimum is the larger); joint counts the combinations \
             of the varlist over the rows with no missing value; abbrev() cuts long names to \
             at least 5 characters with a ~. r(ndistinct) and r(N) are those of the last \
             variable. Its error messages and return codes are the add-on's. Observed on \
             made-up data; the add-on's program was not read.",
        ),
        p(
            "Not yet",
            "in with no single variable (the add-on counts the later variables oddly there), \
             minimum() or maximum() with joint, variable patterns, weights.",
        ),
    ],
}
