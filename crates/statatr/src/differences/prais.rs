Guide {
    command: "prais",
    about: "Prais-Winsten and Cochrane-Orcutt regression",
    points: &[p(
        "As Stata 19",
        "Least squares first, then rho from the residuals on the original data (or 1 - DW/2 \
         with rhotype(dw)) and a fit on the transformed data, until rho moves by less than \
         1e-6 (one round with twostep); corc drops the first observation; vce(robust) is \
         HC1 on the transformed data. Plain variable names and data without gaps only.",
    )],
}
