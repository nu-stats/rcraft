Guide {
    command: "levelsof",
    about: "Lists the distinct values of a variable",
    points: &[
        p(
            "The values",
            "Sorted, each number as Stata keeps it in a macro (%18.0g, so a float 0.1 is \
             .1000000014901161), text in compound quotes unless clean, missing values only \
             with missing; left in r(levels), with r(r) and r(N), as Stata 19 does.",
        ),
        p(
            "local()",
            "The macro is set from r(levels) once the line has run in R, so the lines after it \
             (a foreach over it, most often) use the data's own values.",
        ),
    ],
}
