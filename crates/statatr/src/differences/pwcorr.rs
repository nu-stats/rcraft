Guide {
    command: "pwcorr",
    about: "Pairwise correlations",
    points: &[
        p(
            "Rows",
            "Each correlation has the rows where both of its variables are present; with \
             listwise a row with any missing value is dropped.",
        ),
        p(
            "Significance",
            "The p-value from t on n - 2 degrees of freedom, and Bonferroni's and Sidak's \
             adjustments over the number of pairs, as Stata's.",
        ),
        p(
            "Layout",
            "Stata 19's table: the names cut to 8 over the columns and 12 beside the rows, \
             star() marks, print() blanks, sig's p-values and obs's counts below each row \
             with a blank row after, seven columns to a block. print() with obs is refused \
             (not observed).",
        ),
    ],
}
