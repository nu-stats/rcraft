* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in sktest.recorded
* commands: sktest
* (sktest: normal, skewed, flat and heavy-tailed data at n = 8 to 1,000, with Royston's adjustment and noadjust, frequency and analytic weights, fewer than 8 observations; r() and r(table))
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 51
set obs 1000
generate double x1 = rnormal()
generate double x2 = exp(rnormal())
generate double x3 = runiform()
generate double x4 = rnormal()^3
generate w = 1 + mod(_n, 3)
sktest x1 x2 x3 x4
display "@ all " r(N) " " %24.17g r(p_skew) " " %24.17g r(p_kurt) " " %24.17g r(chi2) " " %24.17g r(p_chi2)
foreach n in 8 9 12 20 50 100 300 {
    sktest x1 x2 x3 x4 in 1/`n'
    matrix T = r(table)
    forvalues i = 1/4 {
        display "@ t`n'_`i' " %24.17g T[`i',1] " " %24.17g T[`i',2] " " %24.17g T[`i',3] " " %24.17g T[`i',4] " " %24.17g T[`i',5]
    }
    sktest x1 x2 x3 x4 in 1/`n', noadjust
    matrix T = r(table)
    forvalues i = 1/4 {
        display "@ u`n'_`i' " %24.17g T[`i',4] " " %24.17g T[`i',5]
    }
}
sktest x1 x3 [fweight = w]
display "@ fw " r(N) " " %24.17g r(p_skew) " " %24.17g r(p_kurt) " " %24.17g r(chi2) " " %24.17g r(p_chi2)
sktest x1 x2 [aweight = w]
display "@ aw " r(N) " " %24.17g r(p_skew) " " %24.17g r(p_kurt) " " %24.17g r(chi2) " " %24.17g r(p_chi2)
sktest x1 in 1/7
display "@ seven " r(N) " " r(p_skew) " " r(chi2)
