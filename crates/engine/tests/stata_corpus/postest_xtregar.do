* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in postest_xtregar.recorded
* commands: hausman predictnl xtregar estimates
* (tolerance 1e-6 because xtregar keeps its transform in single precision; hausman between xtregar fe and re, with constant, reversed, sigmamore and beside regress; predictnl after xtregar with se(), ci(), variance(), an expression with _b[] and the stop when a variable exists)
* tolerance: 1e-6
version 19
clear
set linesize 255
set obs 16
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
estimates store f
quietly xtregar y x1 x2, re
estimates store r
hausman f r
display "@ h1 " %24.17g r(chi2) " " r(df) " " %24.17g r(p) " " r(rank)
hausman r f
display "@ h2 " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
hausman f r, constant
display "@ h3 " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly xtregar y x1, fe
estimates store f2
hausman f f2, sigmamore
display "@ h4 " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
hausman f2 f
display "@ h5 " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
capture hausman f r, sigmaless
display "@ h6 " _rc
quietly regress y x1 x2
estimates store ols
hausman ols r
display "@ h7 " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly xtregar y x1 x2, fe
predictnl double pn = predict(xb) + 1, se(pnse)
quietly summarize pn
display "@ p1 " %24.17g r(mean) " " r(N) " " %24.17g r(sd)
quietly summarize pnse
display "@ p2 " %24.17g r(mean) " " %24.17g r(sd)
capture predictnl double pn = predict(xb) + 1, se(pnse)
display "@ p3 " _rc
predictnl double q = xb()^2 + _b[x1], se(qse) ci(ql qu) variance(qv)
quietly summarize q
display "@ p4 " %24.17g r(mean) " " r(N)
quietly summarize qse
display "@ p5 " %24.17g r(mean)
quietly summarize ql
display "@ p6 " %24.17g r(mean)
quietly summarize qv
display "@ p7 " %24.17g r(mean)
quietly xtregar y x1 x2, re
predictnl double w = predict(xb)*x1 if t > 2, se(wse) wald(ww) p(wp)
quietly summarize w
display "@ p8 " %24.17g r(mean) " " r(N)
quietly summarize wse
display "@ p9 " %24.17g r(mean)
quietly summarize wp
display "@ p10 " %24.17g r(mean)
