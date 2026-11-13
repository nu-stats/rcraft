* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in test_options.recorded
* commands: test
* (test's options after regress and logit: matvlc(), mtest with each adjustment and r(mtest), accumulate, notest; to 6 digits after logit, where Stata stops iterating)
* tolerance: 1e-6
version 19
clear
set linesize 255
set seed 81
set obs 200
generate x = rnormal()
generate w = rnormal()
generate v = rnormal()
generate g = 1 + floor(3*runiform())
generate y = 1 + .5*x + .2*w + .1*v + .3*g + rnormal()
generate yb = y > 2
regress y x w v
test x w, matvlc(L)
display "@ vlc " %24.17g L[1,1] " " %24.17g L[2,1] " " %24.17g L[2,2] " " rowsof(L)
test (x = w) (v = 0.5), matvlc(L2)
display "@ vlc2 " %24.17g L2[1,1] " " %24.17g L2[1,2] " " %24.17g r(F)
test x w v, mtest
matrix M = r(mtest)
display "@ mt " %24.17g M[1,1] " " %24.17g M[2,3] " " %24.17g M[3,3] " " colsof(M) " " r(mtmethod) " " %24.17g r(F)
test x w v, mtest(holm)
matrix M = r(mtest)
display "@ holm " %24.17g M[1,4] " " %24.17g M[2,4] " " %24.17g M[3,4] " " r(mtmethod)
test x w v, mtest(sidak)
matrix M = r(mtest)
display "@ sidak " %24.17g M[1,4] " " %24.17g M[3,4]
test (x = w) (v = .2), mtest(b)
matrix M = r(mtest)
display "@ bonf " %24.17g M[1,1] " " %24.17g M[1,4] " " %24.17g M[2,3] " " r(mtmethod)
test x
test w, accumulate
display "@ acc " %24.17g r(F) " " r(df) " " %24.17g r(p)
test (v = 0.1), accumulate
display "@ acc3 " %24.17g r(F) " " r(df)
test x = w, accumulate notest
display "@ notest " r(df) " " r(F)
logit yb x w v
test x w v, mtest(holm)
matrix M = r(mtest)
display "@ lholm " %24.17g M[1,1] " " %24.17g M[2,4] " " %24.17g r(chi2) " " colsof(M)
test x w, notest
test v, accumulate
display "@ lacc " %24.17g r(chi2) " " r(df)
