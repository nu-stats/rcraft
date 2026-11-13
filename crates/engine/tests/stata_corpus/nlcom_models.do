* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in nlcom_models.recorded
* commands: nlcom
* (nlcom after xtreg fe and re, ivregress, nbreg (with /lnalpha), ologit and oprobit (with the cut points), clogit, stcox and regress with robust errors; to 6 digits, where Stata stops iterating the likelihood models)
* tolerance: 1e-6
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
generate y = 1 + .5*z + .8*w + rnormal()
generate cnt = floor(exp(.2 + .3*z + .1*w) * runiform() * 4)
generate yo = 1 + (y > 2) + (y > 3.5)
generate yb = y > 3
generate time = 1 + floor(10*runiform())
generate fail = runiform() < .7
xtset id t
quietly xtreg y z w, fe
nlcom _b[z] / _b[w]
display "@ xtfe " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
quietly xtreg y z w, re
nlcom _b[z] / _b[w]
display "@ xtre " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
quietly ivregress 2sls y w (z = q)
nlcom _b[z] / _b[w]
display "@ iv " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
quietly nbreg cnt z w
nlcom _b[z] / _b[w]
display "@ nb " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
nlcom exp(_b[/lnalpha])
display "@ alpha " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
quietly ologit yo z w
nlcom _b[z] / _b[w]
display "@ ol " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
nlcom _b[/cut2] - _b[/cut1]
display "@ cuts " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
quietly oprobit yo z w
nlcom _b[z] / _b[w]
display "@ op " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
quietly clogit yb z w, group(id)
nlcom _b[z] / _b[w]
display "@ cl " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
stset time, failure(fail)
quietly stcox z w
nlcom _b[z] / _b[w]
display "@ cox " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
quietly regress y z w, vce(robust)
nlcom _b[z] / _b[w]
display "@ rob " %24.17g el(r(b),1,1) " " %24.17g el(r(V),1,1)
