Guide {
    command: "revrs",
    about: "Reverses a variable's coding (revrs, SSC)",
    points: &[
        p(
            "In R",
            "stata_revrs does what Stata 19 does with revrs, matched to its output: with a \
             value label, the labelled values in the data take the codes k to 1 in order and \
             the label set rev<name> keeps their texts (a value without a label is missing); \
             without one, (max + 1) - x with missing values kept. rev<name> is new, or with \
             replace the variable itself. Stata's errors come through capture as r(111), \
             r(109) and r(110).",
        ),
        p(
            "Not yet",
            "Values that are not whole numbers or halves (Stata's float arithmetic there was \
             not matched), value labels that name missing values, if and in are refused.",
        ),
    ],
}
