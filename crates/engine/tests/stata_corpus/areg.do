* RCraft's Stata corpus (P2C.12, batch E): made-up data; Stata 19's results are in areg.recorded
* commands: areg
* (areg: absorbing the indicators of one variable; the fit with plain and factor regressors, robust and clustered errors, fweights, aweights and pweights, if, level(), the F test of the absorbed indicators, e() scalars, e(b) and e(V), predict xb, residuals, d and xbd, test after it, regressors omitted for collinearity, interactions and ib. bases, and the errors with their return codes)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 240
generate int id = ceil(_n / 6)
generate int g = 1 + mod(_n * 7, 3)
generate double x = mod(_n * 37, 101) / 20
generate double z = mod(_n * 13, 17) / 9
generate double u = mod(id * 29, 19) / 6
generate double y = 1 + .5 * x - .4 * z + u + mod(_n * 11, 23) / 7
generate double w = .5 + mod(_n * 3, 11) / 10
generate int fw = 1 + mod(_n * 5, 3)
program define shwareg
    display "@ s " %24.17g e(N) " " %24.17g e(sum_w) " " %24.17g e(df_m) " " %24.17g e(df_r) " " %24.17g e(rank) " " e(k_absorb)
    display "@ f " %24.17g e(F) " " %24.17g e(p) " " %24.17g e(r2) " " %24.17g e(r2_a) " " %24.17g e(rmse)
    display "@ l " %24.17g e(ll) " " %24.17g e(ll_0) " " %24.17g e(tss) " " %24.17g e(rss) " " %24.17g e(mss)
    capture display "@ a " %24.17g e(F_absorb) " " %24.17g e(p_absorb) " " e(df_a)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(e(V), 1, `j')
    }
end
areg y x z, absorb(id)
shwareg
display "@ m1 " (e(cmd) == "areg") " " (e(absvar) == "id") " " (e(vce) == "ols") " " (e(depvar) == "y") " " (e(title) == "Linear regression, absorbing indicators")
areg y x z, absorb(id) vce(robust)
shwareg
areg y x z, absorb(id) vce(cluster id)
shwareg
display "@ m2 " e(N_clust)
areg y x z, absorb(id) vce(cluster g)
shwareg
areg y x z [aw = w], absorb(id)
shwareg
areg y x z [fw = fw], absorb(id)
shwareg
areg y x z [pw = w], absorb(id)
shwareg
areg y i.g x, absorb(id)
shwareg
areg y x z if x < 4, absorb(id) level(90)
shwareg
areg y x, absorb(g)
shwareg
areg y x z, absorb(id) robust
shwareg
predict double yh
predict double dd, d
predict double rr, residuals
predict double xbd, xbd
summarize yh dd rr xbd
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
areg y x z, absorb(id)
predict double yh2
predict double dd2, d
display "@ p2 " %24.17g yh2[3] " " %24.17g dd2[3] " " %24.17g yh2[100] " " %24.17g dd2[100]
predict double rr2, residuals
display "@ p3 " %24.17g rr2[3] " " %24.17g rr2[100]
test x
display "@ t1 " %24.17g r(F) " " %24.17g r(p) " " r(df) " " r(df_r)
test x = z
display "@ t2 " %24.17g r(F) " " %24.17g r(p) " " r(df) " " r(df_r)
capture noisily areg y x z
display "@ e1 " _rc
capture noisily areg y x z [iw = w], absorb(id)
display "@ e2 " _rc
capture noisily areg y x z if x > 100, absorb(id)
display "@ e3 " _rc
* regressors the absorbed groups or the others make collinear: omitted with a note, an o. column in e(b)
generate double x2 = 2 * x
generate double xz = x + z
generate double cst = 3
areg y x u z, absorb(id)
shwareg
areg y u x z, absorb(id)
shwareg
areg y x x2 z, absorb(id)
shwareg
areg y x z xz, absorb(id)
shwareg
areg y x cst z, absorb(id)
shwareg
areg y x u, absorb(id) vce(robust)
shwareg
areg y x u, absorb(id) vce(cluster id)
shwareg
areg y x i.g u [fweight = fw], absorb(id)
shwareg
predict double pc1
predict double rc1, residuals
predict double dc1, d
summarize pc1
display "@ pc1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
summarize rc1
display "@ rc1 " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
summarize dc1
display "@ dc1 " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
test x
display "@ tc1 " %24.17g r(F) " " r(df) " " r(df_r) " " %24.17g r(p)
* interactions and ib. bases: e(b) with Stata's base-level columns, the table as Stata lays it out
generate byte hb = mod(_n * 5, 2)
areg y x ib2.g, absorb(id)
shwareg
areg y x i.g#c.z, absorb(id)
shwareg
areg y x c.z##i.hb, absorb(id)
shwareg
areg y x i.g#i.hb, absorb(id)
shwareg
areg y x c.x#c.z, absorb(id)
shwareg
areg y x i.g##c.z, absorb(id) vce(robust)
shwareg
areg y x c.z##i.hb [fweight = fw], absorb(id)
shwareg
predict double pi1
predict double ri1, residuals
summarize pi1
display "@ pi1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
summarize ri1
display "@ ri1 " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
test 1.hb
display "@ ti1 " %24.17g r(F) " " r(df) " " r(df_r) " " %24.17g r(p)
