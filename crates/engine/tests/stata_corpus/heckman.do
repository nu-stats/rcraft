* RCraft's Stata corpus (P2C.12ar, batch J): made-up data; Stata 19's results are in heckman.recorded
* commands: heckman
* (heckman: ML from the two-step start, twostep, select(s = ...), LR or Wald test of rho, robust, clustered, fw/pw/iw weights, replay, predict)
* tolerance: 1e-5
version 19
set linesize 255
clear
set obs 300
generate double x1 = mod(_n * 37, 101) / 25 - 2
generate double x2 = mod(_n * 29, 53) / 53
generate double z1 = invnormal((mod(_n * 41, 307) + 0.5) / 307)
generate byte z2 = mod(_n * 7, 5) < 2
generate byte id = 1 + mod(_n, 15)
generate double e1 = invnormal((mod(_n * 53, 997) + 0.5) / 997)
generate double u = invnormal((mod(_n * 71, 991) + 0.5) / 991)
generate double e2 = 0.5 * e1 + sqrt(0.75) * u
generate byte s = 0.3 + 0.8 * z1 + 0.5 * z2 + 0.4 * x1 + e2 > 0
generate double y = 1 + 0.7 * x1 - 0.4 * x2 + e1 if s
generate double fw = 1 + mod(_n, 3)
generate double pw = 0.5 + mod(_n, 4) / 2
program define shwk
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(chi2) " " %24.17g e(p) " " e(df_m) " " e(rank) " " e(k) " " e(k_eq) " " e(k_dv) " " e(k_eq_model)
    display "@ t " %24.17g e(rho) " " %24.17g e(sigma) " " %24.17g e(lambda) " " %24.17g e(selambda) " " e(N_selected) " " e(N_nonselected)
    display "@ u " %24.17g e(p_c) " " %24.17g e(chi2_c) " " e(k_aux) " " e(converged) " " e(ic) " " e(rc)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
program define shwt
    display "@ s " %24.17g e(N) " " %24.17g e(chi2) " " %24.17g e(p) " " e(df_m) " " e(rank)
    display "@ t " %24.17g e(rho) " " %24.17g e(sigma) " " %24.17g e(lambda) " " %24.17g e(selambda) " " e(N_selected) " " e(N_nonselected) " " e(fconverged)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
heckman y x1 x2, select(z1 z2 x1)
shwk
display "@ m1 " (e(cmd) == "heckman") " " (e(predict) == "heckma_p") " " (e(vce) == "oim") " " (e(method) == "ml") " " (e(chi2_ct) == "LR") " " (e(depvar) == "y")
heckman y x1 x2, select(z1 z2 x1) nolog
shwk
heckman y x1 x2, select(s = z1 z2 x1) nolog
shwk
display "@ m2 " (e(depvar) == "y s")
heckman y x1 x2, select(z1 z2 x1) vce(robust) nolog
shwk
display "@ m3 " (e(vcetype) == "Robust") " " (e(chi2_ct) == "Wald")
heckman y x1 x2, select(z1 z2 x1) vce(cluster id) nolog
shwk
display "@ m4 " e(N_clust) " " (e(clustvar) == "id")
heckman y x1 x2, select(z1 z2 x1) level(90) nolog
shwk
heckman y x1 x2 [fweight = fw], select(z1 z2 x1) nolog
shwk
display "@ m5 " (e(wtype) == "fweight") " " (e(wexp) == "= fw")
heckman y x1 x2 [pweight = pw], select(z1 z2 x1) nolog
shwk
heckman y x1 x2 [iweight = pw], select(z1 z2 x1) nolog
shwk
heckman y x1 x2 [pweight = pw], select(z1 z2 x1) vce(cluster id) nolog
shwk
capture noisily heckman y x1 [aweight = pw], select(s = z2)
display "@ rc1 " _rc
capture noisily heckman y x1 x2 [fweight = fw], select(z1 z2 x1) twostep
display "@ rc2 " _rc
capture noisily heckman y x1 x2
display "@ rc3 " _rc
heckman y x1 x2, select(s = z1 z2 x1) twostep
shwt
display "@ m6 " (e(method) == "two-step") " " (e(rhometh) == "rhosigma") " " (e(vce) == "conventional") " " (e(depvar) == "y s")
heckman y x1 x2, select(z1 z2 x1) twostep
shwt
heckman y x1 x2, select(z1 z2 x1) twostep level(90)
heckman y x1 x2, select(z1 z2 x1)
heckman
heckman, level(80)
display "@ lv " _b[x1] " " _se[x1] " " _b[_cons]
heckman y x1 x2, select(z1 z2 x1) nolog
predict double p0
predict double p1, xb
predict double p2, xbsel
predict double p3, psel
predict double p4, mills
predict double p5, ycond
predict double p6, yexp
foreach v in p0 p1 p2 p3 p4 p5 p6 {
    quietly summarize `v'
    display "@ pr_`v' " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
}
display "@ bx " _b[x1] " " _b[x2] " " _se[x2]
test x1 = x2
display "@ tt " %24.17g r(chi2) " " %24.17g r(p) " " r(df)
* offset() and iterate()
generate double off = mod(_n * 13, 7) / 10
heckman y x1 x2, select(s = z1 z2 x1) offset(off) nolog
shwk
display "@ o1 " (e(offset1) == "off")
predict double xbo, xb
predict double yco, ycond
foreach v in xbo yco {
    quietly summarize `v'
    display "@ po_`v' " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
}
heckman y x1 x2, select(z1 z2 x1) iterate(1)
shwk
heckman
* noconstant (twostep only: Stata's ML start with no constant is not reproduced)
heckman y x1 x2, select(s = z1 z2 x1) noconstant twostep
shwt
predict double xbn, xb
quietly summarize xbn
display "@ pn " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
