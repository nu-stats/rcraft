* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in kap.recorded
* commands: kap
* (Cohen's kappa of two raters, unweighted and weighted by w or w2, absolute, frequency weights, if/in; Fleiss's kappa of three or more raters; Stata's errors)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 80
generate int r1 = 1 + mod(_n * 7, 3)
generate int r2 = r1
replace r2 = 1 + mod(r2, 3) if mod(_n, 6) == 0
replace r2 = 1 + mod(r2 + 1, 3) if mod(_n, 11) == 0
generate int r3 = r1
replace r3 = 1 + mod(r3, 3) if mod(_n, 5) == 0
generate int r4 = r2 + 2 * (r2 == 3)
generate int w = 1 + mod(_n, 4)
quietly kap r1 r2
display "@ two " %24.17g r(kappa) " " %24.17g r(se) " " %24.17g r(z) " " %24.17g r(prop_o) " " %24.17g r(prop_e) " " r(N)
quietly kap r1 r2, wgt(w)
display "@ w " %24.17g r(kappa) " " %24.17g r(se) " " %24.17g r(prop_e)
quietly kap r1 r2, wgt(w2)
display "@ w2 " %24.17g r(kappa) " " %24.17g r(se) " " %24.17g r(prop_o)
quietly kap r1 r4, wgt(w) absolute
display "@ absolute " %24.17g r(kappa) " " %24.17g r(se) " " %24.17g r(prop_e)
quietly kap r1 r4, wgt(w)
display "@ ranks " %24.17g r(kappa) " " %24.17g r(se)
quietly kap r1 r2 [fw = w]
display "@ fw " %24.17g r(kappa) " " %24.17g r(se) " " r(N)
quietly kap r1 r2 if w > 1
display "@ if " %24.17g r(kappa) " " r(N)
quietly kap r1 r2 r3
display "@ three " %24.17g r(kappa) " " %24.17g r(z)
quietly kap r1 r2 r3 r4
display "@ four " %24.17g r(kappa) " " %24.17g r(z)
capture kap r1
display "@ rc one " _rc
capture kap r1 r2 r3, tab
display "@ rc tab " _rc
