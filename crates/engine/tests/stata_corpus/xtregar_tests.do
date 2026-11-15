* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in xtregar_tests.recorded
* commands: test lincom nlcom
* (test, lincom and nlcom after xtregar, fe and re: F or chi2, t or z, as Stata 19 reads xtregar's e(b) and e(V); the re test's p, far in the tail, is Stata's to 5e-8)
* tolerance: 1e-7
version 19
clear
set linesize 255
set obs 14
generate int id = _n
expand 7
bysort id: generate int t = _n
generate double x1 = mod(id*37 + t*101 + id*t*13, 89)/20
generate double x2 = mod(id*53 + t*29 + id*t*7, 71)/30
generate double u = (mod(id*61, 23) - 11)/8
generate double eps = (mod(id*17 + t*43 + id*t*5, 67) - 33)/22
generate double e = eps
bysort id (t): replace e = eps + .5*e[_n-1] if t > 1
generate double y = 1 + .5*x1 - .3*x2 + u + e
xtset id t
quietly xtregar y x1 x2, fe
test x1 x2
display "@ fe " %24.17g r(F) " " r(df_r)
lincom 2*x1 - x2 + _cons
display "@ fel " %24.17g r(estimate) " " %24.17g r(se) " " r(df)
nlcom _b[x1]/_b[x2]
display "@ fen " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
quietly xtregar y x1 x2, re
test x1 = x2
display "@ re " %24.17g r(chi2) " " %24.17g r(p)
lincom x1 + x2
display "@ rel " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(z)
