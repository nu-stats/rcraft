* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in contrast_areg.recorded
* commands: contrast pwcompare areg
* (contrast and pwcompare after areg: the tests with e(df_r), r., ar. and the margins, effects and mcompare after it)
* tolerance: 1e-8
version 19
clear
set linesize 255
set seed 7
set obs 120
generate int id = ceil(_n/6)
generate int g = 1 + floor(runiform()*3)
generate int h = 1 + floor(runiform()*2)
generate double x = round(rnormal(), .01)
generate double y = round(1 + .4*(g==2) + .9*(g==3) - .5*(h==2) + .5*x + mod(id,5)/3 + rnormal(), .01)
areg y x i.g, absorb(id)
contrast g
display "@ a1 " %24.17g el(r(F),1,1) " " el(r(df),1,1) " " %24.17g el(r(p),1,1) " " r(df_r)
contrast r.g, effects
display "@ a2 " %24.17g el(r(table),1,1) " " %24.17g el(r(table),2,2) " " %24.17g el(r(table),3,1) " " %24.17g el(r(table),4,2) " " %24.17g el(r(table),5,1)
contrast ar.g
display "@ a3 " %24.17g el(r(b),1,1) " " %24.17g el(r(b),1,2) " " %24.17g el(r(V),1,2)
pwcompare g, effects mcompare(bonferroni)
display "@ a4 " %24.17g el(r(table_vs),1,1) " " %24.17g el(r(table_vs),2,2) " " %24.17g el(r(table_vs),4,3) " " %24.17g el(r(table_vs),5,3) " " %24.17g el(r(table_vs),6,1)
areg y x i.g i.h, absorb(id)
contrast g h
display "@ a5 " %24.17g el(r(F),1,1) " " %24.17g el(r(F),1,2) " " el(r(df),1,2) " " %24.17g el(r(p),1,2) " " r(df_r)
pwcompare h, cimargins
display "@ a6 " %24.17g el(r(table),1,1) " " %24.17g el(r(table),2,2) " " %24.17g el(r(table_vs),1,1)
areg y x i.g, absorb(id) vce(robust)
contrast g
display "@ a8 " %24.17g el(r(F),1,1) " " %24.17g el(r(p),1,1) " " r(df_r)
areg y x i.g, absorb(id) vce(cluster id)
contrast r.g
display "@ a9 " %24.17g el(r(F),1,1) " " %24.17g el(r(F),1,2) " " r(df_r)
