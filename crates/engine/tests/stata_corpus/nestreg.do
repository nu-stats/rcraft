* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in nestreg.recorded
* nestreg with regress: the Wald F test of each block, R2, and what is left in r(wald) and e()
* commands: nestreg
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 200
generate double x1 = round(sin(_n * 1.13) + 0.3 * cos(_n * 0.71), .001)
generate double x2 = round(cos(_n * 2.07) + 0.2 * sin(_n * 0.37), .001)
generate double x3 = round(sin(_n * 3.31 + 1), .001)
generate double x4 = round(cos(_n * 4.17 + 2) + 0.4 * sin(_n * 0.91), .001)
generate double e = sin(_n * 7.7) + 0.5 * cos(_n * 1.93)
generate double y = 1 + .6*x1 + .35*x2 + .12*x3 + e
generate int g = ceil(_n / 20)
local tag = "plain"
quietly nestreg: regress y (x1) (x2 x3) (x4)
forvalues i = 1/3 {
    display "@ `tag'_w`i' " %20.0g el(r(wald), `i', 1) " " %20.0g el(r(wald), `i', 2) " " %20.0g el(r(wald), `i', 3) " " %20.0g el(r(wald), `i', 4) " " %20.0g el(r(wald), `i', 5)
}
forvalues i = 2/3 {
    display "@ `tag'_d`i' " %20.0g el(r(wald), `i', 6)
}
display "@ `tag'_e " e(N) " " e(df_m) " " e(df_r) " " %20.0g e(r2) " " %20.0g el(e(b), 1, 1)
local tag = "robust"
nestreg, quietly: regress y (x1) (x2 x3) (x4), vce(robust)
forvalues i = 1/3 {
    display "@ `tag'_w`i' " %20.0g el(r(wald), `i', 1) " " %20.0g el(r(wald), `i', 2) " " %20.0g el(r(wald), `i', 3) " " %20.0g el(r(wald), `i', 4) " " %20.0g el(r(wald), `i', 5)
}
display "@ `tag'_e " e(N) " " e(df_r) " " %20.0g el(e(V), 1, 1)
local tag = "cluster"
nestreg, quietly: regress y (x1) (x2 x3) (x4), vce(cluster g)
forvalues i = 1/3 {
    display "@ `tag'_w`i' " %20.0g el(r(wald), `i', 1) " " %20.0g el(r(wald), `i', 2) " " %20.0g el(r(wald), `i', 3) " " %20.0g el(r(wald), `i', 4) " " %20.0g el(r(wald), `i', 5)
}
display "@ `tag'_e " e(N) " " e(df_r) " " e(N_clust) " " %20.0g el(e(V), 1, 1)
local tag = "cond"
nestreg, quietly: regress y (x1) (x2 x3) if x4 > -1
forvalues i = 1/2 {
    display "@ `tag'_w`i' " %20.0g el(r(wald), `i', 1) " " %20.0g el(r(wald), `i', 2) " " %20.0g el(r(wald), `i', 3) " " %20.0g el(r(wald), `i', 4) " " %20.0g el(r(wald), `i', 5)
}
display "@ `tag'_e " e(N) " " e(df_r) " " %20.0g e(r2)
local tag = "inrange"
quietly nestreg: regress y x1 (x2 x3) in 1/100, level(90)
forvalues i = 1/2 {
    display "@ `tag'_w`i' " %20.0g el(r(wald), `i', 1) " " %20.0g el(r(wald), `i', 2) " " %20.0g el(r(wald), `i', 3) " " %20.0g el(r(wald), `i', 4) " " %20.0g el(r(wald), `i', 5)
}
display "@ `tag'_e " e(N) " " e(df_r) " " %20.0g e(r2)
local tag = "oneblock"
nestreg, quietly: regress y (x1 x2)
display "@ `tag'_w1 " %20.0g el(r(wald), 1, 1) " " %20.0g el(r(wald), 1, 2) " " %20.0g el(r(wald), 1, 3) " " %20.0g el(r(wald), 1, 4) " " %20.0g el(r(wald), 1, 5)
local tag = "stored"
nestreg, quietly store(m): regress y (x1) (x2 x3)
estimates restore m1
display "@ `tag'_m1 " e(N) " " e(df_m) " " %20.0g e(r2) " " %20.0g el(e(b), 1, 1)
estimates restore m2
display "@ `tag'_m2 " e(N) " " e(df_m) " " %20.0g e(r2) " " %20.0g el(e(b), 1, 1)

