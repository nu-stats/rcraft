Guide {
    command: "parmest",
    about: "Saves the estimates as a dataset (the SSC package)",
    points: &[
        p(
            "In R",
            "stata_parmest saves the dataset as the package does, matched to the files it \
             saves (not its code): one row per column of e(b), with idnum, eq (when e(b) has \
             equation names), parm, estimate, stderr, z, p, min95 and max95 and its labels; a \
             base level is 0 with z and p missing. The data in memory stay as they are.",
        ),
        p(
            "Not yet",
            "parmest without saving(), after a model with t tests, and its other options.",
        ),
    ],
}
