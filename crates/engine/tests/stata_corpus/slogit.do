* RCraft's Stata corpus (batch MV): made-up data; Stata 19's results are in slogit.recorded
* commands: slogit predict summarize
* (the stereotype logistic model: one dimension with the last or a chosen base outcome, two dimensions, no log, predicted probabilities and linear indexes, the dimension limit)
* tolerance: 1e-6
version 19
set linesize 255
clear
set seed 536
set obs 300
generate double x1 = runiform() * 4
generate double x2 = rnormal()
generate double s = 0.9 * x1 - 0.7 * x2 + rnormal()
generate byte y = 1 + (s > 1) + (s > 2) + (s > 3)
label define yl 1 "poor" 2 "fair" 3 "good" 4 "best"
label values y yl
generate double u1 = -ln(-ln(runiform()))
generate double u2 = 0.5 + 0.8 * x1 - ln(-ln(runiform()))
generate double u3 = -0.5 + 1.2 * x2 - ln(-ln(runiform()))
generate double u4 = 0.3 + 0.4 * x1 + 0.9 * x2 - ln(-ln(runiform()))
generate double umax = max(u1, u2, u3, u4)
generate byte z = 1 * (u1 == umax) + 2 * (u2 == umax) + 3 * (u3 == umax) + 4 * (u4 == umax)

slogit y x1 x2
display "@ a_sc " e(N) " " e(k) " " e(k_eq) " " e(rank) " " e(df_m) " " e(k_dim) " " e(k_indvars) " " e(k_out) " " e(i_base) " " e(df_0) " " e(k_eq_model)
display "@ a_ll " %17.0g e(ll) " " %17.0g e(chi2) " " %17.0g e(p) " " %17.0g e(ll_0)
display "@ a_txt " ("`e(depvar)'" == "y") ("`e(indvars)'" == "x1 x2") ("`e(out4)'" == "best") ("`e(cmd)'" == "slogit")
matrix b = e(b)
local s ""
forvalues j = 1/8 {
    local s "`s' `=string(b[1,`j'], "%17.0g")'"
}
display "@ a_b`s'"
matrix V = e(V)
forvalues i = 1/8 {
    local s ""
    forvalues j = 1/8 {
        local s "`s' `=string(V[`i',`j'], "%17.0g")'"
    }
    display "@ a_V `i'`s'"
}
slogit y x1 x2, baseoutcome(1)
display "@ b_ll " %17.0g e(ll) " " %17.0g e(chi2) " " e(i_base)
matrix b = e(b)
local s ""
forvalues j = 1/8 {
    local s "`s' `=string(b[1,`j'], "%17.0g")'"
}
display "@ b_b`s'"
slogit z x1 x2, dimension(2)
display "@ c_sc " e(N) " " e(k) " " e(k_eq) " " e(rank) " " e(df_m) " " e(k_dim)
display "@ c_ll " %17.0g e(ll) " " %17.0g e(chi2) " " %17.0g e(p) " " %17.0g e(ll_0)
matrix b = e(b)
local s ""
forvalues j = 1/13 {
    local s "`s' `=string(b[1,`j'], "%17.0g")'"
}
display "@ c_b`s'"
matrix V = e(V)
local s ""
forvalues j = 1/13 {
    local s "`s' `=string(V[`j',`j'], "%17.0g")'"
}
display "@ c_Vd`s'"
slogit y x1, nolog
display "@ d_ll " %17.0g e(ll) " " %17.0g e(chi2) " " %17.0g e(p) " " e(df_m)
predict p1 p2 p3 p4
summarize p1 p2 p3 p4
forvalues i = 1/4 {
    summarize p`i'
    display "@ d_p`i' " %17.0g r(mean) " " %17.0g r(sd) " " %17.0g r(min) " " %17.0g r(max)
}
predict xb1, xb
summarize xb1
display "@ d_xb " %17.0g r(mean) " " %17.0g r(sd) " " %17.0g r(min) " " %17.0g r(max)
capture noisily slogit y x1 x2, dimension(4)
display "@ rc " _rc
