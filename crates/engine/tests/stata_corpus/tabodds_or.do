* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in tabodds_or.recorded
* commands: tabodds
* (tabodds, or and base(): each level's Mantel-Haenszel odds ratio against the base level; adjust() and its stratified test for trend; binomial(); frequency weights; Stata's errors)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 300
generate byte dose = 1 + mod(_n * 11, 4)
generate byte case = mod(_n * 13 + 3 * dose, 17) < 6
generate byte w = 1 + mod(_n, 3)
generate byte agegrp = 1 + mod(_n, 3)
generate byte sex = mod(floor(_n / 3), 2)
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
quietly tabodds case dose, adjust(agegrp)
display "@ adjust " %24.17g r(chi2_tr) " " %24.17g r(p_trend)
quietly tabodds case dose [fw=w], adjust(agegrp sex) base(2)
display "@ adjust weights " %24.17g r(chi2_tr) " " %24.17g r(p_trend)
capture tabodds case dose [fw=w], binomial(w)
display "@ rc binomial weights " _rc
generate byte trials = 1
collapse (sum) case (sum) trials, by(dose agegrp)
quietly tabodds case dose, binomial(trials)
display "@ binomial " %24.17g r(chi2_hom) " " %24.17g r(chi2_tr) " " %24.17g r(p_hom)
quietly tabodds case dose, binomial(trials) adjust(agegrp)
display "@ binomial adjust " %24.17g r(chi2_tr)
