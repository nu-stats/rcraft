* RCraft's Stata corpus (P2C.12ba, batch M): made-up data; Stata 19's results are in zcount.recorded
* commands: zip zinb tpoisson tnbreg ztp ztnb
* (zero-inflated and zero-truncated counts: inflate(), irr, level(), if, fweights, pweights, iweights, vce(cluster), tolerance options, the LR and Wald tests, e(b), e(V), e(N_zero), predict n and xb, and the stops for aweights and a zero count)
* tolerance: 1e-7
version 19
set linesize 255
clear
set obs 300
generate double x1 = sin(_n * 1.3) * 1.2
generate double x2 = mod(_n * 7, 101) / 50
generate double z = cos(_n * .9)
generate double lambda = exp(0.4 + 0.5 * x1 - 0.3 * x2 + 0.5 * sin(_n * 1.7))
generate int count = floor(lambda * (1 + mod(_n * 13, 17) / 6) * (mod(_n * 5, 7) / 3))
replace count = 0 if mod(_n * 11, 5) < 2 & z > -0.5
generate int pos = count if count > 0
generate int fw = 1 + mod(_n * 5, 3)
generate double pw = round(0.5 + 2 * mod(_n * 7, 11) / 11, .01)
generate byte cl = 1 + mod(_n, 10)
program define shwz
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(ll_0) " " %24.17g e(chi2) " " %24.17g round(e(p), 1e-9) " " e(df_m) " " e(rank) " " e(converged) " " e(k_eq)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
zip count x1 x2, inflate(z) nolog
shwz
display "@ z1 " e(N_zero) " " (e(cmd) == "zip") " " (e(inflate) == "logit") " " (e(predict) == "zip_p") " " (e(chi2type) == "LR") " " (e(vce) == "oim")
zip count x1 x2, inflate(z x1) irr nolog
shwz
zip count x1 x2 if x2 > .3, inflate(z) level(90) nolog
shwz
zip count x1 x2 [fw = fw], inflate(x2) nolog
shwz
zip count x1 x2 [pw = pw], inflate(x2) nolog
shwz
zip count x1 x2 [iw = pw], inflate(x2) nolog
shwz
zip count x1 x2, inflate(z) vce(cluster cl) nolog
shwz
display "@ z2 " e(N_clust) " " (e(vce) == "cluster") " " (e(chi2type) == "Wald")
zinb count x1 x2, inflate(z) nolog
shwz
display "@ z3 " e(N_zero) " " e(k_aux) " " (e(cmd) == "zinb")
zinb count x1 x2 [fw = fw], inflate(x2) nolog
shwz
tpoisson pos x1 x2, nolog
shwz
display "@ t1 " %24.17g e(r2_p) " " (e(cmd) == "tpoisson") " " (e(predict) == "tpoisson_p") " " (e(llopt) == "0")
tpoisson pos x1 x2 [fw = fw], nolog
shwz
tpoisson pos x1 x2 [pw = pw], nolog
shwz
tnbreg pos x1 x2, nolog tolerance(1e-10) ltolerance(1e-12) nrtolerance(1e-12)
shwz
display "@ n1 " %24.17g e(r2_p) " " %24.17g e(alpha) " " %24.17g e(ll_c) " " %24.17g e(chi2_c)
predict double nhat, n
predict double xb, xb
summarize nhat xb
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
ztp pos x1 x2, nolog
shwz
ztnb pos x1 x2, nolog
shwz
display "@ n2 " %24.17g e(r2_p) " " %24.17g e(alpha) " " %24.17g e(ll_c) " " %24.17g e(chi2_c) " " e(rank0) " " (e(cmd) == "ztnb")
capture zip count x1 [aw = pw], inflate(x1)
display "@ r1 " _rc
capture ztnb count x1
display "@ r2 " _rc
