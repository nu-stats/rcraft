Guide {
    command: "expand",
    about: "Copies rows",
    points: &[
        p("The new rows", "Each row's copies go after all the rows, in the rows' order, as Stata 19 puts them; a count is rounded, and one below 1 or missing leaves the row once."),
        p("Notes", "expand says what Stata 19 says: the noninteger, missing, negative and zero counts and the observations created."),
        p("generate()", "The new variable is 0 for the rows there before and 1 for the copies; Stata also gives 0 and 1 value labels, which RCraft does not."),
    ],
}
