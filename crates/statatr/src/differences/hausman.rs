Guide {
    command: "hausman",
    about: "Hausman specification test (hausman fe re, sigmamore)",
    points: &[
        p(
            "In R",
            "stata_hausman compares the coefficients two stored results (or . ) have, the \
             constant only with constant, with V_b - V_B inverted by Gauss-Jordan and its rank \
             as df; a negative chi2 prints Stata's warning with p = 1; sigmamore and sigmaless \
             rescale by the models' root MSEs. r(rank), r(chi2), r(df), r(p).",
        ),
        p(
            "Not yet",
            "alleqs, equations(), df(), tconsistent, rhs; results after xtregar and the \
             models other than regress, xtreg, ivregress, logit, probit and poisson.",
        ),
    ],
}
