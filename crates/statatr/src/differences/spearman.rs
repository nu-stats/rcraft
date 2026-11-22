Guide {
    command: "spearman",
    about: "Spearman's rank correlations (spearman x w z)",
    points: &[
        p(
            "In R",
            "With three or more variables, the matrix of rank correlations as Stata 19 \
             prints it: the lower triangle, over the rows with none of them missing.",
        ),
        p(
            "Not yet",
            "Two variables are refused: Stata 19's p-value comes from its own approximation \
             to the permutation distribution, not R's, and is not matched yet. The options \
             (stats(), pw, star()), and r() after the matrix, too.",
        ),
    ],
}
