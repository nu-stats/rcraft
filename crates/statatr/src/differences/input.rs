Guide {
    command: "input",
    about: "Data typed into the do-file",
    points: &[p(
        "As Stata 19",
        "The lines up to end become the data, into empty data: float unless a type says \
         otherwise (double(a b) types a group), strings as written, . and .a missing. \
         Stata's echo of the lines is not printed. A value a numeric variable cannot \
         read, a line with too few or too many values, and input into data already in \
         memory are declined.",
    )],
}
