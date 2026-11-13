* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in nlcom.recorded
* commands: nlcom
* (nlcom after regress and logit: ratios, products, exp(), sqrt() and ln(), invlogit(), factor-variable coefficients, named expressions, level(), df(), noheader, iterate(); r(b), r(V), r(table), r(N), r(df_r) and r(level))
* tolerance: 1e-8
version 19
clear
set linesize 255
set seed 41
set obs 200
generate z = rnormal()
generate w = 2 + rnormal()
generate treat = runiform() < .5
generate female = runiform() < .5
generate y = 1 + .5*z + .8*w + .3*treat + .2*treat*female + rnormal()
generate yb = y > 3
regress y z w i.treat##i.female
nlcom _b[z] / _b[w]
display "@ ratio " %24.17g el(r(b), 1, 1) " " %24.17g sqrt(el(r(V), 1, 1)) " " r(N) " " r(level)
nlcom (ratio: _b[z] / _b[w]) (prod: _b[z] * _b[w])
display "@ two " %24.17g el(r(b), 1, 2) " " %24.17g el(r(V), 2, 2) " " %24.17g el(r(V), 1, 2)
nlcom _b[1.treat#1.female] / _b[1.treat]
display "@ fv " %24.17g el(r(b), 1, 1) " " %24.17g el(r(V), 1, 1)
nlcom exp(_b[_cons])
display "@ exp " %24.17g el(r(b), 1, 1) " " %24.17g el(r(V), 1, 1)
nlcom sqrt(_b[w]) + ln(_b[w]), level(90)
display "@ sqrt " %24.17g el(r(b), 1, 1) " " %24.17g el(r(V), 1, 1) " " r(level)
logit yb z w
nlcom (a: exp(_b[z])) (b: invlogit(_b[_cons]))
display "@ logit " %24.17g el(r(b), 1, 1) " " %24.17g el(r(V), 1, 1) " " %24.17g el(r(b), 1, 2) " " %24.17g el(r(V), 2, 2) " " %24.17g el(r(V), 1, 2)
quietly nlcom _b[z] / _b[w]
display "@ quiet " %24.17g el(r(b), 1, 1) " " %24.17g el(r(V), 1, 1)
regress y z w
nlcom (r: _b[z] / _b[w]) (p: _b[z]*_b[w]), df(30) noheader
display "@ df " %24.17g el(r(table), 3, 1) " " %24.17g el(r(table), 4, 2) " " %24.17g el(r(table), 5, 1) " " r(df_r) " " %24.17g el(r(table), 8, 1)
nlcom _b[z] / _b[w], df(197) level(90) iterate(50)
display "@ df90 " %24.17g el(r(table), 5, 1) " " %24.17g el(r(table), 6, 1) " " r(df_r) " " r(level)
