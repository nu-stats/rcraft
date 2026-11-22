Guide {
    command: "signrank",
    about: "Wilcoxon matched-pairs signed-rank test (signrank x = w, signrank x = 0)",
    points: &[
        p(
            "In R",
            "stata_signrank prints Stata 19's table (positive, negative and zero differences \
             with their observed and expected rank sums), the unadjusted variance with the \
             ties' and the zeros' adjustments, z, Prob > |z| and the exact p (over every sign \
             of the nonzero ranks); r() as Stata's.",
        ),
        p(
            "Not yet",
            "Options, weights, and an expression on the right other than a variable or a \
             number are refused.",
        ),
    ],
}
