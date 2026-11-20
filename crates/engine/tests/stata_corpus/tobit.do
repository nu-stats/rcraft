* RCraft's Stata corpus (P2C.12ar, batch J): made-up data; Stata 19's results are in tobit.recorded
* commands: tobit
* (tobit: ll and ul, censoring counts, LR or F test, robust, clustered, weights, factor variables, replay, predict xb stdp pr e ystar)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 120
generate double x1 = mod(_n * 37, 101) / 25 - 2
generate byte x2 = mod(_n * 7, 5) < 2
generate byte g = 1 + mod(_n, 3)
generate byte id = 1 + mod(_n, 8)
generate double e = invnormal((mod(_n * 53, 997) + 0.5) / 997)
generate double ystar = 1 + 0.8 * x1 - 0.5 * x2 + e
generate double y = max(ystar, 0)
generate double yu = min(ystar, 2)
generate double yb = max(min(ystar, 2), 0)
generate double fw = 1 + mod(_n, 3)
generate double pw = 0.5 + mod(_n, 4) / 2
program define shwt
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(ll_0) " " %24.17g e(chi2) " " %24.17g e(F) " " %24.17g e(p) " " %24.17g e(r2_p) " " e(df_m) " " e(df_r) " " e(rank) " " e(k)
    display "@ c " e(N_unc) " " e(N_lc) " " e(N_rc) " " e(converged) " " e(ic) " " e(k_aux) " " e(k_dv) " " e(k_eq)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
tobit y x1 x2, ll(0)
shwt
display "@ m1 " (e(cmd) == "tobit") " " (e(predict) == "tobit_p") " " (e(title) == "Tobit regression") " " (e(depvar) == "y") " " (e(vce) == "oim") " " (e(llopt) == "0") " " (e(chi2type) == "LR")
tobit y x1 x2, ll
shwt
tobit yu x1 x2, ul(2) nolog
shwt
tobit yb x1 x2, ll(0) ul(2)
shwt
tobit y x1 x2, ll(0) vce(robust) nolog
shwt
display "@ m2 " (e(vcetype) == "Robust") " " (e(vce) == "robust")
tobit y x1 x2, ll(0) vce(cluster id) nolog
shwt
display "@ m3 " e(N_clust) " " (e(clustvar) == "id") " " (e(vce) == "cluster")
tobit y x1 x2 if x1 > -1, ll(0) level(90) nolog
shwt
tobit y x1 x2 [fweight = fw], ll(0) nolog
shwt
display "@ m4 " (e(wtype) == "fweight") " " (e(wexp) == "= fw")
tobit y x1 x2 [pweight = pw], ll(0) nolog
shwt
tobit y x1 x2 [iweight = pw], ll(0)
shwt
tobit y x1 x2 [pweight = pw], ll(0) vce(cluster id) nolog
shwt
tobit y i.g x1, ll(0) nolog
shwt
tobit y x1 x2, ll(0.4) nolog
shwt
tobit y x1 x2, ll(0)
tobit, level(80)
display "@ lv " _b[x1] " " _se[x1] " " _b[_cons] " " _b[/var(e.y)]
tobit y x1 x2, ll(0) nolog
predict double xb0
predict double xb1, xb
predict double sd, stdp
predict double p1, pr(0, .)
predict double p2, pr(., 2)
predict double p3, pr(0.5, 1.5)
predict double p4, e(0, .)
predict double p5, e(., 2)
predict double p6, e(0.5, 1.5)
predict double p7, ystar(0, .)
predict double p8, ystar(0, 2)
predict double p9, ystar(0.5, 1.5)
predict double p10, pr(x1, .)
quietly summarize xb0
display "@ pr0 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
foreach v in xb1 sd p1 p2 p3 p4 p5 p6 p7 p8 p9 p10 {
    quietly summarize `v'
    display "@ pr_`v' " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
}
display "@ bx " _b[x1] " " _b[x2] " " _se[x2]
test x1 = x2
display "@ tt " %24.17g r(F) " " %24.17g r(p) " " r(df) " " r(df_r)
lincom x1 + x2
display "@ lc " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(t) " " r(df)
