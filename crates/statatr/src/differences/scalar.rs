Guide {
    command: "scalar",
    about: "Scalars",
    points: &[p(
        "Names",
        "A variable comes before a scalar of the same name, as in Stata. R cannot see a \
         file's columns before it runs, so a command that reads a scalar by name checks \
         the data first and stops if a variable has that name; scalar(name) is always the \
         scalar.",
    )],
}
