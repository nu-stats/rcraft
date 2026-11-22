* RCraft's Stata corpus (P2C.12ba, batch M): made-up data; Stata 19's results are in fracreg.recorded
* commands: fracreg
* (fracreg probit and logit: or, het(), level(), if, fweights, pweights, iweights, vce(cluster), factor variables, e(b), e(V), e(V_modelbased), e(mns), the Wald test and the pseudo R2, predict cm and xb, and the stop for aweights)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 150
generate double x1 = round(sin(_n * 1.3) * 1.5 + mod(_n, 7) / 5, .01)
generate double x2 = round(2 * mod(_n * 13, 101) / 101, .01)
generate double y = round(invlogit(-0.2 + 0.7 * x1 - 0.3 * x2 + 0.8 * sin(_n * 1.9)), .001)
replace y = 0 in 1/6
replace y = 1 in 7/10
generate int fw = 1 + mod(_n * 5, 3)
generate double pw = round(0.5 + 2 * mod(_n * 7, 11) / 11, .01)
generate byte g = 1 + mod(_n, 3)
generate byte cl = 1 + mod(_n, 10)
program define shwf
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(ll_0) " " %24.17g e(r2_p) " " %24.17g e(chi2) " " %24.17g e(p) " " e(df_m) " " e(rank) " " e(ic) " " e(converged)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(e(V_modelbased), `j', `j')
    }
end
fracreg probit y x1 x2
shwf
display "@ m1 " (e(cmd) == "fracreg") " " (e(estimator) == "probit") " " (e(vce) == "robust") " " (e(depvar) == "y") " " (e(predict) == "fracreg_p") " " (e(chi2type) == "Wald")
forvalues j = 1/3 {
    display "@ mns`j' " %24.17g el(e(mns), 1, `j')
}
fracreg logit y x1 x2, or nolog
shwf
fracreg logit y x1 if x2 < 1.5, level(90)
shwf
fracreg probit y x1 x2, het(x2) nolog
shwf
fracreg logit y x1 x2 [fw = fw], nolog
shwf
display "@ w1 " (e(wtype) == "fweight") " " (e(wexp) == "= fw")
fracreg probit y x1 x2 [pw = pw], nolog
shwf
fracreg logit y x1 x2 [iw = pw], nolog
shwf
fracreg probit y x1 x2 [pw = pw], het(x2) nolog
shwf
fracreg logit y x1 x2, vce(cluster cl) nolog
shwf
display "@ c1 " e(N_clust) " " (e(clustvar) == "cl") " " (e(vce) == "cluster")
fracreg logit y x1 i.g, nolog
shwf
fracreg probit y x1 x2, nolog
predict double m
predict double xb, xb
summarize m xb
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
summarize m
display "@ p2 " %24.17g r(mean) " " %24.17g r(sd)
capture fracreg logit y x1 x2 [aw = pw]
display "@ r1 " _rc
