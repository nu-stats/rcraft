* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in mhodds.recorded
* commands: mhodds
* (Mantel-Haenszel odds ratios with their chi2 and interval, controls, compare(), the score test for trend, by() with the test of homogeneity, frequency weights, level(); Stata's errors)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 300
generate byte agegrp = 1 + mod(_n, 3)
generate byte sex = mod(floor(_n / 3), 2)
generate byte smoke = mod(_n * 7, 10) < 3 + agegrp
generate byte dose = 1 + mod(_n * 11, 4)
generate byte case = mod(_n * 13 + 5 * smoke + 3 * dose + agegrp, 17) < 6
generate byte w = 1 + mod(_n, 3)
quietly mhodds case smoke
display "@ binary " %24.17g r(or) " " %24.17g r(lb_or) " " %24.17g r(ub_or) " " %24.17g r(chi2) " " %24.17g r(p)
quietly mhodds case smoke agegrp sex
display "@ controls " %24.17g r(or) " " %24.17g r(lb_or) " " %24.17g r(ub_or) " " %24.17g r(chi2)
quietly mhodds case dose agegrp
display "@ trend " %24.17g r(or) " " %24.17g r(lb_or) " " %24.17g r(ub_or) " " %24.17g r(chi2)
quietly mhodds case dose, compare(4,1)
display "@ compare " %24.17g r(or) " " %24.17g r(lb_or) " " %24.17g r(ub_or) " " %24.17g r(chi2)
quietly mhodds case smoke sex, by(agegrp)
display "@ by " %24.17g r(or) " " %24.17g r(chi2) " " r(df_hom) " " %24.17g r(chi2_hom)
matrix T = r(strata_table)
display "@ by table " %24.17g T[1,1] " " %24.17g T[2,2] " " %24.17g T[3,3] " " %24.17g T[3,4] " " %24.17g T[1,5]
quietly mhodds case dose, by(sex)
display "@ by trend " %24.17g r(or) " " %24.17g r(lb_or) " " r(df_hom) " " %24.17g r(chi2_hom)
quietly mhodds case smoke, by(agegrp sex)
display "@ by two " r(df_hom) " " %24.17g r(chi2_hom) " " rowsof(r(strata_table))
quietly mhodds case smoke agegrp [fw=w]
display "@ weights " %24.17g r(or) " " %24.17g r(lb_or) " " %24.17g r(chi2)
quietly mhodds case smoke, level(90)
display "@ level " %24.17g r(lb_or) " " %24.17g r(ub_or)
capture mhodds case smoke [aw=w]
display "@ rc aweights " _rc
capture mhodds case
display "@ rc too few " _rc
capture mhodds case dose, compare(3,9)
display "@ rc compare " _rc
generate byte case2 = case + 1
capture mhodds case2 smoke
display "@ rc coded " _rc
capture mhodds case smoke if agegrp > 5
display "@ rc no observations " _rc
