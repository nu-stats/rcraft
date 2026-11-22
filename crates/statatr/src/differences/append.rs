Guide {
    command: "append",
    about: "Adds the rows of another file",
    points: &[p(
        "Types",
        "A string variable in one file and a number in the other stops with an error, as \
         in Stata; R's rbind() would turn both into text. With force the using file's \
         values of such a variable are missing (\"\" or .), as in Stata, which also prints \
         a note for each.",
    )],
}
