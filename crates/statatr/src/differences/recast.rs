Guide {
    command: "recast",
    about: "Changes a variable's storage type",
    points: &[
        p(
            "As Stata 19",
            "byte, int, long, float and double: with force a value the type cannot hold is \
             truncated toward zero or rounded to float, and one out of range becomes \
             missing; without it nothing changes when a value would, and Stata's count of \
             the values is shown.",
        ),
        p("Not yet", "String types."),
    ],
}
