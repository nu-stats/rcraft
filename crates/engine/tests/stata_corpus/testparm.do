* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in testparm.recorded
* commands: testparm
* (testparm after regress and logit: continuous variables, i.g, interactions in either order, ## as its main effects and interaction, c.x#c.x, wildcards, equal; r() as test leaves it)
* tolerance: 1e-8
version 19
clear
set seed 5
set obs 300
generate g = 1 + floor(4*runiform())
generate female = runiform() < .5
generate x1 = rnormal()
generate x2 = rnormal()
generate z3 = rnormal()
generate age = 20 + 40*runiform()
generate y = 1 + .5*x1 - .2*x2 + .3*(g == 3) + .02*age + .4*female*(g == 2) + rnormal()
generate yb = y > 1.5
regress y c.age##c.age i.g##i.female x1 x2 z3
testparm c.age#c.age
display "@ sq " %24.17g r(F) " " r(df) " " r(df_r) " " %24.17g r(p) " " r(drop)
testparm i.g#i.female
display "@ gf " %24.17g r(F) " " r(df) " " r(df_r) " " %24.17g r(p)
testparm i.female#i.g
display "@ fg " %24.17g r(F) " " r(df)
testparm i.g##i.female
display "@ full " %24.17g r(F) " " r(df) " " %24.17g r(p)
testparm z3 i.g x1
display "@ mixed " %24.17g r(F) " " r(df) " " %24.17g r(p)
testparm c.age
display "@ age " %24.17g r(F) " " r(df)
testparm x*
display "@ wild " %24.17g r(F) " " r(df) " " %24.17g r(p)
testparm x2 x1 z3, equal
display "@ equal " %24.17g r(F) " " r(df) " " %24.17g r(p)
capture testparm x1, equal
display "@ one_equal " _rc
capture testparm i.female#c.age
display "@ none " _rc
regress y i.female##c.age x1
testparm c.age#i.female
display "@ fa " %24.17g r(F) " " r(df) " " r(df_r)
logit yb x1 x2 i.g
testparm x1 x2
display "@ logit " %24.17g r(chi2) " " r(df) " " %24.17g r(p) " " r(drop)
testparm i.g, equal
display "@ logit_eq " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly testparm i.g
display "@ quiet " %24.17g r(chi2) " " r(df)
