* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in contrast_omitted.recorded
* commands: contrast pwcompare
* (contrast and pwcompare where the design cannot tell the margins apart: a factor that is a copy of another, a factor constant within the panels after xtreg fe, and a time-varying one beside it; xtreg re and fe tests)
* tolerance: 1e-8
version 19
clear
set linesize 255
set obs 90
generate int id = ceil(_n/6)
generate int t = mod(_n-1, 6) + 1
xtset id t
generate int g = mod(id, 3)
generate int h = g
generate int k = mod(_n, 3)
generate double x = mod(_n*7, 11)
generate double y = x + g + mod(_n*5, 7)
regress y x i.g i.h
contrast g h
display "@ nt1 " missing(el(r(F),1,1)) " " missing(el(r(F),1,2)) " " r(df_r)
pwcompare h
display "@ pw1 " missing(el(r(table_vs),1,1)) " " missing(el(r(table_vs),1,3)) " " colsof(r(table_vs))
regress y x i.h i.g
contrast g
display "@ nt2 " missing(el(r(F),1,1))
xtreg y x i.g, fe
contrast g
display "@ fe1 " missing(el(r(F),1,1)) " " r(df_r)
pwcompare g
display "@ fe2 " missing(el(r(table_vs),1,1)) " " missing(el(r(table_vs),1,3))
xtreg y x i.g i.k, fe
contrast g k
display "@ fe3 " missing(el(r(F),1,1)) " " %24.17g el(r(F),1,2) " " el(r(df),1,2) " " %24.17g el(r(p),1,2) " " r(df_r)
pwcompare k
display "@ fe4 " %24.17g el(r(table_vs),1,1) " " %24.17g el(r(table_vs),2,2) " " %24.17g el(r(table_vs),6,3)
contrast r.k, effects
display "@ fe5 " %24.17g el(r(table),1,1) " " %24.17g el(r(table),4,2)
xtreg y x i.k, re
contrast k
display "@ re1 " %24.17g el(r(chi2),1,1) " " el(r(df),1,1) " " %24.17g el(r(p),1,1)
pwcompare k, effects
display "@ re2 " %24.17g el(r(table_vs),3,1) " " %24.17g el(r(table_vs),5,3)
