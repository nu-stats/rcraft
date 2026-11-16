* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in tetrachoric.recorded
* commands: tetrachoric
* (maximum-likelihood tetrachoric correlations and their standard errors, Edwards and Edwards's, Fisher's exact p, listwise and pw, Bonferroni and Sidak, frequency weights; Stata's errors)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 120
generate byte b1 = mod(_n * 7, 11) < 5
generate byte b2 = (mod(_n * 7, 11) + mod(_n * 3, 5)) < 7
generate byte b3 = (mod(_n * 13, 17) + 2 * b1) > 9
generate byte b4 = mod(_n * 5, 9) < 4
generate int w = 1 + mod(_n, 3)
quietly tetrachoric b1 b2
display "@ ml " %24.17g r(rho) " " %24.17g r(se_rho) " " %24.17g r(p) " " r(N) " " r(nneg) " `r(method)'"
quietly tetrachoric b1 b2, edwards
display "@ edwards " %24.17g r(rho) " " %24.17g r(se_rho) " `r(method)'"
quietly tetrachoric b1 b2 b3 b4
display "@ matrix " %24.17g el(r(Rho), 3, 1) " " %24.17g el(r(Rho), 4, 2) " " %24.17g el(r(Se_Rho), 3, 2) " " %24.17g el(r(P), 4, 3) " " r(nneg)
quietly tetrachoric b1 b2 b3, bonferroni
display "@ bonferroni " %24.17g el(r(P), 2, 1) " " %24.17g el(r(P), 3, 2)
quietly tetrachoric b1 b2 b3, sidak
display "@ sidak " %24.17g el(r(P), 2, 1) " " %24.17g el(r(P), 3, 2)
quietly tetrachoric b1 b2 [fw = w]
display "@ fw " %24.17g r(rho) " " %24.17g r(se_rho) " " r(N)
replace b3 = . in 1/9
quietly tetrachoric b1 b2 b3
display "@ listwise " r(N) " " %24.17g el(r(Rho), 2, 1) " " el(r(Nobs), 3, 1)
quietly tetrachoric b1 b2 b3, pw
display "@ pw " r(N) " " %24.17g el(r(Rho), 2, 1) " " el(r(Nobs), 3, 1) " " %24.17g el(r(Rho), 3, 2)
generate byte b5 = 2 * b2
capture tetrachoric b1 b5
display "@ rc coded " _rc
capture tetrachoric b1
display "@ rc one " _rc
