Guide {
    command: "svmat",
    about: "A matrix's columns as variables",
    points: &[p(
        "As Stata 19",
        "A1, A2, ... by default, the column names with names(col), stub1, stub2, ... with \
         names(stub); floats unless double; observations added when the matrix has more \
         rows than the data, with Stata's message; a name already taken stops (r(110)).",
    )],
}
