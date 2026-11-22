* RCraft's Stata corpus (P2C.12ar, batch J): made-up data; Stata 19's results are in intreg.recorded
* commands: intreg
* (intreg: Olsen's parameters, interval, left, right and exact outcomes, LR or Wald test, robust, clustered, weights, factor variables, replay, predict)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 80
generate double x1 = mod(_n * 37, 101) / 25 - 2
generate byte x2 = mod(_n * 7, 5) < 2
generate byte g = 1 + mod(_n, 3)
generate byte id = 1 + mod(_n, 8)
generate double e = invnormal((mod(_n * 53, 997) + 0.5) / 997)
generate double ystar = 1 + 0.8 * x1 - 0.5 * x2 + e
generate double lo = floor(ystar)
generate double hi = lo + 1
replace lo = . in 1/6
replace hi = . in 70/75
replace hi = lo in 20/26
generate double fw = 1 + mod(_n, 3)
generate double pw = 0.5 + mod(_n, 4) / 2
program define shwi
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(ll_0) " " %24.17g e(chi2) " " %24.17g e(p) " " e(df_m) " " e(rank) " " e(rank0) " " e(k) " " e(k_eq) " " e(k_eq_model)
    display "@ c " e(N_unc) " " e(N_lc) " " e(N_rc) " " e(N_int) " " e(converged) " " e(ic) " " e(k_aux) " " e(k_dv)
    display "@ sg " %24.17g e(sigma) " " %24.17g e(se_sigma)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
intreg lo hi x1 x2
shwi
display "@ m1 " (e(cmd) == "intreg") " " (e(predict) == "intreg_p") " " (e(title) == "Interval regression") " " (e(depvar) == "lo hi") " " (e(vce) == "oim") " " (e(chi2type) == "LR")
intreg lo hi x1 x2, nolog
shwi
intreg lo hi x1 x2, vce(robust) nolog
shwi
display "@ m2 " (e(vcetype) == "Robust") " " (e(chi2type) == "Wald")
intreg lo hi x1 x2, vce(cluster id) nolog
shwi
display "@ m3 " e(N_clust) " " (e(clustvar) == "id")
intreg lo hi x1 x2 if x1 > -1, level(90) nolog
shwi
intreg lo hi x1 x2 [fweight = fw], nolog
shwi
display "@ m4 " (e(wtype) == "fweight") " " (e(wexp) == "= fw")
intreg lo hi x1 x2 [pweight = pw], nolog
shwi
intreg lo hi x1 x2 [iweight = pw], nolog
shwi
intreg lo hi x1 x2 [pweight = pw], vce(cluster id) nolog
shwi
intreg lo hi i.g x1, nolog
shwi
intreg lo hi x1 x2
intreg, level(80)
display "@ lv " _b[x1] " " _se[x1] " " _b[_cons]
intreg lo hi x1 x2, nolog
predict double xb0
predict double xb1, xb
predict double sd, stdp
predict double sf, stdf
predict double p1, pr(0, .)
predict double p2, pr(., 2)
predict double p3, pr(0.5, 1.5)
predict double p4, e(0, .)
predict double p5, e(., 2)
predict double p6, e(0.5, 1.5)
predict double p7, ystar(0, .)
predict double p8, ystar(0, 2)
predict double p9, ystar(0.5, 1.5)
foreach v in xb0 xb1 sd sf p1 p2 p3 p4 p5 p6 p7 p8 p9 {
    quietly summarize `v'
    display "@ pr_`v' " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
}
display "@ bx " _b[x1] " " _b[x2] " " _se[x2]
test x1 = x2
display "@ tt " %24.17g r(chi2) " " %24.17g r(p) " " r(df)
lincom x1 + x2
display "@ lc " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(z)
