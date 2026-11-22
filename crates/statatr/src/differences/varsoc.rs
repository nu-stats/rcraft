Guide {
    command: "varsoc",
    about: "Lag-order selection statistics for a vector autoregression",
    points: &[
        p(
            "As Stata 19",
            "varsoc y w, maxlag(#) (the default is 4) fits lag orders 0 to # on the sample the longest one leaves and prints each one's log likelihood, the likelihood-ratio test against the order before (a star on the last significant at 5%), FPE, AIC, HQIC and SBIC (a star on each one's smallest). exog() adds variables to every equation. r() holds N, mlag, tmin, tmax, N_gaps, the texts endog and exog and the matrix stats (lag, LL, LR, df, p, FPE, AIC, HQIC, SBIC; lag 0 has missing LR, df and p).",
        ),
        p(
            "Errors",
            "No variable, an unknown option and maxlag() that is not a positive whole number r(198), a name the data lack r(111), a string variable r(109), weights r(101), without tsset r(111). Refused: noconstant, varsoc after xtset, a name list that is not plain variables, and after a var (the lags then come from the model).",
        ),
    ],
}
