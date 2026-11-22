Guide {
    command: "codebook",
    about: "Describes variables",
    points: &[
        p(
            "As Stata 19",
            "For each variable: its label, range and units, unique and missing values, then \
             a tabulation when it has 9 values or fewer, or else the mean, standard \
             deviation and Stata's percentiles; if and in. Printed in R's layout.",
        ),
        p(
            "Not yet",
            "Options; the storage type (R does not keep Stata's) and the value label's name; \
             r(), which Stata fills for some checks.",
        ),
    ],
}
