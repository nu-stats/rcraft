* RCraft's Stata corpus (P2C.12ar, batch J): made-up data; Stata 19's results are in hetprobit.recorded
* commands: hetprobit
* (hetprobit: the probit start, LR or Wald test of lnsigma, robust, clustered, fw/pw/iw weights, replay, predict pr xb stdp sigma)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 240
generate double x1 = mod(_n * 37, 101) / 25 - 2
generate double x2 = mod(_n * 29, 53) / 53
generate double z = mod(_n * 17, 61) / 61
generate double z2 = mod(_n * 11, 43) / 43
generate byte id = 1 + mod(_n, 12)
generate double e = invnormal((mod(_n * 53, 997) + 0.5) / 997)
generate byte yb = (0.3 + 0.7 * x1 - 0.6 * x2 + exp(0.8 * z) * e) > 0
generate double fw = 1 + mod(_n, 3)
generate double pw = 0.5 + mod(_n, 4) / 2
program define shwh
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(chi2) " " %24.17g e(p) " " e(df_m) " " e(rank) " " e(k) " " e(k_eq) " " e(k_dv) " " e(k_eq_model)
    display "@ t " %24.17g e(ll_c) " " %24.17g e(chi2_c) " " e(df_m_c) " " %24.17g e(p_c) " " e(N_f) " " e(N_s) " " e(converged) " " e(ic)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
hetprobit yb x1 x2, het(z)
shwh
display "@ m1 " (e(cmd) == "hetprobit") " " (e(predict) == "hetprobit_p") " " (e(vce) == "oim") " " (e(chi2type) == "Wald") " " (e(chi2_ct) == "LR") " " (e(depvar) == "yb")
hetprobit yb x1 x2, het(z) nolog
shwh
hetprobit yb x1 x2, het(z z2) nolog
shwh
hetprobit yb x1 x2, het(z) vce(robust) nolog
shwh
display "@ m2 " (e(vcetype) == "Robust") " " (e(chi2_ct) == "Wald")
hetprobit yb x1 x2, het(z) vce(cluster id) nolog
shwh
display "@ m3 " e(N_clust) " " (e(clustvar) == "id")
hetprobit yb x1 x2, het(z) level(90) nolog
shwh
hetprobit yb x1 x2 [fweight = fw], het(z) nolog
shwh
display "@ m4 " (e(wtype) == "fweight") " " (e(wexp) == "= fw")
hetprobit yb x1 x2 [pweight = pw], het(z) nolog
shwh
hetprobit yb x1 x2 [iweight = pw], het(z) nolog
shwh
capture noisily hetprobit yb x1 x2 [aweight = pw], het(z)
display "@ rc " _rc
capture noisily hetprobit yb x1 x2
display "@ rc2 " _rc
hetprobit yb x1 x2, het(z)
hetprobit
hetprobit, level(80)
display "@ lv " _b[x1] " " _se[x1] " " _b[_cons]
hetprobit yb x1 x2, het(z) nolog
predict double p0
predict double p1, pr
predict double xb1, xb
predict double sd, stdp
predict double sg, sigma
foreach v in p0 p1 xb1 sd sg {
    quietly summarize `v'
    display "@ pr_`v' " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
}
display "@ bx " _b[x1] " " _b[x2] " " _se[x2]
test x1 = x2
display "@ tt " %24.17g r(chi2) " " %24.17g r(p) " " r(df)
