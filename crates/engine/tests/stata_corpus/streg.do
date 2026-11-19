* RCraft's Stata corpus (P2C.12, batch K): made-up data; Stata 19's results are in streg.recorded
* commands: stset streg
* (loglogistic with late entry: the coefficient of x is compared to 7 decimals, Stata stops 3e-8 off the maximum even at its tightest tolerance; streg with the exponential, Weibull, lognormal, loglogistic and Gompertz distributions: hazard ratios, nohr, time, tr, robust, vce(cluster), if, late entry with time0(), the log likelihoods, the model test and e())
* Stata is run with tolerance(), ltolerance() and nrtolerance() set tight, so its estimates are the maximum (at its default it stops about 1e-7 short)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 60
generate int id = _n
generate byte group = 1 + (_n > 30)
generate double x = round(sin(_n * 12.9898) * 2, .01)
generate byte s = mod(_n, 2)
generate int t = 1 + floor(abs(sin(_n * 3.7)) * 24 * (1 + (group == 1)))
generate byte died = abs(cos(_n * 5.1)) < 0.8
local tol "tolerance(1e-13) ltolerance(1e-15) nrtolerance(1e-13)"
stset t, failure(died)
foreach d in exponential weibull lognormal loglogistic gompertz {
    streg x group, distribution(`d') nolog `tol'
    display "@ `d'_b " %24.17g _b[x] " " %24.17g _b[group] " " %24.17g _b[_cons]
    display "@ `d'_se " %24.17g _se[x] " " %24.17g _se[group] " " %24.17g _se[_cons]
    display "@ `d'_e " e(N) " " e(N_sub) " " e(N_fail) " " %24.17g e(risk) " " %24.17g e(ll) " " %24.17g e(ll_0) " " %24.17g e(chi2) " " e(df_m) " " %24.17g e(p)
}
streg x group, distribution(weibull) nolog `tol'
display "@ w_lnp " %24.17g _b[/ln_p] " " %24.17g _se[/ln_p] " " %24.17g e(aux_p)
streg x group, distribution(lognormal) nolog `tol'
display "@ ln_sig " %24.17g _b[/lnsigma] " " %24.17g _se[/lnsigma] " " %24.17g e(sigma)
streg x group, distribution(loglogistic) nolog `tol'
display "@ ll_gam " %24.17g _b[/lngamma] " " %24.17g _se[/lngamma] " " %24.17g e(gamma)
streg x group, distribution(gompertz) nolog `tol'
display "@ gp_gam " %24.17g _b[/gamma] " " %24.17g _se[/gamma] " " %24.17g e(gamma)
streg x group, distribution(exponential) time nolog `tol'
display "@ et_b " %24.17g _b[x] " " %24.17g _b[group] " " %24.17g _b[_cons] " " %24.17g _se[x] " " %24.17g _se[_cons] " " %24.17g e(ll)
streg x group, distribution(weibull) time nolog `tol'
display "@ wt_b " %24.17g _b[x] " " %24.17g _b[group] " " %24.17g _b[_cons] " " %24.17g _se[x] " " %24.17g _se[_cons] " " %24.17g _b[/ln_p] " " %24.17g _se[/ln_p] " " %24.17g e(chi2)
streg x group, distribution(weibull) tr nolog `tol'
display "@ wr_b " %24.17g _b[x] " " %24.17g _b[group] " " %24.17g _b[_cons] " " %24.17g e(ll)
streg x group, distribution(weibull) robust nolog `tol'
display "@ wrob_se " %24.17g _se[x] " " %24.17g _se[group] " " %24.17g _se[_cons] " " %24.17g _se[/ln_p] " " %24.17g e(chi2) " " %24.17g e(p) " " %24.17g e(ll) " " %24.17g e(ll_0) " " e(df_m)
streg x group, distribution(weibull) vce(cluster s) nolog `tol'
display "@ wcl_se " %24.17g _se[x] " " %24.17g _se[group] " " %24.17g _se[_cons] " " %24.17g _se[/ln_p] " " e(N_clust) " " e(df_m)
streg x group, distribution(lognormal) robust nolog `tol'
display "@ lnrob_se " %24.17g _se[x] " " %24.17g _se[group] " " %24.17g _se[_cons] " " %24.17g _se[/lnsigma] " " %24.17g e(chi2)
streg x group, distribution(exponential) vce(cluster s) nolog `tol'
display "@ excl_se " %24.17g _se[x] " " %24.17g _se[group] " " %24.17g _se[_cons] " " e(N_clust)
streg x group if x > -1, distribution(weibull) nolog `tol'
display "@ wif_b " %24.17g _b[x] " " %24.17g _b[group] " " %24.17g _b[/ln_p] " " e(N) " " %24.17g e(ll)
streg group, distribution(loglogistic) nolog `tol'
display "@ llg_b " %24.17g _b[group] " " %24.17g _b[_cons] " " %24.17g _b[/lngamma] " " %24.17g e(ll)
streg, distribution(weibull) nolog `tol'
display "@ w0_b " %24.17g _b[_cons] " " %24.17g _b[/ln_p] " " %24.17g e(ll) " " e(df_m) " " %24.17g e(chi2)
matrix V = e(V)
display "@ w0_V " %24.17g V[1,1] " " %24.17g V[2,2] " " %24.17g V[1,2]
* time0() with gaps: late entry
clear
set obs 60
generate int id = ceil(_n / 2)
generate byte rec = 2 - mod(_n, 2)
generate double a = 1 + floor(abs(sin(id * 2.3)) * 9)
generate double t0 = cond(rec == 1, 0, a + floor(abs(sin(id * 5.7)) * 3))
generate double t1 = cond(rec == 1, a, t0 + 1 + floor(abs(cos(id * 4.1)) * 8))
generate byte fail = rec == 2 & abs(sin(id * 1.9)) < 0.7
generate double x = round(sin(id * 12.9898) * 2, .01)
stset t1, failure(fail) id(id) time0(t0)
foreach d in exponential weibull lognormal loglogistic gompertz {
    streg x, distribution(`d') nolog `tol'
    scalar bx = _b[x]
    if "`d'" == "loglogistic" {
        scalar bx = round(bx, .0000001)
    }
    display "@ `d'_t0b " %24.17g bx " " %24.17g _b[_cons] " " %24.17g _se[x] " " %24.17g e(ll) " " %24.17g e(ll_0) " " e(N) " " e(N_sub) " " %24.17g e(risk)
}
streg x, distribution(weibull) robust nolog `tol'
display "@ wt0rob " %24.17g _se[x] " " %24.17g _se[_cons] " " %24.17g _se[/ln_p] " " e(N_clust) " " %24.17g e(chi2)
