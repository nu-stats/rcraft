* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in contrast_glm.recorded
* commands: contrast pwcompare
* (contrast and pwcompare after logit, probit and poisson: chi-squared tests, r., gw. contrasts, effects, pwcompare with the equation name)
* tolerance: 1e-6
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
logit yb i.g x1, nolog
contrast g
display "@ lg " %24.17g el(r(chi2),1,1) " " el(r(df),1,1) " " %24.17g el(r(p),1,1)
contrast r.g, effects
display "@ lgr " %24.17g el(r(table),3,1) " " %24.17g el(r(table),4,2)
pwcompare g, effects
display "@ lgp " %24.17g el(r(table_vs),1,3) " " %24.17g el(r(table_vs),3,3) " " %24.17g el(r(table_vs),6,3)
contrast gw.g
display "@ lgw " %24.17g el(r(table),1,1) " " %24.17g el(r(chi2),1,1)
quietly probit yb i.g x1
contrast g
display "@ pr " %24.17g el(r(chi2),1,1)
quietly poisson yb i.g x1
contrast g
display "@ po " %24.17g el(r(chi2),1,1)
