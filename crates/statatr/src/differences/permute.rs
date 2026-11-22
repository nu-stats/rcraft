Guide {
    command: "permute",
    about: "Repeats a command on random permutations of one variable",
    points: &[
        p(
            "Draws",
            "The permutations are Stata's own, drawn from the generator after seed(): the \
             command runs once on the data as they are, then once for each replication, \
             with the variable permuted among the estimation sample's rows (within each \
             stratum with strata()). A variable with two values is permuted the way Stata \
             does it, with fewer draws. The data are put back afterwards.",
        ),
        p(
            "Results",
            "The table, r() with its matrices (b, n, c_lower, p_twosided, ci_p_*, ...) and \
             saving(file[, replace double]); e() is empty afterwards. A program the do-file \
             defines runs once for each replication, so the R holds its lines that many times.",
        ),
        p(
            "Not yet",
            "spearman, r(nosuch), an e-class program, more than 2,000 replications, left, \
             right and eps() are refused with the reason.",
        ),
    ],
}
