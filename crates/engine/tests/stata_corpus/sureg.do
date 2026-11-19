* RCraft's Stata corpus (P2C.12, batch E): made-up data; Stata 19's results are in sureg.recorded
* commands: sureg mvreg
* (sureg: two-step GLS, small, dfk, isure, corr, noconstant, if, fweights and aweights, equations with different regressors, e(Sigma), the chi2 and F tests of each equation, predict with equation(), test across equations; mvreg: the same data in one regressor list, corr, noheader, level(), weights, predict and test)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 90
generate double x1 = mod(_n * 37, 101) / 7
generate double x2 = mod(_n * 13, 17) / 3
generate double x3 = mod(_n * 11, 23) / 4 + x1 / 3
generate double e = mod(_n * 29, 19) / 5 - 1.8
generate double y1 = 1 + .8 * x1 - .5 * x2 + e + mod(_n * 7, 5) / 4
generate double y2 = 2 - .3 * x1 + .6 * x3 + .5 * e + mod(_n * 3, 7) / 3
generate double y3 = x1 - x2 + mod(_n * 5, 11) / 6
generate int fw = 1 + mod(_n, 3)
generate double aw = .5 + mod(_n * 7, 13) / 10
program define shwsur
    display "@ s " %24.17g e(N) " " %24.17g e(k) " " %24.17g e(rank) " " %24.17g e(ll) " " e(k_eq)
    forvalues i = 1/`=e(k_eq)' {
        display "@ q`i' " %24.17g e(rss_`i') " " %24.17g e(rmse_`i') " " %24.17g e(mss_`i') " " %24.17g e(r2_`i') " " e(df_m`i')
        capture display "@ x`i' " %24.17g e(chi2_`i') " " %24.17g e(p_`i')
        capture display "@ w`i' " %24.17g e(F_`i') " " %24.17g e(p_`i')
    }
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(e(V), 1, `j')
    }
    forvalues i = 1/`=rowsof(e(Sigma))' {
        forvalues j = 1/`=colsof(e(Sigma))' {
            display "@ g`i'_`j' " %24.17g el(e(Sigma), `i', `j')
        }
    }
end
sureg (y1 x1 x2) (y2 x1 x3)
shwsur
display "@ m1 " (e(cmd) == "sureg") " " (e(method) == "sure") " " (e(vce) == "unadjusted") " " (e(depvar) == "y1 y2") " " (e(exog) == "x1 x2 x3")
sureg (y1 x1 x2) (y2 x1 x3), corr
sureg (y1 x1 x2) (y2 x1 x3), small dfk
shwsur
display "@ m2 " e(df_r)
test [y1]x1 = [y2]x1
display "@ t1 " %24.17g r(F) " " %24.17g r(p) " " r(df) " " r(df_r)
sureg (y1 x1 x2) (y2 x1 x3) if x2 < 4, isure nolog
shwsur
predict double p1, equation(y1)
predict double r2, residuals equation(y2)
predict double p3, xb equation(#2)
summarize p1 r2 p3
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
sureg (y1 x1 x2) (y2 x1 x3) (y3 x1 x2)
shwsur
capture noisily sureg (y1 x1 x2) (y2 x1 x3) (y3 x2), corr noconstant
display "@ e3 " _rc
sureg (y1 x1 x2) (y2 x1 x3) (y3 x2), corr
shwsur
sureg (y1 x1 x2) (y2 x1 x3), small
shwsur
sureg (y1 x1 x2) (y2 x1 x3) [fweight = fw]
shwsur
sureg (y1 x1 x2) (y2 x1 x3) [aweight = aw], corr
shwsur
sureg (y1 x1 x2) (y2 x1 x3) [aweight = aw], small dfk
shwsur
sureg (y2 x1) (y3 x2) in 1/60
shwsur
capture noisily sureg (y1 x1) (y2 x1) [pweight = aw]
display "@ e1 " _rc
capture noisily predict qq, equation(zz)
display "@ e2 " _rc
* mvreg
program define shwmv
    display "@ s " %24.17g e(N) " " %24.17g e(k) " " %24.17g e(rank) " " %24.17g e(df_r) " " e(k_eq)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(e(V), 1, `j')
    }
    forvalues i = 1/`=rowsof(e(Sigma))' {
        forvalues j = 1/`=colsof(e(Sigma))' {
            display "@ g`i'_`j' " %24.17g el(e(Sigma), `i', `j')
        }
    }
end
mvreg y1 y2 = x1 x2
shwmv
display "@ n1 " (e(cmd) == "mvreg") " " (e(depvar) == "y1 y2") " " (e(eqnames) == "y1 y2")
mvreg y1 y2 y3 = x1 x2, corr
shwmv
mvreg y1 y2 = x1 x2 if x2 < 4, level(90) noheader
shwmv
test [y1]x1 = [y2]x1
display "@ t2 " %24.17g r(F) " " %24.17g r(p) " " r(df) " " r(df_r)
predict double p2, equation(y2)
predict double r1, residuals equation(y1)
summarize p2 r1
display "@ p2 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
mvreg y1 y2 = x1 x2, noconstant
shwmv
mvreg y2 y3 = x1 x2 [fweight = fw]
shwmv
mvreg y2 y3 = x1 x2 [aweight = aw], corr
shwmv
mvreg y1 y2 = x1 x2 x3, notable
shwmv
