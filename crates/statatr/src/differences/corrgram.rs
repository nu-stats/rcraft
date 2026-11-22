Guide {
    command: "corrgram",
    about: "Autocorrelations, partial autocorrelations and Q statistics",
    points: &[
        p(
            "As Stata 19",
            "corrgram y, lags(#) prints the autocorrelation (sum of products about the mean over the sum of squares, over the pairs of times both present), the partial autocorrelation (the last coefficient of y on its first k lags, or Yule-Walker with yw), Ljung and Box's Q and its p-value, and the bars (not with noplot). The default is min(floor(n/2) - 2, 40) lags. A gap in time or a missing value is noted. r() holds lags, the scalars ac#, pac#, q# and p# for the first ten lags and the matrices AC, PAC, Q and P with all of them.",
        ),
        p(
            "Errors",
            "Without tsset r(111), a second variable r(103), no variable r(100), lags() of 0 or as long as the sample r(498), a string variable r(109), several panels r(459), an unknown option r(198).",
        ),
    ],
}
