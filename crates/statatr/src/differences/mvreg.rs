Guide {
    command: "mvreg",
    about: "Multivariate regression",
    points: &[
        p(
            "As Stata 19",
            "mvreg y1 y2 = x1 x2 fits each equation by least squares on the same rows; e(Sigma) \
             = E'E/(N - k), V = Sigma kron (X'X)^-1, t tests on N - k degrees of freedom, and \
             e(F), e(rmse), e(p_F), e(r2) as space-led lists, one per equation. The equation \
             table, the coefficient table, corr (the correlation matrix of the residuals and \
             the Breusch-Pagan test), noheader, notable, noconstant, level(), fweights and \
             aweights (scaled to sum to N) are Stata's; pweights and iweights stop with \
             `not allowed`, r(101). predict (xb, residuals, equation(name or #k)) and test \
             across equations ([y1]x1 = [y2]x1) work after it.",
        ),
        p(
            "Not yet",
            "Factor variables and operators, a single dependent variable, constraints, and \
             predict's other statistics.",
        ),
    ],
}
