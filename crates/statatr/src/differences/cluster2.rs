Guide {
    command: "cluster2",
    about: "Least squares with errors clustered two ways (Petersen's add-on)",
    points: &[
        p(
            "As Stata 19",
            "Matched to the add-on's results, not its code: the variance clustered by \
             fcluster() plus the one clustered by tcluster() less the one clustered by their \
             cells, each with Stata's G/(G-1) (N-1)/(N-K); with one row per cell the last is \
             the robust one. e(F) and e(df_r) are the cell-clustered fit's (the robust one's \
             with one row per cell); a variance below 0 is kept and its standard error is \
             missing; Stata's header and footer, and no e(sample), as the add-on leaves.",
        ),
        p(
            "Differences",
            "A missing value in a cluster variable stops the R script: the add-on then fits \
             its parts on different rows, which RCraft does not reproduce. Collinear \
             variables, factor variables and predict after it are not translated yet; the \
             coefficient table is in R's layout.",
        ),
    ],
}
