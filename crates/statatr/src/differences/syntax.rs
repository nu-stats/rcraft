Guide {
    command: "syntax",
    about: "Reads a program's call as the program describes it",
    points: &[p(
        "In R",
        "Done as the do-file runs: the call's text sets the locals varlist (or anything, \
         namelist), if, in and one for each option, abbreviated by its capitals \
         (Detail, noCONstant), with real and integer defaults and * for the rest, as \
         Stata sets them; what Stata refuses (a varlist required, an option not allowed) \
         is refused with its message. A varlist with a pattern, weights and passthru \
         options are not translated yet.",
    )],
}
