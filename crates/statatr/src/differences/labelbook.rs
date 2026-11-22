Guide {
    command: "labelbook",
    about: "Describes value label sets and their problems",
    points: &[
        p(
            "As Stata 19",
            "labelbook [names] [, problems length(#)] prints a page per set (range, gaps, string lengths, uniqueness, \
             blanks, numeric labels, the definition and the variables that use it) or the table of problems, and \
             leaves r(names), r(gaps), r(notused), r(numeric), r(blanks), r(nuniq) and r(nuniq_sh).",
        ),
        p(
            "In R",
            "It knows the sets the do-file defined. After a data file was read, or for a set with an empty label, it is \
             refused: R does not keep the names of the sets that came with a file. The heading says [unnamed]; list() \
             and the other options are refused.",
        ),
    ],
}
