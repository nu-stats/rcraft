Guide {
    command: "discrim",
    about: "Linear discriminant analysis with its classification table",
    points: &[
        p(
            "As Stata 19",
            "discrim lda x1 x2 x3, group(g) [priors(equal|proportional) notable]: the classification functions, \
             the resubstitution table, the canonical discriminant functions in e() (L_raw, L_unstd, L_std, \
             L_totalstd, canstruct, cmeans, candisc_stat), predict (the classification) and estat classtable. \
             Checked against Stata 19 to 1e-9.",
        ),
        p(
            "Not yet",
            "qda, knn and logistic, priors(#...), ties(), a group variable with value labels (Stata names the \
             groups by them), predict with pr, and the other estat commands.",
        ),
    ],
}
