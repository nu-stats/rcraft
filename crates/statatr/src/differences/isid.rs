Guide {
    command: "isid",
    about: "Checks that variables identify the observations",
    points: &[
        p(
            "As Stata 19",
            "Nothing when they do; Stata's r(459) messages when a variable is missing (missok \
             allows it) or two observations share the values.",
        ),
        p("Not yet", "sort, if and in."),
    ],
}
