Guide {
    command: "ds",
    about: "Lists variable names in columns",
    points: &[
        p(
            "As Stata 19",
            "ds [varlist] [, not has(...) not(...) alpha] prints the names in columns like describe, simple and \
             leaves them in r(varlist). not alone leaves out the varlist; has() and not() choose by type string, \
             type numeric, varlabel [pattern] or vallab.",
        ),
        p(
            "In R",
            "The storage types, format, char and vallab patterns, detail, varwidth() and skip() are refused: R keeps \
             none of Stata's types or formats.",
        ),
    ],
}
