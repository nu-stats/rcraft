* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in lincom_post.recorded
* commands: lincom test nlcom
* (lincom with a constant after regress, logit, poisson, stcox, ivregress and mlogit, the base outcome's 0; nlcom, post: e(), then test, lincom and nlcom on the combinations; to 6 digits, where Stata stops iterating after the likelihood models)
* tolerance: 1e-6
version 19
clear
set linesize 255
set seed 74
set obs 300
generate x = rnormal()
generate w = rnormal()
generate y = 1 + .5*x + .5*w + rnormal()
generate yb = y > 1
generate yc = floor(3*runiform()*exp(.3*x + .2*w))
generate z = x + rnormal()
generate t = -ln(runiform())/exp(.3*x)
generate d = runiform() < .8
generate yo = 1 + (y > 0.5) + (y > 1.5)
regress y x w
lincom x + w - 1
display "@ reg " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(lb) " " %24.17g r(ub) " " r(df)
lincom 2 + 0.5*x, level(90)
display "@ reg2 " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(p) " " r(level)
test 2*x = 3*w + 1
display "@ test " %24.17g r(F) " " %24.17g r(p)
logit yb x w
lincom x - w + 0.25
display "@ logit " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(lb) " " %24.17g r(ub) " " %24.17g r(z)
poisson yc x w
lincom x - w, irr
display "@ pois " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(p)
stset t, failure(d)
stcox x w
lincom x - w
display "@ cox " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(z)
ivregress 2sls y w (x = z)
lincom x - w + 0.5
display "@ iv " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(lb)
mlogit yo x w
lincom [1]x + 1
display "@ mlogit " %24.17g r(estimate) " " %24.17g r(se)
lincom [3]x
display "@ base " r(estimate) " " r(se)
regress y x w
nlcom (r: _b[x]/_b[w]) (p: _b[x]*_b[w]), post
display "@ post " e(N) " " e(rank) " " %24.17g _b[r] " " %24.17g _se[p] " " e(cmd) " " e(depvar)
test r = 1
display "@ ptest " %24.17g r(chi2) " " %24.17g r(p)
lincom r - p
display "@ plincom " %24.17g r(estimate) " " %24.17g r(se) " " %24.17g r(z)
nlcom (q: _b[r]/_b[p])
display "@ pnlcom " %24.17g el(r(b), 1, 1) " " %24.17g el(r(V), 1, 1)
