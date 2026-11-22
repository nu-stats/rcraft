* RCraft's Stata corpus (P2C.12ar, batch M): made-up data; Stata 19's results are in boxcox.recorded
* commands: boxcox
* (boxcox: the four models, notrans(), lrtest, level(), if, e(b) and e(V), the three restricted fits, the LR tests per regressor, predict yhat, residuals and btransform, and the errors for weights and for values that are not positive)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 100
generate double x1 = sin(_n * 1.3) * 1.2 + 2.2
generate double x2 = round(0.5 + 3 * mod(_n * 7, 101) / 101, .01)
generate byte d = mod(_n * 5, 7) < 3
generate double e = sin(_n * 1.7) * .8 + cos(_n * .7) * .5
generate double y = round((1 + 0.3 * (0.8 * x1 - 0.5 * x2 + 0.4 * d + 0.5 * e) + 1.5) ^ (1 / 0.3), .01)
program define shwbx
    display "@ s " %24.17g e(N) " " %24.17g e(df_m) " " %24.17g e(df_r) " " %24.17g e(chi2) " " %24.17g e(ll) " " %24.17g e(ll0) " " e(rank) " " e(ic) " " e(rc)
    display "@ t " %24.17g e(chi2_t1) " " %24.17g round(e(p_t1), 1e-9) " " %24.17g e(ll_t1) " " %24.17g e(chi2_tm1) " " %24.17g round(e(p_tm1), 1e-9) " " %24.17g e(ll_tm1) " " %24.17g e(chi2_t0) " " %24.17g round(e(p_t0), 1e-9) " " %24.17g e(ll_t0)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
boxcox y x1 x2, nolog
shwbx
display "@ m1 " (e(cmd) == "boxcox") " " (e(model) == "lhsonly") " " (e(predict) == "boxcox_p") " " (e(depvar) == "y") " " (e(chi2type) == "LR") " " (e(ntrans) == "yes")
boxcox y x1 x2, model(rhsonly) notrans(d) nolog
shwbx
boxcox y x1 x2, model(lambda) lrtest nolog
shwbx
forvalues j = 1/2 {
    display "@ l`j' " %24.17g el(e(chi2m), 1, `j') " " %24.17g round(el(e(pm), 1, `j'), 1e-9) " " el(e(df), 1, `j')
}
boxcox y x1 x2 if d == 0, model(theta) level(90) nolog
shwbx
boxcox y x1 x2, model(theta) notrans(d) lrtest nolog
shwbx
boxcox y x1 x2, nolog
predict double yh
predict double r, residuals
predict double yb, btransform
summarize yh r yb
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
