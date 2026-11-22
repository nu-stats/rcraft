Guide {
    command: "tabm",
    about: "One table of several variables' values (tabm, tab_chi from SSC)",
    points: &[
        p(
            "In R",
            "stata_tabm stacks the variables and prints tabulate's table of variable (a row \
             each, in the varlist's order) by values (a column each, labelled with the first \
             variable's value label, cut at 9 characters), split into blocks as Stata 19 does; \
             if and missing; r(N), r(r) and r(c).",
        ),
        p(
            "Not yet",
            "in, weights, string variables, missing with .a to .z in the data, and tabm's \
             other options are refused.",
        ),
    ],
}
