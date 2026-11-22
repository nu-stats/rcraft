Guide {
    command: "dfgls",
    about: "DF-GLS unit-root test",
    points: &[
        p(
            "As Stata 19",
            "dfgls y detrends the series by GLS (a trend, or only a mean with notrend) and tests the detrended series for each lag from maxlag(#) (the default is Schwert's floor(12 (T/100)^(1/4))) down to 1. The 1% value is Elliott, Rothenberg and Stock's, the 5% and 10% values Cheung and Lai's response surface (all three Elliott-Rothenberg-Stock's with ers). It prints the lag Ng and Perron's sequential test picks and the minimum SIC and MAIC. r() holds maxlag, N, the chosen lags and criteria, and the matrices cvalues and results; with maxlag(0) only N, cv_1, cv_5, cv_10, rmse0 and dft0, with maxlag(1) only maxlag, N and the matrices.",
        ),
        p(
            "Errors",
            "Without tsset r(111), a gap in the sample r(498), maxlag() that is not a whole number r(126), more lags than the sample holds r(2000) or r(2001), several panels r(459), an unknown option r(198). A sample too small for the lags is refused.",
        ),
    ],
}
