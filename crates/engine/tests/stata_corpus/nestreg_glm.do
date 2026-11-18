* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in nestreg_glm.recorded
* Stata stops iterating at its tolerance, so the likelihood models agree to about 7 digits, and a Wald p-value (which grows with chi2) to about 6
* commands: nestreg
* (nestreg with logit, probit and poisson: the Wald chi2 table, the likelihood-ratio table, and both)
* tolerance: 1e-5
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
generate byte z = (y + 0.8*cos(_n * 9.1) > 1)
generate int cnt = floor(exp(0.8 + 0.5 * x1 - 0.4 * x2 + 0.3 * x3) * (1.2 + 0.8 * sin(_n * 3.7)))
local tag = "logit_w"
quietly nestreg: logit z (x1) (x2 x3) (x4)
forvalues i = 1/3 {
    display "@ `tag'`i' " %20.0g el(r(wald), `i', 1) " " %20.0g el(r(wald), `i', 2) " " %20.0g el(r(wald), `i', 3)
}
display "@ `tag'_e " e(N) " " e(df_m) " " %20.0g e(ll) " " %20.0g e(ll_0)
local tag = "logit_lr"
nestreg, quietly lr: logit z (x1) (x2 x3) (x4)
forvalues i = 1/3 {
    display "@ `tag'`i' " %20.0g el(r(lr), `i', 1) " " %20.0g el(r(lr), `i', 2) " " %20.0g el(r(lr), `i', 3) " " %20.0g el(r(lr), `i', 4) " " %20.0g el(r(lr), `i', 5) " " %20.0g el(r(lr), `i', 6)
}
local tag = "logit_both"
nestreg, quietly lr waldtable: logit z (x1) (x2 x3)
forvalues i = 1/2 {
    display "@ `tag'_w`i' " %20.0g el(r(wald), `i', 1) " " %20.0g el(r(wald), `i', 2) " " %20.0g el(r(wald), `i', 3)
    display "@ `tag'_l`i' " %20.0g el(r(lr), `i', 1) " " %20.0g el(r(lr), `i', 2) " " %20.0g el(r(lr), `i', 3) " " %20.0g el(r(lr), `i', 4) " " %20.0g el(r(lr), `i', 5) " " %20.0g el(r(lr), `i', 6)
}
local tag = "probit_w"
nestreg, quietly: probit z (x1 x2) (x3 x4)
forvalues i = 1/2 {
    display "@ `tag'`i' " %20.0g el(r(wald), `i', 1) " " %20.0g el(r(wald), `i', 2) " " %20.0g el(r(wald), `i', 3)
}
display "@ `tag'_e " e(N) " " e(df_m) " " %20.0g e(ll)
local tag = "probit_lr"
nestreg, quietly lrtable: probit z (x1 x2) (x3 x4)
forvalues i = 1/2 {
    display "@ `tag'`i' " %20.0g el(r(lr), `i', 1) " " %20.0g el(r(lr), `i', 2) " " %20.0g el(r(lr), `i', 3) " " %20.0g el(r(lr), `i', 4) " " %20.0g el(r(lr), `i', 5) " " %20.0g el(r(lr), `i', 6)
}
local tag = "pois_w"
nestreg, quietly: poisson cnt (x1) (x2 x3)
forvalues i = 1/2 {
    display "@ `tag'`i' " %20.0g el(r(wald), `i', 1) " " %20.0g el(r(wald), `i', 2) " " %20.0g el(r(wald), `i', 3)
}
local tag = "pois_lr"
nestreg, quietly lr: poisson cnt (x1) (x2 x3)
forvalues i = 1/2 {
    display "@ `tag'`i' " %20.0g el(r(lr), `i', 1) " " %20.0g el(r(lr), `i', 2) " " %20.0g el(r(lr), `i', 3) " " %20.0g el(r(lr), `i', 4) " " %20.0g el(r(lr), `i', 5) " " %20.0g el(r(lr), `i', 6)
}
local tag = "logit_if"
nestreg, quietly lr waldtable: logit z (x1) (x2 x3) if x4 > -1
display "@ `tag'_w2 " %20.0g el(r(wald), 2, 1) " " %20.0g el(r(wald), 2, 2) " " %20.0g el(r(wald), 2, 3)
display "@ `tag'_l2 " %20.0g el(r(lr), 2, 1) " " %20.0g el(r(lr), 2, 2) " " %20.0g el(r(lr), 2, 4)
display "@ `tag'_e " e(N) " " e(df_m)
