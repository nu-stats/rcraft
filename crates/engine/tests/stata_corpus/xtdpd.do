* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in xtdpd.recorded
* commands: xtabond xtdpdsys xtdpd
* (difference and system GMM: one and two steps, vce(robust) and Windmeijer's two-step variance,
* lags(), maxldep(), maxlags(), pre(), endogenous(), inst(), noconstant, diffvars(), xtdpd's
* dgmmiv(), lgmmiv(), div(), liv() and L(#/#).y; unbalanced panels; estat sargan and abond;
* predict; Stata's errors)
* tolerance: 1e-7
version 19
set linesize 255
clear
set obs 40
generate int id = _n
generate double u = 0.8 * sin(_n * 1.37)
expand 7
bysort id: generate int t = _n
xtset id t
generate double x = round(sin(_n * 0.913) + 0.5 * cos(_n * 2.17), .01)
generate double w = round(cos(_n * 1.71) + 0.3 * x, .01)
generate double q = round(x + 0.7 * sin(_n * 4.03), .01)
generate double e = sin(_n * 3.11) + 0.6 * cos(_n * 0.57 + id)
generate double y = u + e if t == 1
bysort id (t): replace y = 0.5 * y[_n-1] + 0.4 * x + 0.2 * w + u + e if t > 1
replace y = round(y, .001)
quietly xtabond y x
display "@ ab " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2) " " %20.0g el(e(b), 1, 3) " " %20.0g el(e(V), 1, 1) " " %20.0g el(e(V), 2, 3)
display "@ ab_e " %20.0g e(sig2) " " %20.0g e(chi2) " " %20.0g e(sargan) " " e(N) " " e(N_g) " " e(zrank) " " e(artests) " " %20.0g e(arm1) " " %20.0g e(arm2)
quietly xtabond y x, twostep vce(robust)
display "@ ab_wc " %20.0g el(e(b), 1, 1) " " %20.0g el(e(V), 1, 1) " " %20.0g el(e(V), 3, 3) " " %20.0g e(arm1) " " %20.0g e(arm2)
quietly xtabond y x w, lags(2) maxldep(3) vce(robust)
display "@ ab_lags " %20.0g el(e(b), 1, 2) " " %20.0g el(e(V), 2, 2) " " e(zrank) " " e(N)
quietly xtdpdsys y x
display "@ sys " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2) " " %20.0g el(e(b), 1, 3) " " %20.0g el(e(V), 1, 1) " " %20.0g el(e(V), 1, 3)
display "@ sys_e " %20.0g e(sig2) " " %20.0g e(rss) " " %20.0g e(chi2) " " %20.0g e(sargan) " " e(N) " " e(zrank) " " e(artests)
capture estat abond
display "@ sys_abond_rc " _rc
quietly xtdpdsys y x, twostep
display "@ sys_two " %20.0g el(e(b), 1, 1) " " %20.0g el(e(V), 1, 1) " " %20.0g e(sargan) " " %20.0g e(sig2)
quietly estat abond, artests(3)
display "@ sys_arm " %20.0g el(r(arm), 1, 2) " " %20.0g el(r(arm), 2, 2) " " %20.0g el(r(arm), 3, 3) " " e(artests) " " %20.0g e(arm3)
quietly xtdpdsys y x, twostep vce(robust)
display "@ sys_wc " %20.0g el(e(b), 1, 2) " " %20.0g el(e(V), 1, 1) " " %20.0g el(e(V), 2, 3)
quietly xtdpdsys y w, pre(x) maxlags(2)
display "@ sys_pre " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2) " " %20.0g el(e(V), 2, 2) " " e(zrank)
quietly xtdpdsys y w, endogenous(x) lags(2)
display "@ sys_endog " %20.0g el(e(b), 1, 3) " " %20.0g el(e(V), 3, 3) " " e(zrank)
quietly xtdpdsys y x, inst(q) maxldep(2)
display "@ sys_inst " %20.0g el(e(b), 1, 1) " " %20.0g el(e(V), 2, 2) " " e(zrank)
quietly xtdpdsys y x w, noconstant diffvars(w) twostep
display "@ sys_nocons " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 3) " " %20.0g e(sargan) " " e(N)
quietly xtdpdsys y w, pre(x, lagstruct(1, 2))
display "@ sys_lagstruct " %20.0g el(e(b), 1, 2) " " %20.0g el(e(b), 1, 3) " " e(zrank)
quietly xtdpd L(0/1).y x w, dgmmiv(y) lgmmiv(y) div(x) liv(w)
display "@ dpd " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 3) " " %20.0g el(e(V), 1, 1) " " e(N) " " e(zrank)
quietly xtdpd L(0/2).y x, dgmmiv(y, lagrange(2 4)) div(x) twostep vce(robust)
display "@ dpd_wc " %20.0g el(e(b), 1, 2) " " %20.0g el(e(V), 2, 2) " " %20.0g e(arm1) " " e(N)
quietly xtdpd L(1/2).y x, dgmmiv(y) div(x)
display "@ dpd_l12 " %20.0g el(e(b), 1, 1) " " %20.0g el(e(V), 1, 1) " " e(N)
quietly xtdpdsys y x
predict double p1
predict double p2, e difference
summarize p1, meanonly
display "@ predict_xb " %20.0g r(mean) " " r(N)
summarize p2, meanonly
display "@ predict_ed " %20.0g r(mean) " " r(N)
drop if inlist(id, 3, 8) & t == 4
drop if id > 35 & t > 5
replace x = . if id == 12 & t == 5
quietly xtdpdsys y x
display "@ unbal " %20.0g el(e(b), 1, 1) " " %20.0g el(e(V), 1, 1) " " e(N) " " e(g_min) " " e(g_max) " " %20.0g e(g_avg)
quietly xtabond y x, vce(robust)
display "@ unbal_ab " %20.0g el(e(b), 1, 2) " " %20.0g el(e(V), 2, 2) " " e(N) " " %20.0g e(arm1)
capture xtdpdsys y x, lags(0)
display "@ err_lags " _rc
capture xtdpdsys y x, maxldep(0)
display "@ err_maxldep " _rc
capture xtdpdsys y, diffvars(x) noconstant
display "@ err_diffvars " _rc
capture xtdpd L(0/1).y x
display "@ err_dgmmiv " _rc
capture xtdpd L(0/1).y x, dgmmiv(y, lagrange(3 2))
display "@ err_lagrange " _rc
