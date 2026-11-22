Guide {
    command: "filefilter",
    about: "Replaces bytes in a file",
    points: &[
        p(
            "As Stata 19",
            "Every occurrence of from() becomes to(), left to right and not overlapping, the \
             rest copied byte for byte; the codes \\BS \\Q \\LQ \\RQ \\$ \\t \\n \\r \\W \\U \\M and \
             \\###d; r(occurrences), r(bytes_from), r(bytes_to); replace, and the errors for \
             a missing file, a file that exists, the same file and an unknown code.",
        ),
        p("Not yet", "Hexadecimal codes (\\##h)."),
    ],
}
