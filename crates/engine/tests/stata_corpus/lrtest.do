* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in lrtest.recorded
* commands: lrtest
* (lrtest of nested logit, probit, poisson and regress fits stored or active; either order; r(chi2), r(df), r(p); the models given twice, equal degrees of freedom, a name not stored)
* tolerance: 1e-6
version 19
clear
set linesize 255
set seed 5
set obs 200
generate x1 = rnormal()
generate x2 = rnormal()
generate g = mod(_n, 4)
generate yb = (x1 + rnormal() > 0)
generate y = x1 + x2 + rnormal()
generate yc = floor(exp(.2 + .3*x1 + .3*runiform()))
quietly logit yb x1 x2
estimates store full
quietly logit yb x1
estimates store small
lrtest small full
display "@ ab " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
lrtest full small
display "@ ba " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
lrtest full
display "@ dot " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly probit yb x1 x2
estimates store pfull
quietly probit yb x1
lrtest pfull .
display "@ probit " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly regress y x1 x2
estimates store rfull
quietly regress y x1
lrtest rfull .
display "@ reg " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly poisson yc x1 x2
estimates store qfull
quietly poisson yc
lrtest qfull .
display "@ pois " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
capture noisily lrtest full full
display "@ rc1 " _rc
quietly logit yb x1 x2 if g > 0
capture noisily lrtest full .
display "@ rc2 " _rc
capture noisily lrtest nosuch .
display "@ rc3 " _rc
