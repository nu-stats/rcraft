Guide {
    command: "erase",
    about: "Deletes a file (rm is the same)",
    points: &[p(
        "Missing file",
        "A file that is not there stops with Stata's message, file ... not found, and \
         capture gives _rc 601, as in Stata. No extension is added: erase x looks for x, \
         not x.dta.",
    )],
}
