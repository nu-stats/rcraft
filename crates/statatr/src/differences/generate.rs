Guide {
    command: "generate",
    about: "Makes a new variable",
    points: &[
        p(
            "Float",
            "Without a type the new variable is a float, as in Stata: the translation rounds \
             it to float precision (about 7 digits).",
        ),
        p(
            "Missing values",
            "A row the if leaves out is missing (.), or empty text for a string, as in Stata, \
             and Stata's note says how many: (6 missing values generated).",
        ),
    ],
}
