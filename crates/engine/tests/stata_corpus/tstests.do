* RCraft's Stata corpus (P2C.12, batch G): made-up data; Stata 19's results are in tstests.recorded
* commands: dfuller pperron dfgls corrgram wntestq
* (dfuller: constant, noconstant, trend and drift, lags, if and in, a time-series operator as the variable, a gap in time, missing values, one panel of a panel, r() and the errors with their return codes; pperron: the same, its Newey-West lags and cvalues; dfgls: notrend, ers, maxlag, the Schwert default, results and cvalues; corrgram and wntestq: lags, gaps, noplot, yw, if and in, r())
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 200
generate int t = _n
tsset t
generate double e = sin(_n * 12.9898) * 3.7 + cos(_n * 78.233) * 2.1 + sin(_n * _n / 91)
generate double w = sum(e)
generate double y = e in 1
replace y = 0.6 * L.y + e in 2/l
generate double x = cos(_n / 7) + mod(_n, 5) / 4 + sin(_n * _n / 17)
* dfuller
dfuller y
display "@ d1 " r(N) " " r(lags) " " %24.17g r(Zt) " " %24.17g r(p) " " r(cv_1) " " r(cv_5) " " r(cv_10)
dfuller w, trend
display "@ d2 " r(N) " " r(lags) " " %24.17g r(Zt) " " %24.17g r(p) " " r(cv_1) " " r(cv_5) " " r(cv_10)
dfuller w, lags(2) regress
display "@ d3 " r(N) " " r(lags) " " %24.17g r(Zt) " " %24.17g r(p) " " r(cv_1) " " r(cv_5) " " r(cv_10)
dfuller y, noconstant
display "@ d4 " r(N) " " r(lags) " " %24.17g r(Zt) " " (r(p) == .) " " r(cv_1) " " r(cv_5) " " r(cv_10)
dfuller y, drift lags(1)
display "@ d5 " r(N) " " r(lags) " " %24.17g r(Zt) " " %24.17g r(p) " " %24.17g r(cv_1) " " %24.17g r(cv_5) " " %24.17g r(cv_10)
dfuller y, lags(3) trend
display "@ d6 " r(N) " " r(lags) " " %24.17g r(Zt) " " %24.17g r(p) " " r(cv_1) " " r(cv_5) " " r(cv_10)
dfuller y if t > 20
display "@ d7 " r(N) " " %24.17g r(Zt) " " %24.17g r(p) " " r(cv_1)
dfuller y in 10/150, lags(2)
display "@ d8 " r(N) " " %24.17g r(Zt) " " %24.17g r(p) " " r(cv_5)
dfuller D.w
display "@ d9 " r(N) " " %24.17g r(Zt) " " %24.17g r(p)
dfuller w, lags(4) trend regress
display "@ d10 " r(N) " " %24.17g r(Zt) " " %24.17g r(p) " " r(cv_10)
dfuller x, noconstant lags(1)
display "@ d11 " r(N) " " %24.17g r(Zt) " " r(cv_1)
dfuller w, drift
display "@ d12 " r(N) " " %24.17g r(Zt) " " %24.17g r(p) " " %24.17g r(cv_1)
dfuller y, lags(-2)
display "@ d13 " r(N) " " r(lags) " " %24.17g r(Zt)
dfuller w if t <= 40, trend lags(1)
display "@ d14 " r(N) " " %24.17g r(Zt) " " %24.17g r(p) " " r(cv_1) " " r(cv_5)
dfuller x if t <= 28, lags(1)
display "@ d15 " r(N) " " %24.17g r(Zt) " " %24.17g r(p) " " r(cv_1)
* pperron
pperron y
display "@ p1 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt) " " %24.17g r(pval)
matrix C = r(cvalues)
display "@ p1c " rowsof(C) " " colsof(C) " " C[1, 1] " " C[1, 2] " " C[1, 3] " " (C[1, 4] == .) " " C[2, 1] " " C[2, 2] " " C[2, 3] " " %24.17g C[2, 4]
pperron w, trend lags(3)
display "@ p2 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt) " " %24.17g r(pval)
matrix C = r(cvalues)
display "@ p2c " C[1, 1] " " C[1, 2] " " C[1, 3] " " C[2, 1] " " C[2, 2] " " C[2, 3] " " %24.17g C[2, 4]
pperron y, noconstant
display "@ p3 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt) " " (r(pval) == .)
matrix C = r(cvalues)
display "@ p3c " C[1, 1] " " C[2, 1] " " (C[2, 4] == .)
pperron w, regress lags(0)
display "@ p4 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt) " " %24.17g r(pval)
pperron y if t > 30, lags(6)
display "@ p5 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt)
pperron w in 5/90, trend
display "@ p6 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt)
pperron D.y
display "@ p7 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt)
pperron x if t <= 29, lags(5)
display "@ p8 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt)
pperron y, lags(-1)
display "@ p9 " r(N) " " r(lags)
* dfgls
dfgls w if t <= 150, maxlag(4)
display "@ g1 " r(maxlag) " " r(N) " " r(siclag) " " r(maiclag) " " %24.17g r(maicrmse) " " %24.17g r(maicmin) " " %24.17g r(sicrmse) " " %24.17g r(sicmin) " " %24.17g r(optrmse) " " r(optlag)
matrix R = r(results)
matrix C = r(cvalues)
display "@ g1r " rowsof(R) " " colsof(R) " " R[1, 1] " " %24.17g R[1, 2] " " %24.17g R[1, 3] " " %24.17g R[1, 4] " " %24.17g R[1, 5] " " %24.17g R[4, 2] " " %24.17g R[4, 5]
display "@ g1c " rowsof(C) " " colsof(C) " " C[1, 1] " " C[1, 2] " " %24.17g C[1, 3] " " %24.17g C[1, 4] " " %24.17g C[3, 3] " " %24.17g C[4, 4]
dfgls w if t <= 150, maxlag(4) notrend
display "@ g2 " r(maxlag) " " r(N) " " r(siclag) " " r(maiclag) " " %24.17g r(sicmin) " " %24.17g r(maicmin) " " %24.17g r(optrmse) " " r(optlag)
matrix C = r(cvalues)
display "@ g2c " C[1, 2] " " %24.17g C[1, 3] " " %24.17g C[1, 4] " " %24.17g C[1, 5] " " %24.17g C[2, 5]
dfgls w if t <= 150
display "@ g3 " r(maxlag) " " r(N) " " r(siclag) " " r(maiclag) " " %24.17g r(sicmin) " " %24.17g r(maicmin) " " r(optlag)
dfgls y if t <= 60, maxlag(2)
display "@ g4 " r(maxlag) " " r(N) " " r(siclag) " " r(maiclag) " " %24.17g r(sicmin) " " %24.17g r(maicmin) " " r(optlag) " " (r(optrmse) == .)
matrix C = r(cvalues)
display "@ g4c " C[1, 2] " " %24.17g C[1, 3] " " %24.17g C[1, 4] " " %24.17g C[1, 5] " " %24.17g C[2, 5]
dfgls y if t <= 60, maxlag(2) notrend
display "@ g5 " r(N) " " r(siclag) " " r(maiclag) " " %24.17g r(sicmin) " " %24.17g r(maicmin) " " r(optlag)
dfgls y if t <= 100, maxlag(1)
display "@ g6 " r(maxlag) " " r(N)
matrix C = r(cvalues)
display "@ g6c " rowsof(C) " " C[1, 1] " " C[1, 2] " " %24.17g C[1, 3] " " %24.17g C[1, 4] " " %24.17g C[1, 5]
dfgls y if t <= 140, maxlag(1)
matrix C = r(cvalues)
display "@ g7c " C[1, 2] " " %24.17g C[1, 3] " " %24.17g C[1, 4] " " %24.17g C[1, 5]
dfgls y if t <= 520, maxlag(1) notrend
matrix C = r(cvalues)
display "@ g8c " C[1, 2] " " %24.17g C[1, 3] " " %24.17g C[1, 4] " " %24.17g C[1, 5]
dfgls y, maxlag(3) ers
display "@ g9 " r(maxlag) " " r(N) " " r(siclag) " " r(maiclag) " " %24.17g r(sicmin) " " %24.17g r(maicmin) " " r(optlag)
matrix C = r(cvalues)
display "@ g9c " C[1, 2] " " C[1, 3] " " C[1, 4] " " %24.17g C[1, 5] " " C[3, 3]
dfgls y, maxlag(0)
display "@ g10 " (r(maxlag) == .) " " r(N) " " r(cv_1) " " %24.17g r(cv_5) " " %24.17g r(cv_10) " " %24.17g r(rmse0) " " %24.17g r(dft0)
dfgls x, maxlag(6) ers notrend
display "@ g11 " r(maxlag) " " r(N) " " r(siclag) " " r(maiclag) " " r(optlag)
matrix C = r(cvalues)
display "@ g11c " C[1, 2] " " C[1, 3] " " C[1, 4] " " %24.17g C[1, 5]
dfgls D.w, maxlag(3)
display "@ g12 " r(maxlag) " " r(N) " " r(siclag) " " r(optlag)
* corrgram and wntestq
corrgram y, lags(8)
display "@ c1 " r(lags) " " %24.17g r(ac1) " " %24.17g r(pac1) " " %24.17g r(q1) " " %24.17g r(p1) " " %24.17g r(ac8) " " %24.17g r(pac8) " " %24.17g r(q8) " " %24.17g r(p8)
matrix A = r(AC)
matrix P = r(PAC)
matrix Q = r(Q)
matrix H = r(P)
display "@ c1m " rowsof(A) " " colsof(A) " " %24.17g A[1, 3] " " %24.17g P[1, 3] " " %24.17g Q[1, 5] " " %24.17g H[1, 7]
corrgram w
display "@ c2 " r(lags) " " %24.17g r(ac1) " " %24.17g r(pac2) " " %24.17g r(q40) " " %24.17g r(ac40)
corrgram y, lags(5) noplot
display "@ c3 " r(lags) " " %24.17g r(ac3) " " %24.17g r(pac5)
corrgram y, lags(5) yw
display "@ c4 " r(lags) " " %24.17g r(ac3) " " %24.17g r(pac1) " " %24.17g r(pac2) " " %24.17g r(pac5)
corrgram y if t > 100, lags(4)
display "@ c5 " r(lags) " " %24.17g r(ac1) " " %24.17g r(pac4) " " %24.17g r(q4)
corrgram y in 10/80, lags(6)
display "@ c6 " r(lags) " " %24.17g r(ac2) " " %24.17g r(pac6) " " %24.17g r(q6)
corrgram D.w, lags(3)
display "@ c7 " r(lags) " " %24.17g r(ac1) " " %24.17g r(pac3)
corrgram x if t <= 30
display "@ c8 " r(lags) " " %24.17g r(ac1) " " %24.17g r(q13) " " %24.17g r(pac13)
wntestq y, lags(10)
display "@ q1 " r(df) " " %24.17g r(stat) " " %24.17g r(p)
wntestq w
display "@ q2 " r(df) " " %24.17g r(stat) " " %24.17g r(p)
wntestq y if t > 100
display "@ q3 " r(df) " " %24.17g r(stat) " " %24.17g r(p)
wntestq y in 3/60, lags(4)
display "@ q4 " r(df) " " %24.17g r(stat) " " %24.17g r(p)
wntestq D.y, lags(3)
display "@ q5 " r(df) " " %24.17g r(stat) " " %24.17g r(p)
* a gap in time and missing values
preserve
drop if t == 50 | t == 51 | t == 120
tsset t
dfuller y
display "@ k1 " r(N) " " %24.17g r(Zt) " " %24.17g r(p)
dfuller y, lags(2) trend
display "@ k2 " r(N) " " %24.17g r(Zt) " " %24.17g r(p)
pperron y
display "@ k3 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt)
pperron w, trend
display "@ k4 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt)
corrgram y, lags(5)
display "@ k5 " r(lags) " " %24.17g r(ac1) " " %24.17g r(ac5) " " %24.17g r(pac3) " " %24.17g r(q5)
wntestq y
display "@ k6 " r(df) " " %24.17g r(stat) " " %24.17g r(p)
capture dfgls y, maxlag(2)
display "@ k7 " _rc
capture dfgls y if t > 121, maxlag(2)
display "@ k8 " _rc " " r(N)
corrgram y if t > 121, lags(3)
display "@ k9 " r(lags) " " %24.17g r(ac1) " " %24.17g r(pac3)
restore
preserve
replace y = . in 20/22
replace w = . in 80
dfuller y
display "@ m1 " r(N) " " %24.17g r(Zt) " " %24.17g r(p)
dfuller w, lags(1)
display "@ m2 " r(N) " " %24.17g r(Zt) " " %24.17g r(p)
pperron y
display "@ m3 " r(N) " " r(lags) " " %24.17g r(Zrho) " " %24.17g r(Zt)
corrgram y, lags(4)
display "@ m4 " r(lags) " " %24.17g r(ac1) " " %24.17g r(ac4) " " %24.17g r(pac2) " " %24.17g r(q4)
wntestq y, lags(5)
display "@ m5 " r(df) " " %24.17g r(stat) " " %24.17g r(p)
capture dfgls y
display "@ m6 " _rc
dfgls y if t > 30, maxlag(2)
display "@ m7 " r(N) " " r(siclag)
replace y = . in 1/3
replace y = . in 198/l
corrgram y, lags(3)
display "@ m8 " r(lags) " " %24.17g r(ac1) " " %24.17g r(pac3)
restore
* one panel of a panel
preserve
generate byte id = 1 + (t > 100)
xtset id t
capture dfuller y
display "@ x1 " _rc
capture pperron y
display "@ x2 " _rc
capture corrgram y
display "@ x3 " _rc
capture wntestq y
display "@ x4 " _rc
capture dfgls y
display "@ x5 " _rc
dfuller y if id == 2
display "@ x6 " r(N) " " %24.17g r(Zt) " " %24.17g r(p)
pperron y if id == 1, lags(3)
display "@ x7 " r(N) " " %24.17g r(Zrho) " " %24.17g r(Zt)
corrgram y if id == 2, lags(3)
display "@ x8 " r(lags) " " %24.17g r(ac1) " " %24.17g r(pac3)
dfgls y if id == 1, maxlag(2)
display "@ x9 " r(N) " " r(siclag)
dfuller w if id == 1, trend lags(1)
display "@ x10 " r(N) " " %24.17g r(Zt) " " %24.17g r(p)
restore
* errors
capture dfuller y x
display "@ r1 " _rc
capture dfuller y, lags(2.5)
display "@ r2 " _rc
capture dfuller y, trend noconstant
display "@ r3 " _rc
capture dfuller y, drift trend
display "@ r4 " _rc
capture dfuller y, drift noconstant
display "@ r5 " _rc
capture dfuller nosuch
display "@ r6 " _rc
capture dfuller
display "@ r7 " _rc
capture corrgram
display "@ r8 " _rc
capture corrgram y x
display "@ r9 " _rc
capture corrgram y, lags(0)
display "@ r10 " _rc
capture corrgram y, lags(200)
display "@ r11 " _rc
capture wntestq y, lags(0)
display "@ r12 " _rc
capture wntestq y, lags(200)
display "@ r13 " _rc
capture corrgram y, lags(2) nosuchopt
display "@ r14 " _rc
capture corrgram nosuch
display "@ r15 " _rc
capture pperron y, lags(2.5)
display "@ r16 " _rc
capture dfgls y, maxlag(2.5)
display "@ r17 " _rc
capture wntestq y, lags(1.5)
display "@ r18 " _rc
capture dfgls y, nosuchopt
display "@ r19 " _rc
capture dfgls y if t > 150, maxlag(60)
display "@ r20 " _rc
generate str3 s = "abc"
capture corrgram s
display "@ r21 " _rc
capture dfuller s
display "@ r22 " _rc
capture dfgls s
display "@ r23 " _rc
capture pperron s
display "@ r26 " _rc
capture wntestq s
display "@ r27 " _rc
tsset, clear
capture dfuller y
display "@ r24 " _rc
capture corrgram y
display "@ r25 " _rc
