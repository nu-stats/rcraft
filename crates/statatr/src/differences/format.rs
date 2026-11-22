Guide {
    command: "format",
    about: "Sets how variables are shown",
    points: &[p(
        "In R",
        "The display format is kept as the variable's format.stata attribute, which haven \
         writes to a saved .dta file; the values do not change, and R's own printing does \
         not use it. Patterns and ranges of variables are not translated yet.",
    )],
}
