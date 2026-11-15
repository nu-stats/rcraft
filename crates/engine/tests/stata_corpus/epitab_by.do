* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in epitab_by.recorded
* commands: cs cc ir tabodds
* (by(): the strata's risk, odds and rate ratios, the crude and the Mantel-Haenszel combined ones with the Greenland-Robins and Robins-Breslow-Greenland intervals, the internally standardized risk ratio, the test of homogeneity, the Mantel-Haenszel test, the pooled rate ratio; tabodds: the odds by level, the test of equal odds and the score test for trend)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 500
generate id = _n
generate byte grp = 1 + mod(id * 11, 4)
generate byte exposed = mod(id * 13, 10) < 5
generate byte case = mod(id * 31 + 7 * grp, 19) < 6 - 3 * exposed + grp
generate double pt = 40 + mod(id * 23, 300)
generate byte age = 1 + mod(id * 7, 3)
generate byte e2 = mod(id * 17, 10) < 4
generate byte c2 = mod(id * 37 + 11 * age, 23) < 3 + 5 * e2 + age
replace exposed = . in 1/5
quietly cs case exposed, by(grp) level(90)
display "@ cs " %24.17g r(rr) " " %24.17g r(lb_rr) " " %24.17g r(ub_rr) " " %24.17g r(afp) " " %24.17g r(lb_crude) " " %24.17g r(chi2_mh)
quietly cs c2 e2, by(age)
display "@ cs2 " %24.17g r(rr) " " %24.17g r(lb_rr) " " %24.17g r(afp) " " %24.17g r(crude) " " %24.17g r(chi2_mh)
quietly cs case exposed, by(grp) istandard
display "@ csi " %24.17g r(rr) " " %24.17g r(lb_rr) " " %24.17g r(ub_rr)
quietly cs c2 e2, by(age) or woolf
display "@ csor " %24.17g r(or) " " %24.17g r(lb_or) " " %24.17g r(ub_or) " " %24.17g r(chi2) " " %24.17g r(chi2_mh) " " %24.17g r(ub_crude)
quietly cc case exposed, by(grp) tb level(90)
display "@ cctb " %24.17g r(or) " " %24.17g r(lb_or) " " %24.17g r(ub_or) " " %24.17g r(afp) " " %24.17g r(lb_crude) " " %24.17g r(chi2_p)
quietly cc c2 e2, by(age) woolf
display "@ ccw " %24.17g r(lb_or) " " %24.17g r(ub_crude) " " %24.17g r(afp) " " %24.17g r(chi2)
quietly ir case exposed pt, by(grp)
display "@ ir " %24.17g r(irr) " " %24.17g r(lb_irr) " " %24.17g r(ub_irr) " " %24.17g r(lb_crude) " " %24.17g r(pooled) " " %24.17g r(ub_pooled) " " %24.17g r(chi2_mh)
quietly ir c2 e2 pt, by(age) level(90)
display "@ ir2 " %24.17g r(irr) " " %24.17g r(ub_irr) " " %24.17g r(crude) " " %24.17g r(lb_pooled)
generate byte dose = mod(id * 11, 4)
quietly tabodds case dose
display "@ tabodds " %24.17g r(chi2_hom) " " %24.17g r(p_hom) " " %24.17g r(chi2_tr) " " %24.17g r(p_trend) " " r(df_hom)
quietly tabodds c2 age, level(90)
display "@ tabodds2 " %24.17g r(chi2_hom) " " %24.17g r(chi2_tr)
