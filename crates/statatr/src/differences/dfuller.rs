Guide {
    command: "dfuller",
    about: "Augmented Dickey-Fuller unit-root test",
    points: &[
        p(
            "As Stata 19",
            "dfuller y regresses D.y on L.y, lags(#) lagged differences, _trend with trend and a constant unless noconstant, over the rows if and in keep; a gap in time or a missing value only drops the rows that need it. The statistic is L.y's t, with Fuller's critical values (linear in the sample size) and MacKinnon's approximate p-value; drift uses the t distribution of the regression. regress shows the regression table. A time-series operator may stand for the variable. r() holds lags, N, Zt, p (not with noconstant) and cv_1, cv_5, cv_10. A negative lags() counts as 0.",
        ),
        p(
            "Errors",
            "Without tsset r(111), a second variable r(103), no variable r(100), trend with noconstant, drift with trend or noconstant and an unknown option r(198), lags() that is not a whole number r(198), several panels in the sample r(459), a string variable r(109). A sample too small for the regression is refused.",
        ),
    ],
}
