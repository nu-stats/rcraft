* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in coefplot2.recorded
* commands: coefplot
* (coefplot, Ben Jann's SSC package: the rows its gen() leaves with levels(95 90), with asequation swapnames eqrename(), and with asequation alone)
* tolerance: 1e-7
version 19
clear
set seed 3
set obs 200
generate Z = runiform() < .5
generate w = rnormal()
generate y1 = .3*Z + .2*w + rnormal()
generate y2 = .1*Z - .1*w + rnormal()
regress y1 Z w
estimates store INTENT
regress y2 Z w
estimates store COMPLETION
coefplot INTENT, levels(95 90) gen(a_) nodraw
display "@ a1 " a_at[1] " " a_b[1] " " a_ll1[1] " " a_ul1[1] " " a_ll2[1] " " a_ul2[1]
display "@ a2 " a_at[3] " " a_ll2[3] " " a_ul2[3]
coefplot (INTENT) (COMPLETION), drop(_cons) eqrename(INTENT = "Initiation" COMPLETION = "Receipt") asequation swapnames levels(95 90) gen(c_) nodraw
display "@ c1 " c_at[1] " " c_plot[1] " " c_b[1] " " c_ll2[1] " " c_ul2[1]
display "@ c2 " c_at[2] " " c_plot[2] " " c_b[2] " " c_ll2[2]
display "@ c3 " c_at[3] " " c_plot[3] " " c_b[3]
display "@ c4 " c_at[4] " " c_plot[4] " " c_b[4] " " c_ul2[4]
coefplot (INTENT) (COMPLETION), keep(Z) asequation gen(d_) nodraw
display "@ d1 " d_at[1] " " d_at[2] " " d_b[2]
