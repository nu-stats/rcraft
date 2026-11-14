* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in meanest.recorded
* commands: mean total proportion ratio
* (mean, total, proportion and ratio: e(b), e(V), e(N), e(df_r), e(sd), e(freq), e(_N) and r(table), with frequency, analytic and probability weights, over(), level(), iweights, prop, two variables in over(); test and lincom after them, the lines naming over()'s and proportion's levels)
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 93
set obs 120
generate g = 1 + floor(3*runiform())
label define gl 1 "one" 2 "two" 3 "three"
label values g gl
generate b = runiform() < .4
generate y = 1 + .3*g + rnormal()
generate x = 2 + rnormal()
replace y = . in 1/4
replace b = . in 118
generate wt = 1 + mod(_n, 4)
mean y x
display "@ mean " %24.17g _b[y] " " %24.17g _se[x] " " %24.17g el(e(V),1,2) " " e(N) " " e(df_r) " " %24.17g el(e(sd),1,1) " " %24.17g el(r(table),5,1)
mean y, level(90)
display "@ m90 " %24.17g el(r(table),5,1) " " %24.17g el(r(table),6,1)
mean y x, over(b)
display "@ mover " %24.17g el(e(b),1,2) " " %24.17g sqrt(el(e(V),2,2)) " " %24.17g el(e(V),3,4) " " e(N_over) " " el(e(_N),1,2) " " %24.17g el(e(sd),1,3)
mean y x [fweight = wt]
display "@ mfw " %24.17g _se[y] " " %24.17g el(e(V),1,2) " " e(N) " " e(df_r) " " %24.17g el(e(sd),1,1)
mean y [aweight = wt]
display "@ maw " %24.17g _se[y] " " %24.17g el(e(sd),1,1)
mean y [aweight = wt], over(b)
display "@ mawo " %24.17g sqrt(el(e(V),1,1)) " " %24.17g sqrt(el(e(V),2,2))
mean y x [pweight = wt]
display "@ mpw " %24.17g _se[y] " " %24.17g el(e(V),1,2) " " %24.17g el(e(sd),1,2) " " e(vce)
mean y x [pweight = wt], over(g)
display "@ mpwo " %24.17g sqrt(el(e(V),1,1)) " " %24.17g sqrt(el(e(V),6,6)) " " %24.17g el(e(V),1,4)
mean y x
test y = x
display "@ mtest " %24.17g r(F) " " r(df_r)
lincom y - x + 1
display "@ mlin " %24.17g r(estimate) " " %24.17g r(se)
total y x, over(b)
display "@ tover " %24.17g el(e(b),1,1) " " %24.17g el(e(V),1,3) " " %24.17g sqrt(el(e(V),4,4))
lincom c.y@0.b - c.y@1.b
display "@ tlin " %24.17g r(estimate) " " %24.17g r(se)
total y [pweight = wt]
display "@ tpw " %24.17g _b[y] " " %24.17g _se[y]
total y [fweight = wt]
display "@ tfw " %24.17g _b[y] " " %24.17g _se[y]
proportion g b
display "@ prop " e(N) " " %24.17g el(e(b),1,1) " " %24.17g el(e(V),1,2) " " %24.17g el(e(V),1,4) " " %24.17g el(r(table),5,2) " " el(e(freq),1,3)
lincom 1.g - 2.g
display "@ plin " %24.17g r(estimate) " " %24.17g r(se)
test 1.b = .5
display "@ ptest " %24.17g r(F)
proportion b, over(g)
display "@ pover " %24.17g el(e(b),1,2) " " %24.17g sqrt(el(e(V),2,2)) " " %24.17g el(r(table),6,4) " " el(e(_N),1,3)
proportion g [pweight = wt]
display "@ ppw " %24.17g el(e(b),1,1) " " %24.17g sqrt(el(e(V),1,1)) " " %24.17g el(e(V),1,2) " " el(e(freq),1,1)
proportion g [fweight = wt]
display "@ pfw " %24.17g sqrt(el(e(V),1,1)) " " e(N)
ratio y/x
display "@ ratio " %24.17g _b[_ratio_1] " " %24.17g _se[_ratio_1] " " e(vce)
lincom _ratio_1 - 1
display "@ rlin " %24.17g r(estimate) " " %24.17g r(se)
ratio y/x [pweight = wt]
display "@ rpw " %24.17g _se[_ratio_1]
ratio y/x [fweight = wt]
display "@ rfw " %24.17g _se[_ratio_1] " " e(N)
ratio (a: y/x) (c: x/y), over(b)
display "@ rover " %24.17g el(e(b),1,1) " " %24.17g sqrt(el(e(V),1,1)) " " %24.17g sqrt(el(e(V),4,4)) " " e(namelist)
mean y [iweight = wt]
display "@ miw " %24.17g _se[y] " " e(N) " " e(wtype)
proportion g [iweight = wt]
display "@ piw " %24.17g sqrt(el(e(V),1,1)) " " e(N)
prop g
display "@ prop2 " %24.17g el(e(b),1,2) " " e(cmd)
mean y, over(g b)
display "@ mover2 " %24.17g el(e(b),1,3) " " %24.17g sqrt(el(e(V),1,1)) " " %24.17g sqrt(el(e(V),6,6)) " " e(N_over)
mean y x, over(b)
display "@ bat " %24.17g _b[c.y@0.b] " " %24.17g _se[c.x@1.b] " " %24.17g _b[c.x@1bn.b]
proportion g
display "@ bprop " %24.17g _b[2.g] " " %24.17g _se[1.g]
