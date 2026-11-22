Guide {
    command: "insheet",
    about: "Reads a tab or comma file",
    points: &[p(
        "As Stata 19 reads it",
        "A tab in the first line makes it tab, else comma. Names come from the first row \
         when no cell of it is a number (lowercase, what is not a letter, digit or _ left \
         out, the header kept as the label where it differs; case keeps the case), v1, v2, \
         ... when every cell is one; decimals are floats unless double. A first row of \
         both, headers Stata would have to make names up for, and an empty column stop \
         rather than guess.",
    )],
}
