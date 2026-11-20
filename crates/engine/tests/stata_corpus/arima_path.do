* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in arima_path.recorded
* commands: arima
* (arima's default path: the starting values (iterate(0)), each step (iterate(k)) and the default stop, with and without MA terms, a regressor, arima(2,1,0), noconstant and loose tolerances; the path is StatCraft's decoding of Stata's, whose gradient for sigma differs a little from the exact one, so the AR paths agree to 1e-6 and the MA(1) and ARMA(1,1) default stops, where Stata's line search runs past the full step by a rule not decoded, to 2e-4)
* tolerance: 2e-4
version 19
clear
set linesize 255
set obs 150
generate t = _n
tsset t
generate double eps = (mod(_n*37 + 11, 89) - 44)/22
generate double x = (mod(_n*53 + 7, 71) - 35)/30
generate double y = 0 in 1
replace y = 0.3 + 0.6*y[_n-1] + 0.5*x + eps + 0.4*eps[_n-1] if _n > 1
quietly arima y, ar(1) iterate(0)
display "@ s0 " %24.17g e(ll) " " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[sigma:_cons]
forvalues k = 1/4 {
    quietly arima y, ar(1) iterate(`k')
    display "@ s`k' " %24.17g e(ll) " " e(ic) " " e(converged) " " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[sigma:_cons]
}
quietly arima y, ar(1)
display "@ ar1 " %24.17g e(ll) " " e(ic) " " e(converged) " " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[sigma:_cons] " " %24.17g _se[ARMA:L.ar]
quietly arima y x, ar(1)
display "@ x1 " %24.17g e(ll) " " e(ic) " " e(converged) " " %24.17g _b[y:x] " " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[sigma:_cons]
quietly arima y x, ar(1) iterate(0)
display "@ x1s " %24.17g e(ll) " " %24.17g _b[y:x] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[sigma:_cons]
quietly arima y, arima(2,1,0)
display "@ d210 " %24.17g e(ll) " " e(ic) " " e(converged) " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA:L2.ar] " " %24.17g _b[sigma:_cons]
quietly arima y x
display "@ reg " %24.17g e(ll) " " e(ic) " " e(converged) " " %24.17g _b[y:x] " " %24.17g _b[y:_cons] " " %24.17g _b[sigma:_cons]
quietly arima y, ar(1) noconstant
display "@ nocons " %24.17g e(ll) " " e(ic) " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[sigma:_cons]
quietly arima y x, ar(1) ltolerance(1e-3) tolerance(1e-3)
display "@ loose " e(ic) " " e(converged)
quietly arima y, ar(1 2 3)
display "@ ar123 " %24.17g e(ll) " " e(ic) " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA:L2.ar] " " %24.17g _b[ARMA:L3.ar]
quietly arima y, ma(1) iterate(0)
display "@ ma0 " %24.17g e(ll) " " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L.ma] " " %24.17g _b[sigma:_cons]
quietly arima y, ma(1)
display "@ ma1 " %24.17g e(ll) " " e(converged) " " %24.17g _b[y:_cons] " " %24.17g _b[ARMA:L.ma] " " %24.17g _b[sigma:_cons]
quietly arima y x, ar(1) ma(1) iterate(0)
display "@ arma0 " %24.17g e(ll) " " %24.17g _b[y:x] " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA:L.ma] " " %24.17g _b[sigma:_cons]
quietly arima y x, ar(1) ma(1)
display "@ arma1 " %24.17g e(ll) " " e(converged) " " %24.17g _b[ARMA:L.ar] " " %24.17g _b[ARMA:L.ma] " " %24.17g _b[sigma:_cons]
quietly arima y, ma(1 2) iterate(0)
display "@ ma12 " %24.17g _b[ARMA:L.ma] " " %24.17g _b[ARMA:L2.ma] " " %24.17g _b[sigma:_cons]
quietly arima y, ar(1) iterate(0) noconstant
display "@ nc0 " %24.17g _b[ARMA:L.ar] " " %24.17g _b[sigma:_cons]
