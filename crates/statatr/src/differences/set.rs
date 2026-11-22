Guide {
    command: "set",
    about: "Settings",
    points: &[
        p(
            "set obs",
            "With no data, a data frame of that many rows and no variables; with data, new rows at the end, missing (\"\" in a string variable). A number written out is translated; set obs does not drop rows.",
        ),
        p(
            "set seed",
            "stata_set_seed() starts Stata's own generator, so runiform(), rnormal() and \
             rgamma() then give Stata's draws. Its state is kept in R and goes on across \
             lines and typed commands; a line that draws and is not translated stops the \
             draws after it until the next set seed.",
        ),
        p(
            "Other settings",
            "Those that only change how Stata runs or prints (more, linesize, ...) have \
             nothing to do in R.",
        ),
    ],
}
