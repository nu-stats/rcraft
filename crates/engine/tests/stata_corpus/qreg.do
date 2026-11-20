* RCraft's Stata corpus (P2C.12, batch E): made-up data; Stata 19's results are in qreg.recorded
* commands: qreg
* (qreg with continuous regressors, where the minimum is unique: the median and other quantiles, quantile(75) for .75, one and several regressors, fweights, if, level(), the raw and minimum sums of deviations, the pseudo R2, the sparsity and bandwidth of the iid variance, e(b), e(V), predict xb and residuals, and the error for aweights; and, further down, dummies and rounded data where the minimum is not unique, so Stata's own path decides the vertex, with and without nolog, and the error for a bandwidth that leaves (0, 1))
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
* where the minimum is not unique: dummies and rounded data (Stata's own path decides the vertex)
clear
set obs 60
generate byte d = mod(_n, 3) == 0
generate double y = round(3 + 2 * d + 2 * sin(_n * 1.9) + cos(_n * .8), .5)
generate int fw = 1 + mod(_n * 5, 3)
foreach q in .25 .4 .5 .6 .75 {
    qreg y d, quantile(`q') nolog
    shwq
}
qreg y d
shwq
qreg y d [fweight = fw], quantile(.25) nolog
shwq
qreg y d [fweight = fw], nolog
shwq
qreg y d [fweight = fw], quantile(.6) nolog
shwq
clear
set obs 70
generate byte d1 = mod(_n, 2) == 0
generate byte d2 = mod(_n, 5) < 2
generate double x = sin(_n * 1.1) + mod(_n, 4) / 3
generate double y = round(1 + d1 + .5 * d2 + x + sin(_n * 2.3) * 1.5, .1)
generate int fw = 1 + mod(_n * 7, 4)
foreach q in .4 .5 .75 {
    qreg y d1 d2 x, quantile(`q') nolog
    shwq
}
qreg y d1 d2 x [fweight = fw], quantile(.5) nolog
shwq
clear
set obs 90
generate byte d = mod(_n, 4) == 0
generate double x = round(sin(_n * 1.3) * 3, .5)
generate double y = round(2 + d + .6 * x + 2 * cos(_n * 2.7), 1)
generate int fw = 1 + mod(_n * 7, 5)
foreach q in .25 .5 .9 {
    qreg y d x, quantile(`q') nolog
    shwq
}
qreg y d x [fweight = fw], quantile(.5) nolog
shwq
clear
set obs 100
generate byte a = mod(_n, 3) == 1
generate byte b = mod(_n, 3) == 2
generate double y = round(4 + 2 * a - b + 3 * sin(_n * 1.1) + cos(_n * .45), .5)
foreach q in .25 .5 .75 {
    qreg y a b, quantile(`q') nolog
    shwq
}
clear
set obs 80
generate byte d1 = mod(_n, 2) == 0
generate byte d2 = mod(_n, 3) == 0
generate byte d3 = d1 * d2
generate double y = round(5 + d1 + 2 * d2 - d3 + 2 * sin(_n * 1.9), .25)
foreach q in .4 .5 .75 {
    qreg y d1 d2 d3, quantile(`q') nolog
    shwq
}
clear
set obs 64
generate byte d = mod(_n, 4) < 1
generate double x = mod(_n * 3, 8) / 2
generate double y = round(2 + d + .5 * x + 2 * sin(_n * 1.5), .5)
foreach q in .5 .75 .9 {
    qreg y d x, quantile(`q') nolog
    shwq
}
clear
set obs 50
generate byte d1 = mod(_n, 3) == 0
generate byte d2 = mod(_n, 4) == 1
generate int y = floor(5 + 3 * d1 - 2 * d2 + 4 * sin(_n * 1.7))
foreach q in .4 .5 {
    qreg y d1 d2, quantile(`q') nolog
    shwq
}
clear
set obs 18
generate int x = mod(_n * 5, 7)
generate int y = mod(_n * 11, 9) + x
foreach q in .4 .5 .6 {
    qreg y x, quantile(`q') nolog
    shwq
}
capture noisily qreg y x, quantile(.1) nolog
display "@ e3 " _rc
