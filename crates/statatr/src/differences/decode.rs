Guide {
    command: "decode",
    about: "A value-labelled variable to its labels as text",
    points: &[
        p(
            "As Stata 19",
            "decode var, generate(newvar), with if and in: each value's label, \"\" for a \
             value without one, a missing value or a row left out; the variable's current \
             value labels (after a second label values too); Stata's errors for a string \
             variable, one without labels and a variable that exists.",
        ),
        p("Not yet", "maxlength()."),
    ],
}
