Guide {
    command: "pperron",
    about: "Phillips-Perron unit-root test",
    points: &[
        p(
            "As Stata 19",
            "pperron y regresses y on L.y (and _trend, and a constant unless noconstant) and corrects the statistics for the residuals' autocorrelation with Newey-West weights; lags(#) sets the lags (the default is floor(4 (T/100)^(2/9)); a negative number gives the default). Z(rho) and Z(t) have Fuller's critical values; the p-value is MacKinnon's for Z(t) (none with noconstant). r() holds N, lags, Zrho, Zt, pval and the matrix cvalues (the statistic and the 1%, 5% and 10% values of each row).",
        ),
        p(
            "Errors",
            "As dfuller. A sample too small for the regression is refused.",
        ),
    ],
}
