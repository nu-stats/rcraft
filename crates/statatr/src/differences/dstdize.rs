Guide {
    command: "dstdize",
    about: "Direct standardization of rates",
    points: &[
        p(
            "As Stata 19",
            "dstdize cases pop strata, by(group) takes the standard population as every row's \
             population by stratum, and gives each group its adjusted rate (the sum of the \
             standard share times the stratum rate), crude rate, normal interval, the tables \
             and r() with the matrices adj, crude, se, lb_adj, ub_adj and Nobs and the groups' \
             names. level() is taken.",
        ),
        p(
            "Precision",
            "Stata keeps the adjusted rate and its standard error as floats and sometimes \
             lands one float step (about 7e-8 relative) from the rounded double result, in a \
             way not found; r() agrees to about 1e-7.",
        ),
        p(
            "Not yet",
            "Several by() or strata variables, using(), base(), saving(), format(), print, nore \
             and weights are refused.",
        ),
    ],
}
