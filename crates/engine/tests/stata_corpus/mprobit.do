* RCraft's Stata corpus (batch MV): made-up data; Stata 19's results are in mprobit.recorded
* commands: mprobit predict summarize
* (the multinomial probit: default and chosen base outcome, no log, probitparam, e() scalars and matrices, predicted probabilities)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 80
generate double x1 = mod(_n * 37, 101) / 50
generate double x2 = mod(_n * 13, 17) / 8
generate byte y = 1 + mod(_n * 11, 3)
replace y = 3 if x1 > 1.6 & mod(_n, 2)
label define yl 1 "low" 2 "mid" 3 "high"
label values y yl

mprobit y x1 x2
display "@ a_sc " e(N) " " e(k) " " e(rank) " " e(df_m) " " e(k_out) " " e(k_eq) " " e(k_indvars) " " e(baseout) " " e(ibaseout) " " e(k_points) " " e(probitparam)
display "@ a_ll " %17.0g e(ll) " " %17.0g e(chi2) " " %17.0g e(p)
display "@ a_txt " ("`e(depvar)'" == "y") ("`e(indvars)'" == "x1 x2") ("`e(outeqs)'" == "low mid high") ("`e(out3)'" == "high") ("`e(cmd)'" == "mprobit")
matrix b = e(b)
local s ""
forvalues j = 1/9 {
    local s "`s' `=string(b[1,`j'], "%17.0g")'"
}
display "@ a_b`s'"
matrix V = e(V)
forvalues i = 1/9 {
    local s ""
    forvalues j = 1/9 {
        local s "`s' `=string(V[`i',`j'], "%17.0g")'"
    }
    display "@ a_V `i'`s'"
}
mprobit y x1 x2, baseoutcome(3)
display "@ b_ll " %17.0g e(ll) " " %17.0g e(chi2) " " e(ibaseout)
mprobit y x1 x2, baseoutcome(1)
display "@ c_ll " %17.0g e(ll) " " %17.0g e(chi2) " " e(ibaseout) " " e(k)
matrix b = e(b)
local s ""
forvalues j = 1/9 {
    local s "`s' `=string(b[1,`j'], "%17.0g")'"
}
display "@ c_b`s'"
mprobit y x1, nolog
display "@ d_ll " %17.0g e(ll) " " %17.0g e(chi2) " " %17.0g e(p) " " e(df_m)
predict p1 p2 p3
summarize p1 p2 p3
display "@ d_p " %17.0g r(mean) " " %17.0g r(sd) " " %17.0g r(min) " " %17.0g r(max)
forvalues i = 1/3 {
    summarize p`i'
    display "@ d_p`i' " %17.0g r(mean) " " %17.0g r(sd) " " %17.0g r(min) " " %17.0g r(max)
}
mprobit y x1 x2, probitparam
display "@ e_ll " %17.0g e(ll) " " %17.0g e(chi2) " " %17.0g e(p) " " e(probitparam)
matrix b = e(b)
matrix V = e(V)
local s ""
forvalues j = 1/6 {
    local s "`s' `=string(b[1,`j'], "%17.0g")' `=string(V[`j',`j'], "%17.0g")'"
}
display "@ e_b`s'"
