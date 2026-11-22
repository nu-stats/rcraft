Guide {
    command: "wntestq",
    about: "Portmanteau (Q) test for white noise",
    points: &[
        p(
            "As Stata 19",
            "wntestq y, lags(#) prints Ljung and Box's Q at the last lag with its chi-squared p-value (the default lags are those of corrgram). r() holds df, p and stat.",
        ),
        p(
            "Errors",
            "As corrgram, except that a string variable gives r(2000).",
        ),
    ],
}
