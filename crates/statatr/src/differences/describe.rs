Guide {
    command: "describe",
    about: "Describes the data",
    points: &[
        p(
            "Storage types",
            "R has no storage types (byte, int, float, str20), so the description lists R's \
             classes instead, and r(width) and r(changed) are refused.",
        ),
        p(
            "Chosen variables",
            "describe varlist (names, ranges, patterns) lists only those, in the data's order; \
             r(k) stays the number of variables in the data, as in Stata. simple prints the \
             names in columns down the page, as Stata does; varlist leaves r(varlist). A name not in \
             the data stops with Stata's r(111). describe using a file is not translated yet.",
        ),
        LAYOUT_R,
    ],
}
