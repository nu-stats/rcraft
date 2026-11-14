* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in pca.recorded
* commands: pca
* (pca of the correlation and covariance matrices, components(); e(Ev), e(L) signed as Stata signs them, e(Psi), e(components), e(rho), e(trace), e(cond), e(N))
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 103
set obs 100
generate a = rnormal()
generate b = .6*a + rnormal()
generate c = .3*a - .5*b + rnormal()
generate d = rnormal()
replace d = . in 1/3
pca a b c d
display "@ ev " %24.17g el(e(Ev),1,1) " " %24.17g el(e(Ev),1,4) " " e(N) " " e(f) " " %24.17g e(cond)
display "@ L " %24.17g el(e(L),1,1) " " %24.17g el(e(L),2,2) " " %24.17g el(e(L),3,3) " " %24.17g el(e(L),4,4)
pca a b c, components(2)
display "@ k2 " %24.17g e(rho) " " %24.17g el(e(Psi),1,1) " " %24.17g el(e(Psi),1,3) " " %24.17g el(e(L),2,2)
display "@ comp " %24.17g el(e(components),1,2) " " %24.17g el(e(components),2,4) " " %24.17g el(e(components),3,3)
pca a b c, covariance
display "@ cov " %24.17g e(trace) " " %24.17g el(e(Ev),1,1) " " %24.17g el(e(L),1,2)
