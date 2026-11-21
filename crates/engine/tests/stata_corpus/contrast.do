* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in contrast.recorded
* commands: contrast pwcompare
* (contrast and pwcompare after regress and logit: joint tests, r., ar., gw. and g. contrasts, g#h, g@h, effects, mcompare(bonferroni sidak scheffe), cimargins, level(); r(table), r(F), r(df), r(p), r(df2) and r(chi2))
* tolerance: 1e-8
version 19
clear
set linesize 255
set seed 316
set obs 240
generate byte g = 1 + floor(runiform()*3)
generate byte h = 1 + floor(runiform()*2)
generate double x1 = round(rnormal(), .01)
generate double y = round(1 + .4*(g==2) + .9*(g==3) - .5*(h==2) + .3*(g==3)*(h==2) + .5*x1 + rnormal(), .01)
generate byte yb = (y + rnormal() > 1.5)
generate double w = x1 + 4
regress y i.g x1
contrast g
display "@ joint " %24.17g el(r(F),1,1) " " el(r(df),1,1) " " %24.17g el(r(p),1,1) " " el(r(df2),1,1) " " r(df_r) " " r(k_terms) " " r(level)
contrast r.g
display "@ r " %24.17g el(r(b),1,1) " " %24.17g el(r(b),1,2) " " %24.17g sqrt(el(r(V),1,1)) " " %24.17g el(r(V),1,2) " " %24.17g el(r(V),2,2)
display "@ rtab " %24.17g el(r(table),1,1) " " %24.17g el(r(table),2,2) " " %24.17g el(r(table),3,1) " " %24.17g el(r(table),4,2) " " %24.17g el(r(table),5,1) " " %24.17g el(r(table),6,2) " " el(r(table),7,1)
contrast ar.g, effects
display "@ ar " %24.17g el(r(table),1,2) " " %24.17g el(r(table),2,2) " " %24.17g el(r(table),4,2)
contrast gw.g
display "@ gw " %24.17g el(r(table),1,1) " " %24.17g el(r(table),2,3) " " %24.17g el(r(table),5,3) " " %24.17g el(r(F),1,1)
contrast g.g
display "@ gm " %24.17g el(r(table),1,1) " " %24.17g el(r(table),2,3) " " %24.17g el(r(table),6,2)
contrast r.g, mcompare(bonferroni) effects
display "@ bonf " %24.17g el(r(table),4,1) " " %24.17g el(r(table),5,2) " " %24.17g el(r(table),6,2)
contrast r.g, mcompare(sidak) effects
display "@ sidak " %24.17g el(r(table),4,1) " " %24.17g el(r(table),5,2) " " %24.17g el(r(table),6,2)
contrast r.g, mcompare(scheffe)
display "@ scheffe " %24.17g el(r(table),4,1) " " %24.17g el(r(table),5,2) " " %24.17g el(r(table),6,2)
contrast r.g, level(90)
display "@ lev " %24.17g el(r(table),5,1) " " %24.17g el(r(table),6,2) " " r(level)
pwcompare g
display "@ pw " %24.17g el(r(table_vs),1,1) " " %24.17g el(r(table_vs),1,3) " " %24.17g el(r(table_vs),2,2) " " %24.17g el(r(table_vs),5,2) " " %24.17g el(r(table_vs),6,3)
pwcompare g, effects mcompare(bonferroni)
display "@ pwb " %24.17g el(r(table_vs),4,1) " " %24.17g el(r(table_vs),5,2) " " %24.17g el(r(table_vs),6,3)
pwcompare g, effects mcompare(sidak)
display "@ pws " %24.17g el(r(table_vs),4,1) " " %24.17g el(r(table_vs),5,2) " " %24.17g el(r(table_vs),6,3)
pwcompare g, effects mcompare(scheffe)
display "@ pwf " %24.17g el(r(table_vs),4,1) " " %24.17g el(r(table_vs),5,2) " " %24.17g el(r(table_vs),6,3)
regress y i.g##i.h x1
contrast g#h
display "@ gh " %24.17g el(r(F),1,1) " " el(r(df),1,1) " " %24.17g el(r(p),1,1) " " r(df_r)
contrast g h g#h
display "@ ghall " %24.17g el(r(F),1,1) " " %24.17g el(r(F),1,2) " " %24.17g el(r(F),1,3) " " el(r(df),1,3) " " %24.17g el(r(p),1,2)
contrast g@h
display "@ gath " %24.17g el(r(F),1,1) " " el(r(df),1,1)
contrast r.g@h, effects
display "@ rgath " %24.17g el(r(table),1,2) " " %24.17g el(r(table),2,2) " " %24.17g el(r(table),1,4) " " %24.17g el(r(table),4,3)
pwcompare g#h, effects
display "@ pwgh " %24.17g el(r(table_vs),1,1) " " %24.17g el(r(table_vs),1,15) " " %24.17g el(r(table_vs),2,9) " " %24.17g el(r(table_vs),4,6)
quietly regress y ib2.g x1
contrast r.g
display "@ ib2 " %24.17g el(r(table),1,1) " " %24.17g el(r(table),1,2)
quietly regress y i.g##c.x1
contrast g
display "@ gx " %24.17g el(r(F),1,1) " " r(df_r)
contrast r.g
display "@ rgx " %24.17g el(r(table),1,1) " " %24.17g el(r(table),2,2)
quietly regress y i.g x1, robust
contrast g
display "@ rob " %24.17g el(r(F),1,1) " " r(df_r)
quietly regress y i.g x1, vce(cluster h)
contrast g
display "@ clu " %24.17g el(r(F),1,1) " " el(r(df),1,1) " " r(df_r)
quietly regress y i.g x1 [aw = w]
contrast g
display "@ aw " %24.17g el(r(F),1,1)
anova y g h
contrast g
display "@ anova " %24.17g el(r(F),1,1) " " r(df_r)
pwcompare h, effects
display "@ pwh " %24.17g el(r(table),1,1) " " %24.17g el(r(table),4,1)
capture noisily contrast x1
display "@ rc " _rc
