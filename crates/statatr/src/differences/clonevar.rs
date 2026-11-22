Guide {
    command: "clonevar",
    about: "Copies a variable with its type, labels and display format",
    points: &[
        p(
            "As Stata 19",
            "clonevar newvar = oldvar [if] [in] keeps the storage type (a float keeps its \
             rounding), display format, variable label and value label, and puts missing where \
             if or in does not hold. It says (N missing values generated) with the count of \
             missing values in the new variable, `.c` and \"\" included. The errors have \
             Stata's messages and return codes: no = (r(198)), nothing after = (r(102)), more \
             than one name or weights (r(101)), a new name that is taken (r(110)) and an old \
             one that is not there (r(111)).",
        ),
        p(
            "In R",
            "The copy is a plain copy of the column with its attributes, so labelled columns \
             stay labelled. Options are refused rather than guessed; by: stops with Stata's r(190).",
        ),
    ],
}
