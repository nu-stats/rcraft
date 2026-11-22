Guide {
    command: "ktau",
    about: "Kendall's rank correlations (ktau x y, ktau x y z, stats(taub p))",
    points: &[
        p(
            "In R",
            "stata_ktau prints Stata 19's two-variable report (tau-a, tau-b, the score, its \
             standard error corrected for ties and the continuity-corrected p) or, for three \
             or more (or with matrix), the lower triangle of tau-a or of the stats() asked \
             for, with its Key, print() and star(), listwise or pw, Bonferroni's or Sidak's \
             adjustment; r() as Stata's, the matrices included. As Stata 19 counts them, the \
             score takes values that agree to 15 significant digits as tied, the ties' \
             corrections only values exactly equal.",
        ),
        p(
            "Digits",
            "p-values near 1e-14 differ in their second digit (Stata's normal tail is \
             less precise there); what is printed is the same.",
        ),
        p("Not yet", "Weights are refused."),
    ],
}
