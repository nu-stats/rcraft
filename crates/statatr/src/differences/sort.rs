Guide {
    command: "sort",
    about: "Sorts the rows",
    points: &[p(
        "Ties",
        "Stata's sort puts rows that tie in no set order unless stable; R keeps them in \
         their current order. Missing values sort last, as in Stata.",
    )],
}
