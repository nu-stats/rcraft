* RCraft's Stata corpus (P2C.12, batch G): made-up data; Stata 19's results are in xtunitroot.recorded
* commands: xtunitroot
* (ips with and without lags, trend and demean at several numbers of panels and periods; ht with each deterministic term, altt and demean; breitung with each deterministic term and demean; fisher with dfuller (lags, trend, drift) and pperron; hadri with robust and the Bartlett and Parzen kernels; r() and the errors with their return codes)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 12
generate int id = _n
generate double r = cond(mod(_n, 3) == 0, 1, .55 + .04 * _n)
generate double a = sin(_n * 2.3)
expand 25
bysort id: generate int t = _n
xtset id t
generate double e = sin(id * 12.9898 + t * 78.233) * 1.7 + cos(id * t * 3.1) * 0.9 + sin(t * t / 11 + id)
generate double y = a + e if t == 1
replace y = a * (1 - r) + r * L.y + e if t > 1
replace y = round(y, .001)
* ips
xtunitroot ips y
display "@ i1 " r(N) " " r(N_t) " " r(N_g) " " %24.17g r(p_zttildebar) " " r(cv_10) " " r(cv_5) " " r(cv_1) " " %24.17g r(zttildebar) " " %24.17g r(zt) " " %24.17g r(ttildebar) " " %24.17g r(tbar)
display "@ i1s [" r(test) "] [" r(deterministics) "]"
xtunitroot ips y, trend
display "@ i2 " r(N) " " %24.17g r(p_zttildebar) " " r(cv_10) " " r(cv_5) " " r(cv_1) " " %24.17g r(zttildebar) " " %24.17g r(zt) " " %24.17g r(ttildebar) " " %24.17g r(tbar)
display "@ i2s [" r(deterministics) "]"
xtunitroot ips y, demean
display "@ i3 " %24.17g r(p_zttildebar) " " %24.17g r(zttildebar) " " %24.17g r(tbar)
display "@ i3s [" r(demean) "]"
xtunitroot ips y, lags(1)
display "@ i4 " r(N) " " r(lags) " " %24.17g r(p_wtbar) " " %24.17g r(wtbar) " " %24.17g r(tbar)
xtunitroot ips y, trend lags(2)
display "@ i5 " r(N) " " r(lags) " " %24.17g r(p_wtbar) " " %24.17g r(wtbar) " " %24.17g r(tbar)
xtunitroot ips y, lags(0)
display "@ i6 " r(N) " " r(lags) " " %24.17g r(wtbar) " " %24.17g r(p_wtbar)
xtunitroot ips y, demean lags(3)
display "@ i7 " r(N) " " %24.17g r(wtbar) " " %24.17g r(p_wtbar)
xtunitroot ips y if id <= 8 & t <= 23
display "@ i8 " r(N) " " r(N_t) " " r(N_g) " " r(cv_5) " " %24.17g r(zttildebar) " " %24.17g r(tbar)
xtunitroot ips y if id <= 8 & t <= 23, trend
display "@ i9 " r(cv_10) " " r(cv_5) " " r(cv_1) " " %24.17g r(zttildebar) " " %24.17g r(tbar)
xtunitroot ips y if id <= 8 & t <= 23, lags(1)
display "@ i10 " r(N) " " %24.17g r(wtbar) " " %24.17g r(tbar)
xtunitroot ips y if t <= 12
display "@ i11 " r(N) " " r(cv_5) " " %24.17g r(zttildebar) " " %24.17g r(tbar)
xtunitroot ips y if t <= 12, lags(1)
display "@ i12 " r(N) " " %24.17g r(wtbar) " " %24.17g r(tbar)
xtunitroot ips y if id <= 5, trend
display "@ i13 " r(N_g) " " r(cv_1) " " %24.17g r(zttildebar)
xtunitroot ips y if t > 5
display "@ i14 " r(N_t) " " r(cv_10) " " %24.17g r(zttildebar) " " %24.17g r(tbar)
xtunitroot ips y if id > 7 & t <= 40, lags(4) trend
display "@ i15 " r(N) " " %24.17g r(wtbar) " " %24.17g r(tbar)
* ht
xtunitroot ht y
display "@ h1 " r(N) " " r(N_t) " " r(N_g) " " %24.17g r(p) " " %24.17g r(z) " " %24.17g r(rho) " " %24.17g r(mean) " " %24.17g r(Var)
display "@ h1s [" r(test) "] [" r(deterministics) "]"
xtunitroot ht y, trend
display "@ h2 " r(N) " " %24.17g r(p) " " %24.17g r(z) " " %24.17g r(rho) " " %24.17g r(mean) " " %24.17g r(Var)
xtunitroot ht y, noconstant
display "@ h3 " r(N) " " %24.17g r(p) " " %24.17g r(z) " " %24.17g r(rho) " " %24.17g r(mean) " " %24.17g r(Var)
display "@ h3s [" r(deterministics) "]"
xtunitroot ht y, demean altt
display "@ h4 " r(N) " " %24.17g r(p) " " %24.17g r(z) " " %24.17g r(rho) " " %24.17g r(mean) " " %24.17g r(Var)
xtunitroot ht y, trend altt
display "@ h5 " %24.17g r(p) " " %24.17g r(rho) " " %24.17g r(mean) " " %24.17g r(Var)
xtunitroot ht y, noconstant altt demean
display "@ h6 " %24.17g r(p) " " %24.17g r(rho) " " %24.17g r(mean) " " %24.17g r(Var)
xtunitroot ht y if id <= 7 & t <= 15
display "@ h7 " r(N) " " r(N_t) " " r(N_g) " " %24.17g r(p) " " %24.17g r(rho)
* breitung
xtunitroot breitung y
display "@ b1 " r(lags) " " r(N) " " r(N_t) " " r(N_g) " " %24.17g r(p) " " %24.17g r(lambda)
display "@ b1s [" r(test) "] [" r(deterministics) "]"
xtunitroot breitung y, trend
display "@ b2 " %24.17g r(p) " " %24.17g r(lambda)
xtunitroot breitung y, noconstant
display "@ b3 " %24.17g r(p) " " %24.17g r(lambda)
display "@ b3s [" r(deterministics) "]"
xtunitroot breitung y, demean
display "@ b4 " %24.17g r(p) " " %24.17g r(lambda)
xtunitroot breitung y, trend demean
display "@ b5 " %24.17g r(p) " " %24.17g r(lambda)
xtunitroot breitung y if id <= 6 & t <= 12, noconstant demean
display "@ b6 " r(N) " " %24.17g r(p) " " %24.17g r(lambda)
* fisher
xtunitroot fisher y, dfuller lags(1)
display "@ f1 " r(N) " " r(N_t) " " r(N_g) " " %24.17g r(P) " " r(df_P) " " %24.17g r(p_P) " " %24.17g r(Z) " " %24.17g r(p_Z) " " %24.17g r(L) " " r(df_L) " " %24.17g r(p_L) " " %24.17g r(Pm) " " %24.17g r(p_Pm)
display "@ f1s [" r(test) "] [" r(urtest) "] [" r(options) "]"
xtunitroot fisher y, dfuller trend lags(0)
display "@ f2 " %24.17g r(P) " " %24.17g r(Z) " " %24.17g r(L) " " %24.17g r(Pm)
display "@ f2s [" r(options) "]"
xtunitroot fisher y, dfuller drift lags(2)
display "@ f3 " %24.17g r(P) " " %24.17g r(Z) " " %24.17g r(L) " " %24.17g r(Pm)
display "@ f3s [" r(options) "]"
xtunitroot fisher y, pperron lags(2)
display "@ f4 " %24.17g r(P) " " %24.17g r(Z) " " %24.17g r(L) " " %24.17g r(Pm)
display "@ f4s [" r(urtest) "] [" r(options) "]"
xtunitroot fisher y if id <= 8, dfuller lags(1)
display "@ f6 " r(N_g) " " r(df_L) " " %24.17g r(P) " " %24.17g r(L) " " %24.17g r(Pm) " " %24.17g r(Z)
xtunitroot fisher y if id <= 5, dfuller lags(1)
display "@ f7 " r(N_g) " " %24.17g r(P) " " %24.17g r(L) " " %24.17g r(Pm)
xtunitroot fisher y if id <= 3, dfuller lags(1)
display "@ f8 " r(N_g) " " %24.17g r(P) " " %24.17g r(L) " " %24.17g r(Pm)
xtunitroot fisher y if t <= 15, pperron lags(1)
display "@ f9 " r(N) " " %24.17g r(P) " " %24.17g r(Z)
xtunitroot fisher y if id >= 4, dfuller lags(3) demean
display "@ f10 " r(N_g) " " %24.17g r(P) " " %24.17g r(Z) " " %24.17g r(L)
* hadri
xtunitroot hadri y
display "@ a1 " r(N) " " r(N_t) " " r(N_g) " " %24.17g r(p) " " %24.17g r(z)
display "@ a1s [" r(test) "] [" r(deterministics) "]"
xtunitroot hadri y, trend
display "@ a2 " %24.17g r(p) " " %24.17g r(z)
xtunitroot hadri y, robust
display "@ a3 " %24.17g r(p) " " %24.17g r(z)
xtunitroot hadri y, kernel(bartlett 3)
display "@ a4 " %24.17g r(p) " " %24.17g r(z)
xtunitroot hadri y, trend kernel(parzen 2) demean
display "@ a5 " %24.17g r(p) " " %24.17g r(z)
xtunitroot hadri y, kernel(parzen 4)
display "@ a6 " %24.17g r(p) " " %24.17g r(z)
xtunitroot hadri y, trend robust demean
display "@ a7 " %24.17g r(p) " " %24.17g r(z)
xtunitroot hadri y if id <= 8 & t <= 23
display "@ a8 " r(N) " " %24.17g r(p) " " %24.17g r(z)
* a sample that starts late reads the periods before it for the lags
xtunitroot ips y if t > 5
display "@ s1 " r(N) " " r(N_t) " " r(cv_5) " " %24.17g r(zttildebar) " " %24.17g r(tbar)
xtunitroot ips y if t > 5, trend
display "@ s2 " r(N) " " r(N_t) " " r(cv_5) " " %24.17g r(zttildebar) " " %24.17g r(tbar)
xtunitroot ips y if t > 5, lags(1)
display "@ s3 " r(N) " " r(N_t) " " r(lags) " " %24.17g r(wtbar) " " %24.17g r(tbar)
xtunitroot ht y if t > 5
display "@ s4 " r(N) " " r(N_t) " " %24.17g r(rho) " " %24.17g r(z)
xtunitroot breitung y if t > 5
display "@ s6 " r(N) " " r(N_t) " " %24.17g r(lambda)
xtunitroot breitung y if t > 5, trend
display "@ s7 " r(N) " " r(N_t) " " %24.17g r(lambda)
xtunitroot hadri y if t > 5
display "@ s8 " r(N) " " r(N_t) " " %24.17g r(z)
xtunitroot fisher y if t > 5, dfuller lags(1)
display "@ s9 " r(N) " " r(N_t) " " %24.17g r(P) " " %24.17g r(L)
xtunitroot fisher y if t > 5 & id > 2, dfuller lags(2) trend
display "@ s11 " r(N) " " r(N_t) " " r(N_g) " " %24.17g r(P) " " %24.17g r(L)
xtunitroot fisher y if t > 5, dfuller lags(1) demean
display "@ s12 " r(N) " " %24.17g r(P) " " %24.17g r(L)
xtunitroot ips y if id == 3
display "@ s15 " r(N_g) " " r(N) " " %24.17g r(tbar)
capture xtunitroot ht y if id == 3
display "@ s16 " _rc " " r(N_g) " " %24.17g r(rho)
capture xtunitroot hadri y if id == 3
display "@ s17 " _rc
capture xtunitroot breitung y if id == 3
display "@ s18 " _rc
capture xtunitroot fisher y if id == 3, dfuller lags(1)
display "@ s19 " _rc
* demeaned series and series with a deterministic trend that start late
xtunitroot ips y if t > 5, demean
display "@ u1 " r(N) " " r(N_t) " " %24.17g r(zttildebar) " " %24.17g r(tbar)
xtunitroot ips y if t > 5, demean lags(2)
display "@ u2 " r(N) " " %24.17g r(wtbar)
xtunitroot ht y if t > 5, demean
display "@ u3 " r(N) " " %24.17g r(rho) " " %24.17g r(z)
xtunitroot ht y if t > 5, noconstant
display "@ u4 " r(N) " " %24.17g r(rho) " " %24.17g r(z)
xtunitroot ht y if t > 5, altt
display "@ u5 " r(N) " " %24.17g r(rho) " " %24.17g r(z)
xtunitroot fisher y if t > 5 & id != 6, dfuller trend lags(1)
display "@ u6 " r(N) " " %24.17g r(P) " " %24.17g r(L)
xtunitroot fisher y if t > 5, dfuller drift lags(1)
display "@ u7 " r(N) " " %24.17g r(P) " " %24.17g r(L)
xtunitroot fisher y if t > 8 & t < 22, dfuller lags(2) trend
display "@ u8 " r(N) " " r(N_t) " " %24.17g r(P) " " %24.17g r(L)
xtunitroot ips y if t > 8 & t < 22, trend lags(1)
display "@ u9 " r(N) " " %24.17g r(wtbar)
xtunitroot breitung y if t > 8 & t < 22
display "@ u10 " r(N) " " %24.17g r(lambda)
xtunitroot hadri y if t > 8 & t < 22, trend
display "@ u11 " r(N) " " %24.17g r(z)
* errors
capture xtunitroot ips y r
display "@ e1 " _rc
capture xtunitroot ips
display "@ e2 " _rc
capture xtunitroot nosuch y
display "@ e3 " _rc
capture xtunitroot ips y, noconstant
display "@ e4 " _rc
capture xtunitroot hadri y, noconstant
display "@ e5 " _rc
capture xtunitroot ips y, nosuchopt
display "@ e6 " _rc
capture xtunitroot fisher y, lags(1)
display "@ e7 " _rc
capture xtunitroot fisher y, dfuller
display "@ e8 " _rc
capture xtunitroot fisher y, dfuller pperron lags(1)
display "@ e9 " _rc
capture xtunitroot fisher y, dfuller trend drift lags(1)
display "@ e10 " _rc
capture xtunitroot ht y, trend noconstant
display "@ e11 " _rc
capture xtunitroot ips nosuch
display "@ e12 " _rc
capture xtunitroot ips y, lags(-1)
display "@ e13 " _rc
generate str3 s = "abc"
capture xtunitroot ips s
display "@ e16 " _rc
preserve
keep if id == 1
tsset t
capture xtunitroot ips y
display "@ e17 " _rc
restore
xtset, clear
capture xtunitroot ips y
display "@ e18 " _rc
