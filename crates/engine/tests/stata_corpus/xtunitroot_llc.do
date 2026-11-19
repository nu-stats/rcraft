* RCraft's Stata corpus (P2C.12r, batch G second round): made-up data; Stata 19's results are in xtunitroot_llc.recorded
* commands: xtunitroot
* (Levin-Lin-Chu with constant, noconstant, trend, demean, lags(), kernel(bartlett # | parzen # | ... llc); the adjustment table at many numbers of periods; samples that start late; r() and the errors with their return codes)
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
* defaults and each deterministic term
xtunitroot llc y
display "@ l1 " r(N) " " r(N_t) " " r(N_g) " " r(ttilde) " " r(adf_lags) " " r(hac_lagm) " " r(mu_adj) " " r(sig_adj)
display "@ l1a " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(p_tds)
display "@ l1b " %24.17g r(delta) " " %24.17g r(se_delta) " " %24.17g r(Var_ep) " " %24.17g r(sbar)
display "@ l1s [" r(test) "] [" r(deterministics) "] [" r(hac_kernel) "] [" r(hac_method) "]"
xtunitroot llc y, trend
display "@ l2 " r(N) " " r(ttilde) " " r(mu_adj) " " r(sig_adj) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(p_tds)
display "@ l2b " %24.17g r(delta) " " %24.17g r(se_delta) " " %24.17g r(Var_ep) " " %24.17g r(sbar)
display "@ l2s [" r(deterministics) "]"
xtunitroot llc y, noconstant
display "@ l3 " r(N) " " r(ttilde) " " r(mu_adj) " " r(sig_adj) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(p_td) " " %24.17g r(p_tds)
display "@ l3b " %24.17g r(delta) " " %24.17g r(se_delta) " " %24.17g r(Var_ep) " " %24.17g r(sbar)
display "@ l3s [" r(deterministics) "]"
xtunitroot llc y, demean
display "@ l4 " r(N) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(p_tds) " " %24.17g r(sbar) " " %24.17g r(Var_ep)
display "@ l4s [" r(demean) "]"
xtunitroot llc y, demean trend
display "@ l5 " r(N) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, demean noconstant
display "@ l6 " r(N) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
* lags()
xtunitroot llc y, lags(0)
display "@ l7 " r(N) " " r(ttilde) " " r(adf_lags) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, lags(2)
display "@ l8 " r(N) " " r(ttilde) " " r(adf_lags) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, lags(3) trend
display "@ l9 " r(N) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, lags(0) noconstant
display "@ l10 " r(N) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, lags(4) demean
display "@ l11 " r(N) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
* kernel()
xtunitroot llc y, kernel(bartlett 3)
display "@ k1 " r(hac_lags) " " r(hac_lagm) " " %24.17g r(tds) " " %24.17g r(sbar)
display "@ k1s [" r(hac_kernel) "] [" r(hac_method) "]"
xtunitroot llc y, kernel(parzen 3)
display "@ k2 " r(hac_lags) " " %24.17g r(tds) " " %24.17g r(sbar)
display "@ k2s [" r(hac_kernel) "]"
xtunitroot llc y, kernel(parzen 6) trend lags(2)
display "@ k3 " r(hac_lags) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, kernel(bartlett 0) lags(1)
display "@ k4 " r(hac_lags) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, kernel(bartlett 1) noconstant lags(2)
display "@ k5 " r(hac_lags) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, kernel(bartlett 20)
display "@ k6 " r(hac_lags) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, kernel(parzen 30) trend
display "@ k7 " r(hac_lags) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y, kernel(bartlett llc) lags(2)
display "@ k8 " r(hac_lagm) " " %24.17g r(tds) " " %24.17g r(sbar)
display "@ k8s [" r(hac_kernel) "] [" r(hac_method) "]"
xtunitroot llc y, kernel(parzen llc) demean
display "@ k9 " r(hac_lagm) " " %24.17g r(tds) " " %24.17g r(sbar)
display "@ k9s [" r(hac_kernel) "] [" r(hac_method) "]"
* samples
xtunitroot llc y if id <= 8 & t <= 23
display "@ s1 " r(N) " " r(N_t) " " r(N_g) " " r(ttilde) " " r(hac_lagm) " " %24.17g r(td) " " %24.17g r(tds)
xtunitroot llc y if id > 2 & id < 9, trend
display "@ s2 " r(N) " " r(N_g) " " %24.17g r(td) " " %24.17g r(tds)
xtunitroot llc y if id == 5
display "@ s3 " r(N) " " r(N_g) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y in 1/150
display "@ s4 " r(N) " " r(N_g) " " %24.17g r(td) " " %24.17g r(tds)
xtunitroot llc y in 26/200
display "@ s5 " r(N) " " r(N_g) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds)
* a sample that starts late reads the one period before it
xtunitroot llc y if t > 3
display "@ w1 " r(N) " " r(N_t) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y if t > 3, lags(2)
display "@ w2 " r(N) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y if t > 3, trend
display "@ w3 " r(N) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y if t > 3, demean
display "@ w4 " r(N) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y if t < 20, lags(1)
display "@ w5 " r(N) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y if t > 3 & t < 20, demean trend
display "@ w6 " r(N) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y if t > 6 & id != 4, noconstant lags(0)
display "@ w7 " r(N) " " r(N_g) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
xtunitroot llc y if t > 6 & t <= 22, kernel(parzen 3) lags(3)
display "@ w8 " r(N) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " %24.17g r(sbar)
* the adjustment table at many numbers of periods
clear
set obs 5
generate int id = _n
generate double r = .7 + .05 * _n
generate double a = cos(_n * 1.7)
expand 300
bysort id: generate int t = _n
xtset id t
generate double e = sin(id * 12.9898 + t * 78.233) * 1.7 + cos(id * t * 3.1) * 0.9 + sin(t * t / 11 + id)
generate double y = a + e if t == 1
replace y = a * (1 - r) + r * L.y + e if t > 1
replace y = round(y, .001)
foreach n in 8 15 26 27 29 30 33 38 40 41 44 47 52 58 61 65 73 82 95 100 101 130 200 250 251 252 300 {
    quietly xtunitroot llc y if t <= `n', lags(0)
    display "@ t`n'c " r(ttilde) " " r(mu_adj) " " r(sig_adj) " " %24.17g r(tds)
    quietly xtunitroot llc y if t <= `n', lags(0) trend
    display "@ t`n't " r(ttilde) " " r(mu_adj) " " r(sig_adj) " " %24.17g r(tds)
    quietly xtunitroot llc y if t <= `n', lags(0) noconstant
    display "@ t`n'n " r(ttilde) " " r(mu_adj) " " r(sig_adj) " " %24.17g r(tds)
}
quietly xtunitroot llc y if t > 40 & t <= 120, lags(2) trend
display "@ v1 " r(N) " " r(N_t) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds) " " r(hac_lagm)
quietly xtunitroot llc y if t > 40 & t <= 120, lags(1) demean
display "@ v2 " r(N) " " r(N_t) " " r(ttilde) " " %24.17g r(td) " " %24.17g r(tds)
quietly xtunitroot llc y
display "@ v3 " r(N) " " r(N_t) " " r(ttilde) " " r(hac_lagm) " " %24.17g r(td) " " %24.17g r(tds)
* errors
capture xtunitroot llc
display "@ e1 " _rc
capture xtunitroot llc y r
display "@ e2 " _rc
capture xtunitroot llc nosuch
display "@ e3 " _rc
capture xtunitroot llc y, trend noconstant
display "@ e4 " _rc
capture xtunitroot llc y, robust
display "@ e5 " _rc
capture xtunitroot llc y, kernel(qs 3)
display "@ e6 " _rc
capture xtunitroot llc y, kernel(bartlett)
display "@ e7 " _rc
capture xtunitroot llc y, kernel(bartlett -1)
display "@ e8 " _rc
capture xtunitroot llc y, kernel(bartlett 3 4)
display "@ e9 " _rc
capture xtunitroot llc y, lags(-1)
display "@ e10 " _rc
capture xtunitroot llc y, lags(2.5)
display "@ e11 " _rc
capture xtunitroot llc y, lags(1) lags(2)
display "@ e12 " _rc
capture xtunitroot llc y, kernel(bartlett 3) kernel(parzen 2)
display "@ e13 " _rc
capture xtunitroot llc y, kernel(bartlett 2.5)
display "@ e14 " _rc
capture xtunitroot llc y, altt
display "@ e15 " _rc
drop if id == 2 & t == 5
capture xtunitroot llc y
display "@ e16 " _rc
xtset, clear
capture xtunitroot llc y
display "@ e17 " _rc
