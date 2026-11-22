Guide {
    command: "bootstrap",
    about: "Repeats a command on samples drawn with replacement",
    points: &[
        p(
            "Draws",
            "Each replication resamples the command's estimation sample with Stata's own \
             generator, as bsample does, so the same seed gives Stata's standard errors. The \
             replicates are kept as floats, as Stata keeps them, and the observed values as \
             they are.",
        ),
        p(
            "Options",
            "strata() resamples within each stratum as Stata does (e(N_strata), and the \
             strata in the header); saving(file [, replace double]) writes the replicates \
             as a dataset, a variable each labelled with its expression (_b[x] is _b_x), \
             floats unless double; a file already there without replace is r(602) before \
             anything runs. estat bootstrap shows the bias and the normal, percentile and \
             bias-corrected intervals (bc by default, all for the three).",
        ),
        p(
            "Not yet",
            "Statistics are _b after regress, logit or probit, or r() and e() values; bca, \
             strata() with cluster(), saving()'s every(), predict after it and \
             vce(bootstrap) are refused with the reason.",
        ),
    ],
}
