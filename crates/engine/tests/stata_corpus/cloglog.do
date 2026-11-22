* RCraft's Stata corpus (P2C.12ar, batch J): made-up data; Stata 19's results are in cloglog.recorded
* commands: cloglog
* (cloglog: the logit-based start, the iteration count, LR or Wald test, ll_0, eform, robust, clustered, fw/pw/iw weights, replay, predict pr xb)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 160
generate double x1 = mod(_n * 37, 101) / 25 - 2
generate double x2 = mod(_n * 29, 53) / 18
generate byte id = 1 + mod(_n, 10)
generate double u = (mod(_n * 53, 997) + 0.5) / 997
generate byte y = u < 1 - exp(-exp(-0.5 + 0.8 * x1 - 0.4 * x2))
generate double fw = 1 + mod(_n, 3)
generate double pw = 0.5 + mod(_n, 4) / 2
program define shwc
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(ll_0) " " %24.17g e(chi2) " " %24.17g e(p) " " e(df_m) " " e(rank) " " e(k) " " e(k_eq) " " e(k_dv)
    display "@ c " e(N_f) " " e(N_s) " " e(converged) " " e(ic) " " e(k_eq_model)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
cloglog y x1 x2
shwc
display "@ m1 " (e(cmd) == "cloglog") " " (e(predict) == "clog_p") " " (e(vce) == "oim") " " (e(chi2type) == "LR") " " (e(depvar) == "y")
cloglog y x1 x2, nolog
shwc
cloglog y x1 x2, eform
cloglog y x1 x2 if x2 < 2, vce(robust)
shwc
display "@ m2 " (e(vcetype) == "Robust") " " (e(chi2type) == "Wald")
cloglog y x1
shwc
cloglog y x1 x2, vce(cluster id) nolog
shwc
display "@ m3 " e(N_clust) " " (e(clustvar) == "id")
cloglog y x1 x2, level(90) nolog
shwc
cloglog y x1 x2 [fweight = fw], nolog
shwc
display "@ m4 " (e(wtype) == "fweight") " " (e(wexp) == "= fw")
cloglog y x1 x2 [pweight = pw], nolog
shwc
cloglog y x1 x2 [iweight = pw], nolog
shwc
cloglog y x1 x2 [pweight = pw], vce(cluster id) nolog
shwc
capture noisily cloglog y x1 x2 [aweight = pw]
display "@ rc " _rc
cloglog y x1 x2
cloglog
cloglog, level(80)
display "@ lv " _b[x1] " " _se[x1] " " _b[_cons]
cloglog, eform
cloglog y x1 x2, nolog
predict double p0
predict double p1, pr
predict double xb1, xb
foreach v in p0 p1 xb1 {
    quietly summarize `v'
    display "@ pr_`v' " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
}
display "@ bx " _b[x1] " " _b[x2] " " _se[x2]
test x1 = x2
display "@ tt " %24.17g r(chi2) " " %24.17g r(p) " " r(df)
lincom x1 + x2
display "@ lc " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(z)
* offset() and iterate()
generate double off = mod(_n * 13, 7) / 10
cloglog y x1 x2, offset(off) nolog
shwc
display "@ o1 " (e(offset) == "off")
predict double po1, pr
predict double xbo, xb
foreach v in po1 xbo {
    quietly summarize `v'
    display "@ po_`v' " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
}
cloglog y x1 x2, iterate(1)
shwc
cloglog
* noconstant
cloglog y x1 x2, noconstant
shwc
predict double pn, pr
quietly summarize pn
display "@ pn " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
cloglog y x1 x2, noconstant vce(robust) nolog
shwc
cloglog y x1 x2, noconstant offset(off) nolog
shwc
