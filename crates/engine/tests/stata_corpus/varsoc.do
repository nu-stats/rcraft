* RCraft's Stata corpus (P2C.12, batch G): made-up data; Stata 19's results are in varsoc.recorded
* commands: varsoc
* (lag-order selection for one to three variables: the default and given maxlag, exog(), if and in, a gap in time, a time with dates, r() and r(stats), and the errors with their return codes)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 140
generate int t = _n
tsset t
generate double x = sin(_n * 12.9898) * 3.7 + cos(_n * 78.233) * 2.1 + sin(_n * _n / 91)
generate double w = cos(_n * 7.1) + sin(_n * 3.3 + 1) * 1.3 + mod(_n, 7) / 5
generate double z = sin(_n * 1.7) * 2 + cos(_n * _n / 37)
generate double y = x in 1
replace y = 0.5 * L.y + 0.3 * L.w + sin(_n * 0.7 + x) in 2/l
varsoc y w, maxlag(3)
display "@ v1 " r(N) " " r(mlag) " " r(tmin) " " r(tmax) " " r(N_gaps) " [" r(endog) "] [" r(exog) "]"
matrix S = r(stats)
display "@ v1s " rowsof(S) " " colsof(S) " " S[1, 1] " " %24.17g S[1, 2] " " (S[1, 3] >= .) " " (S[1, 4] >= .) " " (S[1, 5] >= .) " " %24.17g S[1, 6] " " %24.17g S[1, 7] " " %24.17g S[1, 8] " " %24.17g S[1, 9]
display "@ v1t " %24.17g S[2, 2] " " %24.17g S[2, 3] " " S[2, 4] " " %24.17g S[2, 5] " " %24.17g S[2, 6] " " %24.17g S[2, 7] " " %24.17g S[2, 8] " " %24.17g S[2, 9]
display "@ v1u " %24.17g S[4, 2] " " %24.17g S[4, 3] " " %24.17g S[4, 5] " " %24.17g S[4, 6] " " %24.17g S[4, 7] " " %24.17g S[4, 8] " " %24.17g S[4, 9]
varsoc y w
display "@ v2 " r(N) " " r(mlag) " " r(tmin) " " r(tmax)
matrix S = r(stats)
display "@ v2s " rowsof(S) " " %24.17g S[3, 2] " " %24.17g S[5, 7] " " %24.17g S[5, 9] " " %24.17g S[2, 6]
varsoc y w z, maxlag(2)
display "@ v3 " r(N) " " r(mlag) " [" r(endog) "]"
matrix S = r(stats)
display "@ v3s " %24.17g S[3, 2] " " S[3, 4] " " %24.17g S[3, 5] " " %24.17g S[3, 6] " " %24.17g S[3, 7] " " %24.17g S[3, 8] " " %24.17g S[3, 9]
varsoc y, maxlag(5)
display "@ v4 " r(N) " " r(mlag)
matrix S = r(stats)
display "@ v4s " %24.17g S[6, 2] " " S[6, 4] " " %24.17g S[6, 5] " " %24.17g S[4, 6] " " %24.17g S[4, 7] " " %24.17g S[2, 8] " " %24.17g S[3, 9]
varsoc y w, maxlag(3) exog(z)
display "@ v5 " r(N) " [" r(exog) "]"
matrix S = r(stats)
display "@ v5s " %24.17g S[1, 2] " " %24.17g S[2, 3] " " %24.17g S[4, 6] " " %24.17g S[3, 7]
varsoc y w if t > 20, maxlag(3)
display "@ v7 " r(N) " " r(tmin) " " r(tmax)
matrix S = r(stats)
display "@ v7s " %24.17g S[2, 2] " " %24.17g S[4, 9]
varsoc y w in 10/90, maxlag(2)
display "@ v8 " r(N) " " r(tmin) " " r(tmax)
matrix S = r(stats)
display "@ v8s " %24.17g S[3, 2] " " %24.17g S[3, 8]
varsoc x w z y, maxlag(2) exog(t)
display "@ v9 " r(N) " [" r(endog) "] [" r(exog) "]"
matrix S = r(stats)
display "@ v9s " %24.17g S[3, 2] " " %24.17g S[3, 6]
preserve
format t %td
tsset t
varsoc y w, maxlag(2)
display "@ d1 " r(N) " " r(tmin) " " r(tmax)
restore
preserve
drop if t == 60 | t == 61
tsset t
varsoc y w, maxlag(2)
display "@ g1 " r(N) " " r(tmin) " " r(tmax) " " r(N_gaps)
matrix S = r(stats)
display "@ g1s " %24.17g S[3, 2] " " %24.17g S[3, 7]
restore
preserve
replace y = . in 30
varsoc y w, maxlag(2)
display "@ g2 " r(N) " " r(N_gaps)
matrix S = r(stats)
display "@ g2s " %24.17g S[3, 2] " " %24.17g S[3, 7]
restore
capture varsoc
display "@ e1 " _rc
capture varsoc nosuch
display "@ e2 " _rc
capture varsoc y w, nosuchopt
display "@ e3 " _rc
capture varsoc y w, maxlag(2.5)
display "@ e4 " _rc
capture varsoc y w, exog(nosuch)
display "@ e5 " _rc
capture varsoc y w, exog(x)
display "@ e6 " _rc
generate str3 s = "abc"
capture varsoc y s
display "@ e7 " _rc
capture varsoc y w, maxlag(0)
display "@ e8 " _rc
capture varsoc y w [fweight = t]
display "@ e9 " _rc
tsset, clear
capture varsoc y w
display "@ e10 " _rc
