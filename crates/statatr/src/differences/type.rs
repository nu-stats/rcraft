Guide {
    command: "type",
    about: "Prints a text file",
    points: &[p(
        "As Stata",
        "The file's lines with tabs expanded to stops every 8 columns, <T> with showtabs, \
         the first # with lines(#); a file not there stops with Stata's r(601). Lines \
         longer than the line size are not wrapped (Stata wraps them with > in its \
         log). asis, smcl and starbang are not translated yet.",
    )],
}
