* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in contrast_xtregar.recorded
* commands: contrast pwcompare predictnl hausman xtregar
* (xtregar with factor variables: contrast and pwcompare after fe (a factor constant within the panels is omitted: not testable, 0 and (omitted)) and re, a copy of a factor, predictnl and hausman with the factor's columns)
* tolerance: 1e-6
version 19
clear
set linesize 255
set obs 16
generate int id = _n
expand 7
bysort id: generate int t = _n
generate double x1 = mod(id*37 + t*101 + id*t*13, 89)/20
generate double u = (mod(id*61, 23) - 11)/8
generate double eps = (mod(id*17 + t*43 + id*t*5, 67) - 33)/22
generate double e = eps
bysort id (t): replace e = eps + .5*e[_n-1] if t > 1
generate double y = 1 + .5*x1 + u + e
xtset id t
generate byte g = mod(id, 3)
generate byte k = mod(t, 3)
generate byte h = g
quietly xtregar y x1 i.g, fe
contrast g
display "@ f1 " missing(el(r(F),1,1)) " " el(r(df),1,1) " " r(df_r)
pwcompare g
display "@ f2 " el(r(table_vs),1,1) " " missing(el(r(table_vs),2,1)) " " el(r(table_vs),7,2) " " colsof(r(table_vs))
quietly xtregar y x1 i.g i.k, fe
contrast g k
display "@ f3 " missing(el(r(F),1,1)) " " %24.17g el(r(F),1,2) " " el(r(df),1,2) " " %24.17g el(r(p),1,2) " " r(df_r)
pwcompare k, effects
display "@ f4 " %24.17g el(r(table_vs),1,1) " " %24.17g el(r(table_vs),2,2) " " %24.17g el(r(table_vs),4,3) " " %24.17g el(r(table_vs),6,1)
contrast r.g r.k, effects
display "@ f5 " el(r(table),1,1) " " missing(el(r(table),2,1)) " " %24.17g el(r(table),1,3) " " %24.17g el(r(table),2,4) " " %24.17g el(r(F),1,4) " " %24.17g el(r(F),1,6) " " el(r(df),1,3)
pwcompare g#k
display "@ f6 " %24.17g el(r(table_vs),1,1) " " el(r(table_vs),1,3) " " missing(el(r(table_vs),2,3)) " " %24.17g el(r(table_vs),1,4)
pwcompare g, cimargins
display "@ f7 " %24.17g el(r(table),1,1) " " %24.17g el(r(table),2,3)
contrast g@k
display "@ f8 " missing(el(r(F),1,1)) " " missing(el(r(F),1,4))
quietly xtregar y x1 i.g i.h, fe
contrast g h
display "@ f9 " missing(el(r(F),1,1)) " " missing(el(r(F),1,2)) " " r(df_r)
pwcompare h
display "@ f10 " missing(el(r(table_vs),1,1)) " " missing(el(r(table_vs),1,3))
quietly xtregar y x1 i.g i.k, re
contrast g k
display "@ r1 " %24.17g el(r(chi2),1,1) " " el(r(df),1,1) " " %24.17g el(r(chi2),1,2) " " %24.17g el(r(p),1,2)
pwcompare g, cimargins
display "@ r2 " %24.17g el(r(table),1,1) " " %24.17g el(r(table),2,2) " " %24.17g el(r(table),1,3)
pwcompare g, effects
display "@ r3 " %24.17g el(r(table_vs),1,1) " " %24.17g el(r(table_vs),2,2) " " %24.17g el(r(table_vs),4,3)
quietly xtregar y x1 i.g i.h, re
contrast g h
display "@ r4 " missing(el(r(chi2),1,1)) " " missing(el(r(chi2),1,2))
pwcompare h
display "@ r5 " missing(el(r(table_vs),1,1)) " " missing(el(r(table_vs),1,3))
quietly xtregar y x1 i.g i.k, fe
estimates store f
quietly xtregar y x1 i.g i.k, re
estimates store r
hausman f r
display "@ h1 " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
estimates restore f
contrast k
display "@ rs1 " %24.17g el(r(F),1,1) " " r(df_r)
estimates restore r
predictnl double pn = predict(xb) + 1, se(pnse)
quietly summarize pn
display "@ p1 " %24.17g r(mean) " " r(N)
quietly summarize pnse
display "@ p2 " %24.17g r(mean)
estimates restore f
predictnl double pf = predict(xb), se(pfse)
quietly summarize pfse
display "@ p3 " %24.17g r(mean)
