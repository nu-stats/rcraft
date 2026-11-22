Guide {
    command: "statsby",
    about: "Collects a command's results for each group of the data",
    points: &[
        p(
            "Groups",
            "The command runs on each group's rows, in ascending order of the by() variables; \
             a missing by value is left out unless by(vars, missing); total adds a last row. \
             The data become the results, floats labelled with their expressions.",
        ),
        p(
            "Not yet",
            "Without clear, saving(), subsets and _b or _se after logit or probit (Stata names \
             those variables after the equation) are refused with the reason.",
        ),
    ],
}
