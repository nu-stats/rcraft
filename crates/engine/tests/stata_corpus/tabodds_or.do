* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in tabodds_or.recorded
* commands: tabodds
* (tabodds, or and base(): each level's Mantel-Haenszel odds ratio against the base level; frequency weights; Stata's errors)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 300
generate byte dose = 1 + mod(_n * 11, 4)
generate byte case = mod(_n * 13 + 3 * dose, 17) < 6
generate byte w = 1 + mod(_n, 3)
quietly tabodds case dose, or
display "@ or " %24.17g r(chi2_hom) " " %24.17g r(chi2_tr) " " r(df_hom)
quietly tabodds case dose, or base(3)
display "@ base " %24.17g r(chi2_hom) " " %24.17g r(p_trend)
quietly tabodds case dose [fw=w]
display "@ weights " %24.17g r(chi2_hom) " " %24.17g r(chi2_tr) " " %24.17g r(p_hom)
quietly tabodds case dose [fw=w], or base(2)
display "@ weights or " %24.17g r(chi2_hom) " " %24.17g r(chi2_tr)
capture tabodds case dose, base(2)
display "@ rc base without or " _rc
capture tabodds case dose, or base(5)
display "@ rc base out of range " _rc
capture tabodds case dose, or base(1.5)
display "@ rc base not whole " _rc
capture tabodds case dose [aw=w]
display "@ rc aweights " _rc
