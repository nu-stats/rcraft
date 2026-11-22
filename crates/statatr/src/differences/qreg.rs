Guide {
    command: "qreg",
    about: "Quantile regression",
    points: &[
        p(
            "As Stata 19",
            "qreg y x..., quantile(#) nolog (quantile(75) means .75) minimises the sum of q|r| above and \
             (1 - q)|r| below by descending the edges of the linear programme's vertices, the iid variance \
             as Stata's (sparsity from fits at q +/- h with Hall and Sheather's bandwidth, z from level()), \
             the header with the raw and minimum sums of deviations, the pseudo R2 and e() (rank, sparsity, \
             bwidth, df_m, df_r, f_r, q_v, sum_rdev, sum_adev, r2_p, b, V). fweights, factor variables, \
             if, in and level() are taken; aweights, pweights and iweights stop as Stata stops (r(101)). \
             predict gives xb and residuals.",
        ),
        p(
            "Not yet",
            "Fits where the minimum is not unique (often with dummy regressors at the median): Stata ends at \
             one of several optimal vertices by its own path, so RCraft stops with a message instead of \
             printing a vertex that might differ. Stata's iteration log (use nolog), vce(), other \
             density and bandwidth methods, interactions and collinear regressors.",
        ),
    ],
}
