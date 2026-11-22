Guide {
    command: "anova",
    about: "Analysis of variance",
    points: &[
        p(
            "Variables",
            "A variable is categorical unless written c.x, as in Stata's anova; in R's \
             formula a number is continuous unless written factor().",
        ),
        p(
            "Sums of squares",
            "Partial (Type III) sums of squares by default, as Stata's; R's anova() gives \
             sequential (Type I) ones, which the translation uses only with sequential. A \
             design with an empty cell stops, since Stata's rule there is not matched yet.",
        ),
        MISSING_IN_MODELS,
    ],
}
