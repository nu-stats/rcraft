* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in pdslasso.recorded
* commands: pdslasso
* (pdslasso, SSC, with robust: the controls selected for y and for d by the rigorous lasso, and e(b) and e(V) of OLS on their union; one with nothing selected for d)
* tolerance: 1e-7
version 19
clear
set obs 300
forvalues j = 1/12 {
  generate c`j' = sin(_n * (`j' + 0.37) * 1.713) + 0.3 * cos(_n * `j' * 0.917)
}
generate d = (sin(_n * 2.31) + 0.6 * c1 - 0.5 * c2) > 0
generate y = 0.5 * d + 0.9 * c1 + 0.6 * c3 - 0.7 * c5 + sin(_n * 7.77) * (1 + 0.5 * abs(c4))
replace y = . in 1/4
pdslasso y d (c1 c2 c3 c4 c5 c6 c7 c8 c9 c10 c11 c12), robust
display "@ a " _b[d] " " _se[d] " " e(N)
display "@ a_sel " e(xselected)
display "@ a_y " e(xselected_y)
display "@ a_d " e(xselected_dex)
generate d2 = mod(_n, 2)
pdslasso y d2 (c1 c2 c3 c4 c5 c6 c7 c8 c9 c10 c11 c12), r
display "@ b " _b[d2] " " _se[d2] " " _b[_cons]
display "@ b_d " e(xselected_dex)
