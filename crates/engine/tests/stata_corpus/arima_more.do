* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in arima_more.recorded
* commands: arima
* (arima with seasonal terms, mar(), mma() and sarima(), at tight tolerances; several AR lags with MA terms and an MA lag with a gap, at tight and at default tolerances, where the optimum agrees)
* tolerance: 1e-5
version 19
clear
set linesize 255
set obs 150
generate t = _n
tsset t
generate double eps = (mod(_n^3*7 + _n^2*13 + _n*37, 97) - 48)/24
generate double x = (mod(_n*53 + 7, 71) - 35)/30
generate double y = 0 in 1
replace y = 0.3 + 0.5*y[_n-1] + 0.5*x + eps + 0.4*eps[_n-1] if _n > 1
replace y = y + 0.2*y[_n-2] if _n > 2
generate double q = eps + 0.3*eps[_n-1] in 1
replace q = eps + 0.3*eps[_n-1] if _n > 1
replace q = q + 0.6*eps[_n-12] if _n > 12
generate double s = y + 0.8*L12.y if t > 12
replace s = y in 1/12
local tight nolog tolerance(1e-10) ltolerance(1e-12) nrtolerance(1e-12)
quietly arima s, ar(1) mar(1, 12) `tight'
display "@ mar " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g _b[s:_cons] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA12:L.ar] " " %24.17g _se[ARMA12:L.ar]
quietly arima q, arima(1,0,0) sarima(0,0,1,12) `tight'
display "@ sar " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g _b[q:_cons] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA12:L.ma] " " %24.17g _se[ARMA12:L.ma]
quietly arima y, ar(1 2) ma(1) `tight'
display "@ ar2ma " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA:L2.ar] " " %24.17g _b[ARMA:L.ma]
quietly arima y, ma(2) `tight'
display "@ ma2 " %24.17g e(ll) " " %24.17g e(sigma) " " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L2.ma]
quietly arima y, ar(1 2) ma(1)
display "@ ar2madef " %24.17g e(ll) " " e(converged) " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA:L2.ar] " " %24.17g _b[ARMA:L.ma]
quietly arima y, ma(2)
display "@ ma2def " %24.17g e(ll) " " e(converged) " " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L2.ma]
