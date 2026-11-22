Guide {
    command: "fre",
    about: "Frequency tables (the SSC package)",
    points: &[
        p(
            "In R",
            "stata_fre prints fre's table as the package prints it, matched to its output \
             (not its code): the variable's name and label, each value with its value label, \
             Valid, Missing and Total rows, Freq., Percent, Valid and Cum.; a string's empty \
             value is missing. It leaves r(N), r(N_valid), r(N_missing), r(r), r(r_valid), \
             r(r_missing) and r(depvar).",
        ),
        p(
            "Missing values",
            "Stata's extended missing values (.a to .z) are one missing value in R, so they \
             are one Missing row.",
        ),
        p("Not yet", "fre's options and weights; r(lab_valid) and the matrices."),
    ],
}
