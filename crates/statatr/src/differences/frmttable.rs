Guide {
    command: "frmttable",
    about: "A table from a matrix (frmttable, part of outreg)",
    points: &[
        p(
            "As Stata 19",
            "statmat() store() with sfmt() of g and f and sdec(): f with sdec() decimals \
             (2), g a whole number, a missing cell empty, no row titles; for outreg's \
             replay() and merge().",
        ),
        p("Not yet", "g of a number with decimals, row and column titles, and writing a file."),
    ],
}
