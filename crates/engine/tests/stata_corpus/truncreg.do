* RCraft's Stata corpus (P2C.12ar, batch J): made-up data; Stata 19's results are in truncreg.recorded
* commands: truncreg
* (truncreg: ll and ul, the rows dropped, Wald test, robust, clustered, fw/aw/pw/iw weights, replay, predict xb stdp)
* tolerance: 1e-5
version 19
set linesize 255
clear
set obs 240
generate double x1 = mod(_n * 37, 101) / 25 - 2
generate double x2 = mod(_n * 29, 53) / 53
generate byte id = 1 + mod(_n, 12)
generate double e = invnormal((mod(_n * 53, 997) + 0.5) / 997)
generate double ys = 1 + 0.8 * x1 - 0.5 * x2 + e
generate double fw = 1 + mod(_n, 3)
generate double pw = 0.5 + mod(_n, 4) / 2
program define shwr
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(chi2) " " %24.17g e(p) " " e(df_m) " " e(rank) " " e(N_bf) " " e(k_eq) " " e(k_aux) " " e(k_eq_model)
    display "@ sg " %24.17g e(sigma) " " e(converged)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
truncreg ys x1 x2, ll(0)
shwr
display "@ m1 " (e(cmd) == "truncreg") " " (e(predict) == "truncr_p") " " (e(title) == "Truncated regression") " " (e(depvar) == "ys") " " (e(vce) == "oim") " " (e(llopt) == "0") " " (e(chi2type) == "Wald")
truncreg ys x1 x2, ll(0) ul(2.5) nolog
shwr
truncreg ys x1 x2, ul(2) nolog
shwr
truncreg ys x1 x2 [fweight = fw], ll(0) nolog
shwr
display "@ m2 " (e(wtype) == "fweight") " " (e(wexp) == "= fw")
truncreg ys x1 x2 [pweight = pw], ll(0) nolog
shwr
display "@ m3 " (e(vcetype) == "Robust") " " (e(vce) == "robust")
truncreg ys x1 x2 [aweight = pw], ll(0) nolog
shwr
truncreg ys x1 x2 [iweight = pw], ll(0) nolog
shwr
truncreg ys x1 x2, ll(0) vce(cluster id) nolog
shwr
display "@ m4 " e(N_clust) " " (e(clustvar) == "id")
truncreg ys x1 x2 if x1 > -1.5, ll(0) level(90) nolog
shwr
truncreg ys x1 x2, ll(0)
truncreg, level(80)
display "@ lv " _b[x1] " " _se[x1] " " _b[_cons]
truncreg ys x1 x2, ll(0) nolog
predict double xb0
predict double xb1, xb
predict double sd, stdp
foreach v in xb0 xb1 sd {
    quietly summarize `v'
    display "@ pr_`v' " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
}
test x1 = x2
display "@ tt " %24.17g r(chi2) " " %24.17g r(p) " " r(df)
