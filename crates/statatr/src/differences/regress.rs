Guide {
    command: "regress",
    about: "Linear regression",
    points: &[
        p(
            "Collinear regressors",
            "The smallest of a collinear set is omitted, whatever the order written, as Stata \
             does (it takes the column with the most variation left at each step), with \
             Stata's note and a 0 (omitted) row. R's lm() would drop the later one.",
        ),
        p(
            "Interactions",
            "g#h is i.g#i.h, as in Stata: in an interaction a bare variable is a factor. \
             i.g#i.h without its main effects estimates every cell but the one where both \
             are at their base, as Stata does (R alone would leave out another, giving \
             other coefficients); i.g##i.h is R's own coding. The cells keep Stata's names \
             in e(b) (1b.g#1.h, 2o.g#0b.h) and its table, value labels and empty cells \
             included.",
        ),
        p(
            "Robust errors",
            "robust is HC1, as Stata's: the sandwich times N / (N - k). R's sandwich \
             package defaults to HC3, which differs.",
        ),
        p(
            "Clustered errors",
            "Scaled by G / (G - 1) and (N - 1) / (N - k), with t tests and intervals on G - 1 \
             degrees of freedom, as Stata's; a row dropped for a missing value is out of its \
             cluster too.",
        ),
        p(
            "Weights",
            "Frequency weights count rows; analytic ones are scaled to sum to N; sampling \
             weights mean robust errors; with importance weights N is their sum rounded \
             down, as Stata's e(N).",
        ),
        p(
            "Factor variables",
            "i.g leaves out the lowest level, as Stata's base; without the constant Stata \
             still leaves it out while R would keep every level, so noconstant with factor \
             variables is refused.",
        ),
        p(
            "Printing",
            "Stata's own table (ANOVA or robust header, coefficients with t, P and the \
             interval), checked character for character; a shape not checked keeps R's \
             layout and says so.",
        ),
        MISSING_IN_MODELS,
    ],
}
