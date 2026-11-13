* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in testparm_models.recorded
* commands: testparm
* (testparm after xtreg re, ivregress (and small), clogit and stcox, with factor variables and equal)
* tolerance: 1e-8
version 19
clear
set linesize 255
set seed 42
set obs 300
generate id = 1 + mod(_n, 30)
generate t = 1 + floor((_n - 1)/30)
generate z = rnormal()
generate w = 2 + rnormal()
generate q = z + rnormal()
generate g = 1 + floor(3*runiform())
generate y = 1 + .5*z + .8*w + .2*g + rnormal()
generate yb = y > 3
generate time = 1 + floor(10*runiform())
generate fail = runiform() < .7
xtset id t
quietly xtreg y z w i.g, re
testparm z w
display "@ xtre " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly ivregress 2sls y w i.g (z = q)
testparm z i.g
display "@ iv " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly clogit yb z w i.g, group(id)
testparm z i.g
display "@ cl " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
stset time, failure(fail)
quietly stcox z w i.g
testparm z w, equal
display "@ cox " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly ivregress 2sls y w (z = q), small
testparm z w
display "@ ivs " %24.17g r(F) " " r(df) " " r(df_r) " " %24.17g r(p)
