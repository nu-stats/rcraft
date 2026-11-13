* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in mlogit_tests.recorded
* commands: testparm nlcom
* (testparm and nlcom after mlogit: every equation, the base outcome's coefficients dropped, equal in the first equation, baseoutcome(), a factor-by-continuous interaction; to 6 digits, where Stata stops iterating)
* tolerance: 1e-6
version 19
clear
set linesize 255
set seed 62
set obs 400
generate z = rnormal()
generate w = rnormal()
generate g = 1 + floor(3*runiform())
generate u = z + .5*w + rnormal()
generate yo = 1 + (u > -.5) + (u > .7)
quietly mlogit yo z w i.g
testparm z
display "@ tz " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
testparm z w
display "@ tzw " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
testparm i.g
testparm z w, equal
display "@ teq " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
nlcom [1]_b[z] / [2]_b[z]
display "@ nl " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
nlcom (a: [1]_b[z] - [2]_b[z]) (b: exp([2]_b[w]))
display "@ nl2 " %24.17g el(r(b),1,2) " " %24.17g el(r(V),2,2) " " %24.17g el(r(V),1,2)
quietly mlogit yo z w, baseoutcome(2)
testparm z
display "@ tb " %24.17g r(chi2) " " r(df)
quietly mlogit yo z i.g c.w#i.g
testparm i.g#c.w
display "@ tint " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
testparm c.w#i.g
display "@ tint2 " %24.17g r(chi2) " " r(df)
