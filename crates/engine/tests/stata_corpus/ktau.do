* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in ktau.recorded
* commands: ktau
* (Kendall's tau-a and tau-b, the score and its variance corrected for ties, the continuity-corrected p; the matrix of three or more, listwise or pw, with Bonferroni's or Sidak's adjustment)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 40
generate double x1 = mod(_n * 37, 101) / 7
generate double x2 = mod(_n * 13, 17) / 3 + x1 / 3
generate double x3 = mod(_n * 11, 23) / 4
generate double x4 = -x1 + mod(_n, 3)
generate int g = 1 + mod(_n, 4)
generate int w = 1 + mod(_n, 3)
quietly ktau x1 x2
display "@ pair " %24.17g r(tau_a) " " %24.17g r(tau_b) " " r(score) " " %24.17g r(se_score) " " %24.17g r(p) " " r(N)
quietly ktau g w
display "@ ties " %24.17g r(tau_b) " " %24.17g r(se_score) " " %24.17g r(p)
quietly ktau x1 x4
display "@ near " %24.17g r(tau_a) " " %24.17g r(tau_b) " " %24.17g r(se_score)
quietly ktau x1 x2 x3 x4
display "@ mat " %24.17g r(tau_a) " " %24.17g el(r(Tau_b), 4, 4) " " %24.17g el(r(Tau_a), 3, 3) " " %24.17g el(r(Se_Score), 4, 1) " " %24.17g el(r(P), 3, 2)
quietly ktau x1 x2 x3, sidak
display "@ sidak " %24.17g el(r(P), 2, 1) " " %24.17g el(r(P), 3, 2)
replace x4 = . in 1/3
quietly ktau x1 x2 x3 x4, pw bonferroni
display "@ pw " r(N) " " %24.17g r(tau_b) " " %24.17g el(r(P), 3, 1) " " el(r(Nobs), 4, 1) " " el(r(Nobs), 2, 1)
capture ktau x1
display "@ rc " _rc
