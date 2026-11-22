Guide {
    command: "misstable",
    about: "Tables the missing values of numeric variables",
    points: &[
        p(
            "As Stata 19",
            "misstable summarize [varlist] [if] [in] [, all] prints Obs=., Obs>. (.a to .z) and Obs<. with the \
             unique values and the minimum and maximum, and leaves the last variable's r(N_lt_dot), r(N_eq_dot), \
             r(N_gt_dot), r(K_uniq), r(min), r(max) and r(vartype).",
        ),
        p(
            "In R",
            "patterns, tree, nested and the options generate() and exok are refused.",
        ),
    ],
}
