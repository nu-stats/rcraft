* RCraft's Stata corpus (P2C.12ba, batch M): made-up data; Stata 19's results are in betareg.recorded
* commands: betareg
* (betareg: the four links, the three scale links, scale(), vce(robust), vce(cluster), level(), if, fweights, pweights, iweights, the LR and Wald tests, e(b), e(V), predict cmean, cvariance, xb, xbscale and stdp, and the stop for aweights)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 120
generate double x1 = round(sin(_n * 1.3) * 1.2, .01)
generate double x2 = round(2 * mod(_n * 13, 101) / 101, .01)
generate double mu = invlogit(-0.3 + 0.6 * x1 - 0.4 * x2)
generate double y = invlogit(logit(mu) + 0.35 * sin(_n * 2.3) + 0.2 * cos(_n * 0.7))
replace y = min(max(round(y, .0001), .0001), .9999)
generate int fw = 1 + mod(_n * 5, 3)
generate double pw = round(0.5 + 2 * mod(_n * 7, 11) / 11, .01)
generate byte cl = 1 + mod(_n, 10)
program define shwb
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(chi2) " " %24.17g round(e(p), 1e-9) " " e(df_m) " " e(rank) " " e(ic) " " e(converged)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
betareg y x1 x2, nolog
shwb
display "@ m1 " (e(cmd) == "betareg") " " (e(vce) == "oim") " " (e(depvar) == "y") " " (e(linkt) == "Logit") " " (e(slinkt) == "Log") " " (e(chi2type) == "LR") " " (e(title) == "Beta regression")
display "@ ll0 " %24.17g e(ll_0)
betareg y x1 x2, scale(x2) nolog
shwb
betareg y x1 x2, link(probit) slink(root) nolog
shwb
betareg y x1, link(cloglog) vce(robust) nolog
shwb
betareg y x1 x2 if x2 < 1.5, link(loglog) slink(identity) level(90) nolog
shwb
betareg y x1 x2, scale(x1 x2) nolog
shwb
betareg y x1 [fw = fw], nolog
shwb
betareg y x1 [pw = pw], nolog
shwb
display "@ w1 " (e(wtype) == "pweight") " " (e(wexp) == "pw") " " (e(vce) == "robust")
betareg y x1 [iw = pw], nolog
shwb
betareg y x1 x2, vce(cluster cl) nolog
shwb
display "@ c1 " e(N_clust) " " (e(clustvar) == "cl")
betareg y x1 x2, scale(x2) nolog
predict double m
predict double v, cvariance
predict double xb, xb
predict double xs, xbscale
predict double sp, stdp
foreach s in m v xb xs sp {
    quietly summarize `s'
    display "@ p`s' " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
}
capture betareg y x1 [aw = pw]
display "@ r1 " _rc
