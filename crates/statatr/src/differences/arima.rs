Guide {
    command: "arima",
    about: "Regression with ARMA disturbances, and predict after it",
    points: &[
        p(
            "As Stata 19",
            "ar(), ma(), arima(p,d,q), mar(), mma() and sarima(P,0,Q,s) with regressors: the exact \
             likelihood by the Kalman filter, Stata's starting values (least squares; AR terms \
             by least squares of the residuals; MA terms by Hannan and Rissanen's two \
             regressions), its BHHH-then-BFGS path and stopping rule (tolerance(), \
             ltolerance(), nrtolerance(), iterate()), OPG standard errors, e() and _b[ARMA:L.ar] \
             as Stata names them. predict has xb, y, residuals, yresiduals, mse, structural and \
             dynamic(t) (StatCraft's decoding of Stata 19).",
        ),
        p(
            "Differences",
            "The AR paths and tight-tolerance fits agree with Stata to 1e-6 or better; the \
             default stop of MA(1) and ARMA(1,1) models differs by about 1e-4 (Stata's line \
             search is not decoded), and ar(1 2) ma(1), ma(2) and seasonal terms reach the same \
             optimum by a path of their own. Time-series operators, weights, constraints(), \
             vce(), seasonal differencing, gaps in the sample, predict with stdp or t0(), and \
             estat and the rest of the post-estimation commands are not translated yet.",
        ),
    ],
}
