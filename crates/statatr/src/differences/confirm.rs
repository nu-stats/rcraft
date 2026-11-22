Guide {
    command: "confirm",
    about: "Checks that something is there, or is not",
    points: &[p(
        "As Stata 19",
        "[numeric|string] variable (r(111), r(7)), new variable (r(110)), [integer] number \
         (r(7)), file and new file (r(601), r(602)), each stopping with Stata's message and \
         return code, so capture leaves Stata's _rc. A variable may be abbreviated. confirm \
         file is declined after a line not translated that may have written files.",
    )],
}
