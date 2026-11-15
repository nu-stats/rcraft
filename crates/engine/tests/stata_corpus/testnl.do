* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in testnl.recorded
* commands: testnl
* (Wald tests of nonlinear restrictions by the delta method after regress and logit, one or several, mtest unadjusted and Bonferroni's)
* tolerance: 1e-8
version 19
clear
set linesize 255
set obs 60
generate double z = mod(_n * 37, 101) / 7
generate double w = mod(_n * 13, 17) / 3
generate double x = 1 + .4 * z + .9 * w + mod(_n * 11, 23) / 5
generate byte b = mod(_n * 7, 5) < 2 + (z > 7)
quietly regress x z w
quietly testnl _b[z] / _b[w] = 0.1
display "@ ratio " %24.17g r(chi2) " " %24.17g r(p) " " r(df) " " %24.17g el(r(G), 1, 1) " " %24.17g el(r(G), 1, 2) " " %24.17g el(r(R), 1, 1)
quietly testnl (_b[z] * _b[w] = 0.05) (_b[_cons]^2 = 4)
display "@ two " %24.17g r(chi2) " " r(df) " " %24.17g el(r(G), 2, 3)
quietly testnl (_b[z] = 0.4) (_b[w] = 1), mtest
display "@ mtest " %24.17g r(chi2) " " %24.17g el(r(mtest), 1, 1) " " %24.17g el(r(mtest), 2, 3) " " r(mtmethod)
quietly testnl (_b[z] = 0.4) (_b[w] = 1), mtest(b)
display "@ bonf " %24.17g el(r(mtest), 1, 3) " " %24.17g el(r(mtest), 2, 4) " " r(mtmethod)
quietly logit b z w
quietly testnl exp(_b[z]) = 1.1
display "@ logit " %24.17g r(chi2) " " %24.17g r(p)
capture testnl _b[z] / _b[w]
display "@ rc " _rc
