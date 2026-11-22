Guide {
    command: "reghdfe",
    about: "Regression absorbing fixed effects (the SSC package)",
    points: &[
        p(
            "In R",
            "stata_reghdfe fits reghdfe as the package does, matched to its results (its code \
             was not read): singletons dropped (again until none are left) unless \
             keepsingletons, the absorbed means taken out, _cons with the grand means added \
             back, and its degrees of freedom: an absorbed variable counts its levels, the \
             second one less one for each connected group the two make, and one nested in a \
             cluster variable counts none.",
        ),
        p(
            "Errors",
            "Unadjusted, vce(robust) (times N / (N - K)), clustered (times (N - 1) / (N - K) \
             G / (G - 1), t tests on G - 1) and two-way clustered (the two less their \
             intersection, G the smaller count, with Cameron, Gelbach and Miller's fix when \
             that is not positive). As reghdfe makes the fix: on the slopes alone, each in \
             units of its standard deviation, and the constant's row from the fix of the \
             whole matrix; F is missing when the slopes' variance falls short of full rank, \
             and a variance with a covariance larger than its variances allow is posted as \
             zero (errors missing), as Stata 19 does.",
        ),
        p(
            "Rounding",
            "With one slope and the fix applied, Stata posts e(V) or zeroes it as its own \
             rounding of a correlation of exactly 1 falls (22 of 30 such models kept their \
             errors in one replication package); RCraft always keeps them, so Stata may show \
             missing errors where RCraft shows numbers.",
        ),
        p(
            "Not yet",
            "A third absorbed variable not nested in a cluster variable, interactions in absorb(), savefe and resid, \
             weights, and predict after reghdfe.",
        ),
    ],
}
