* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in ttest.recorded
* commands: ttest ttesti
* (welch with Welch's degrees of freedom, unequal, unpaired, by() with value labels, ttesti for one and two samples; Stata's errors)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 50
generate byte g = _n > 20
generate double a = mod(_n * 37, 23) / 7 + g * (1 + mod(_n * 13, 9) / 3)
generate double b = a + mod(_n * 11, 5) / 4 - .4
label define gg 0 "control" 1 "treated group"
label values g gg
quietly ttest a, by(g) welch
display "@ welch " %24.17g r(df_t) " " %24.17g r(t) " " %24.17g r(p) " " %24.17g r(lb_diff) " " %24.17g r(ub_diff)
quietly ttest a, by(g) unequal
display "@ unequal " %24.17g r(df_t) " " %24.17g r(p)
quietly ttest a == b, unpaired welch
display "@ unpaired welch " %24.17g r(df_t) " " %24.17g r(se)
quietly ttest a == b
display "@ paired " %24.17g r(t) " " %24.17g r(sd_diff)
quietly ttesti 20 1.2 1.1 30 2.3 2.4, welch
display "@ ttesti welch " %24.17g r(df_t) " " %24.17g r(sd) " " %24.17g r(mu_combined) " " %24.17g r(lb_combined)
quietly ttesti 25 1.5 .8 1, level(90)
display "@ ttesti one " %24.17g r(t) " " %24.17g r(lb_1) " " %24.17g r(p_u)
quietly ttesti 25 1.5 .8 30 1.2 .9
display "@ ttesti two " %24.17g r(t) " " %24.17g r(se) " " %24.17g r(p)
capture ttest a == 0, welch
display "@ rc one welch " _rc
capture ttest a == b, unequal welch
display "@ rc paired unequal " _rc
capture ttest a if g == 1, by(g)
display "@ rc one group " _rc
