* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in prtest.recorded
* commands: prtest
* (prtest var, by(g): the two-sample test of proportions' r(), with if, level() and a string group; Stata's r(450) for other than two groups)
* tolerance: 1e-9
version 19
clear
set seed 9
set obs 300
generate g = runiform() < .4
generate x = runiform() < .3 + .2*g
replace x = . in 1/5
generate h = cond(_n < 100, -1, 1)
prtest x, by(g)
display "@ a " r(N1) " " r(N2) " " r(P1) " " r(P2) " " r(P_diff) " " r(se1) " " r(se2)
display "@ b " r(se_diff0) " " r(se_diff) " " r(lb1) " " r(ub1) " " r(lb2) " " r(ub2)
display "@ c " r(lb_diff) " " r(ub_diff) " " r(z) " " r(p_l) " " r(p) " " r(p_u) " " r(level)
prtest x if _n > 50, by(h) level(90)
display "@ d " r(N1) " " r(N2) " " r(z) " " r(lb_diff) " " r(ub_diff) " " r(level)
generate str5 s = cond(g, "aa", "bb")
quietly prtest x, by(s)
display "@ e " r(P1) " " r(P2) " " r(z)
capture prtest x if h == 1, by(h)
display "@ f " _rc
