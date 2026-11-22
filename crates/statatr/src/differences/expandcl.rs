Guide {
    command: "expandcl",
    about: "Copies whole clusters of observations and numbers the copies",
    points: &[
        p(
            "As Stata 19",
            "expandcl exp, cluster(varlist) generate(newvar) makes exp copies of each selected \
             observation (a missing, zero or negative count keeps the one, a fraction is rounded) \
             and numbers the (cluster, copy) pairs in newvar: clusters in the sort order of \
             their variables, missing last, a cluster with k copies taking k numbers in a row. \
             It says how many counts were missing, noninteger, negative or zero, then (N \
             observations created). The expression must be the same within a cluster (r(198)); \
             both options are required (r(198)); a string count is r(109).",
        ),
        p(
            "In R",
            "Stata keeps the copies of one observation next to it in an order of its own, not \
             stable; RCraft puts them in the order of their numbers, so code that depends on \
             the order of the copies, rather than sorting first, can differ. by: stops with Stata's r(190).",
        ),
    ],
}
