* RCraft's Stata corpus (P2C.12, batch E): made-up data; Stata 19's results are in qreg.recorded
* commands: qreg
* (qreg with nolog and continuous regressors, where the minimum is unique: the median and other quantiles, quantile(75) for .75, one and several regressors, fweights, if, level(), the raw and minimum sums of deviations, the pseudo R2, the sparsity and bandwidth of the iid variance, e(b), e(V), predict xb and residuals, and the error for aweights)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 80
generate double x1 = sin(_n * 1.3) * 1.5 + mod(_n, 7) / 5
generate double x3 = cos(_n * .9) + mod(_n * 3, 11) / 4
generate double e = sin(_n * 1.7) * 1.3 + cos(_n * .7)
generate double y = 2 + 1.2 * x1 - .8 * x3 + (1 + .3 * x3) * e
generate int fw = 1 + mod(_n * 5, 3)
program define shwq
    display "@ s " %24.17g e(N) " " %24.17g e(sum_w) " " %24.17g e(df_m) " " %24.17g e(df_r) " " %24.17g e(rank) " " %24.17g e(q)
    display "@ d " %24.17g e(q_v) " " %24.17g e(sum_rdev) " " %24.17g e(sum_adev) " " %24.17g e(r2_p)
    display "@ v " %24.17g e(sparsity) " " %24.17g e(bwidth) " " %24.17g e(f_r) " " e(convcode)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(e(V), 1, `j')
    }
end
qreg y x1 x3, nolog
shwq
display "@ m1 " (e(cmd) == "qreg") " " (e(vce) == "iid") " " (e(denmethod) == "fitted") " " (e(bwmethod) == "hsheather") " " (e(depvar) == "y")
qreg y x1 x3, quantile(.25) nolog
shwq
qreg y x1 x3, quantile(75) nolog
shwq
qreg y x1, nolog
shwq
qreg y x1 x3, quantile(.1) nolog level(90)
shwq
qreg y x1 x3 [fweight = fw], nolog
shwq
qreg y x1 x3 [fweight = fw], quantile(.25) nolog
shwq
qreg y x1 x3 if x1 > -1, nolog
shwq
qreg y x1 x3, nolog
predict double xb, xb
predict double r, residuals
summarize xb r
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
display "@ p2 " %24.17g xb[3] " " %24.17g r[3] " " %24.17g xb[40] " " %24.17g r[40]
capture noisily qreg y x1 x3 [aweight = fw], nolog
display "@ e1 " _rc
capture noisily qreg y x1 x3 if x1 > 100, nolog
display "@ e2 " _rc
