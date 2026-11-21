* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in arima.recorded
* commands: arima predict
* (arima at tight tolerances (3e-5: the OPG variance and a near-zero d1 value differ from Stata's in the 6th digit), where Stata stops at the optimum: AR, MA, ARMA, a regressor, arima(1,1,0), noconstant, level(); predict xb, y, residuals, yresiduals, mse, structural, dynamic(); e() scalars and _b, _se)
* tolerance: 3e-5
version 19
clear
set linesize 255
set obs 120
generate t = _n
tsset t
generate double eps = (mod(_n*37 + 11, 89) - 44)/22
generate double x = (mod(_n*53 + 7, 71) - 35)/30
generate double y = 0 in 1
replace y = 0.5 + 0.6*y[_n-1] + 0.8*x + eps + 0.3*eps[_n-1] if _n > 1
generate double z = y + 0.05*_n
local tight nolog tolerance(1e-10) ltolerance(1e-12) nrtolerance(1e-12)
quietly arima y, ar(1) `tight'
display "@ ar1 " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g e(chi2) " " e(N) " " e(k) " " e(df_m) " " e(ar_max) " " e(ma_max) " " e(converged) " " %24.17g e(p)
display "@ ar1b " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[sigma:_cons] " " %24.17g _se[y:_cons] " " %24.17g _se[ARMA:L.ar] " " %24.17g _se[sigma:_cons]
quietly arima y, ar(1) ma(1) `tight'
display "@ arma " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g e(chi2) " " e(k) " " e(df_m) " " e(ar_max) " " e(ma_max)
display "@ armab " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA:L.ma] " " %24.17g _se[ARMA:L.ar] " " %24.17g _se[ARMA:L.ma]
quietly arima y x, ar(1) `tight'
display "@ reg " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g e(chi2) " " e(k) " " e(df_m) " " %24.17g _b[y:x] " " %24.17g _se[y:x]
quietly arima z, arima(1,1,0) `tight'
display "@ diff " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g e(chi2) " " e(N) " " %24.17g _b[z:_cons] " " %24.17g _b[ARMA:L.ar]
quietly arima y, ma(1 2) `tight'
display "@ ma12 " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g _b[ARMA:L.ma] " " %24.17g _b[ARMA:L2.ma] " " %24.17g _se[ARMA:L2.ma]
quietly arima y, ar(1 2) noconstant `tight'
display "@ ar12 " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA:L2.ar] " " e(k) " " e(df_m)
quietly arima y x, ar(1) ma(1) `tight' level(90)
display "@ both " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g _b[y:x] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA:L.ma] " " %24.17g _se[y:x]
predict double p1
predict double p1y, y
predict double p1r, residuals
predict double p1yr, yresiduals
predict double p1m, mse
predict double p1s, xb structural
predict double p1d, xb dynamic(70)
quietly summarize p1
display "@ p1 " r(N) " " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g p1[1] " " %24.17g p1[2] " " %24.17g p1[60]
quietly summarize p1r
display "@ p1r " r(N) " " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g p1r[1] " " %24.17g p1r[60]
quietly summarize p1m
display "@ p1m " %24.17g p1m[1] " " %24.17g p1m[2] " " %24.17g p1m[120]
quietly summarize p1s
display "@ p1s " %24.17g r(mean) " " %24.17g p1s[1] " " %24.17g p1s[2]
display "@ p1d " %24.17g p1d[70] " " %24.17g p1d[71] " " %24.17g p1d[120]
display "@ p1y " %24.17g p1y[1] " " %24.17g p1y[5]
display "@ p1yr " %24.17g p1yr[1] " " %24.17g p1yr[5]
quietly arima z, arima(1,1,0) `tight'
predict double d1
predict double d1y, y
predict double d1r, residuals
predict double d1yr, yresiduals
quietly summarize d1
display "@ d1 " r(N) " " %24.17g r(mean) " " %24.17g d1[1] " " %24.17g d1[2] " " %24.17g d1[50]
display "@ d1y " r(N) " " %24.17g d1y[1] " " %24.17g d1y[2] " " %24.17g d1y[50]
display "@ d1r " %24.17g d1r[1] " " %24.17g d1r[2] " " %24.17g d1r[50] " " %24.17g d1yr[2] " " %24.17g d1yr[50]
