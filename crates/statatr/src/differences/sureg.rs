Guide {
    command: "sureg",
    about: "Seemingly unrelated regression",
    points: &[
        p(
            "As Stata 19",
            "sureg (y1 x1 x2) (y2 x1 x3) is two-step feasible GLS: Sigma from each equation's \
             least-squares residuals (cross products over N, or over sqrt((Ni - ki)(Nj - kj)) \
             with dfk), b = (X'(Sigma^-1 kron I)X)^-1 X'(Sigma^-1 kron I)y, V the inverse. isure \
             (with nolog) goes round again with the residuals of the last fit until no \
             coefficient moves by 1e-6; small gives F and t on N*M - k degrees of freedom; dfk \
             prints Stata's note. Per equation: R-squared (centred), RMSE, the Wald chi2 of \
             its slopes; e(ll), e(Sigma), e(b), e(V) and Stata's e() names. corr, noheader, \
             notable, level(), fweights and aweights, predict (xb, residuals, equation()) and \
             test across equations are taken; noconstant, pweights and iweights stop as Stata \
             stops (r(198), r(101)).",
        ),
        p(
            "Not yet",
            "isure without nolog (Stata's iteration log), equation names (name: y x), the \
             y1 y2 = x form, constraints(), factor variables, one equation, the same dependent \
             variable twice, and predict's other statistics. e(dfk2_adj) is N (Stata's has \
             rounding noise).",
        ),
    ],
}
