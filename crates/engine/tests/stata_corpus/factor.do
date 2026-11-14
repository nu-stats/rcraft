* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in factor.recorded
* commands: factor
* (factor by principal factors, principal-component factors and iterated principal factors, with factors(), mineigen() and citerate(): e(L) signed as Stata signs it, e(Psi), e(Ev), the test of independence, e(df_m), e(df_r))
* tolerance: 1e-8
version 19
clear
set linesize 255
set seed 104
set obs 200
generate f1 = rnormal()
generate f2 = rnormal()
generate a1 = .8*f1 + .6*rnormal()
generate a2 = .7*f1 + .7*rnormal()
generate a3 = .6*f1 + .3*f2 + .7*rnormal()
generate a4 = .8*f2 + .6*rnormal()
generate a5 = .7*f2 + .7*rnormal()
factor a1-a5
display "@ pf " e(f) " " %24.17g el(e(L),1,1) " " %24.17g el(e(L),5,2) " " %24.17g el(e(Psi),1,3) " " %24.17g el(e(Ev),1,5) " " %24.17g e(chi2_i) " " e(df_m) " " e(df_r)
factor a1-a5, pcf
display "@ pcf " e(f) " " %24.17g el(e(L),2,1) " " %24.17g el(e(Psi),1,4) " " %24.17g e(evsum)
factor a1-a5, ipf factors(2)
display "@ ipf " %24.17g el(e(L),1,1) " " %24.17g el(e(L),4,2) " " %24.17g el(e(Psi),1,5) " " %24.17g el(e(Ev),1,3)
factor a1-a5, ipf factors(2) citerate(5)
display "@ cit " %24.17g el(e(Ev),1,1) " " %24.17g el(e(L),3,2)
factor a1-a5, factors(1)
display "@ f1 " e(f) " " %24.17g el(e(L),2,1) " " %24.17g el(e(Psi),1,2) " " e(df_m) " " e(df_r)
factor a1-a5, mineigen(.5)
display "@ min " e(f) " " %24.17g el(e(L),1,1)
