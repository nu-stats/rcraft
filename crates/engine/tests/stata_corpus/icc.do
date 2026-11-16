* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in icc.recorded
* commands: icc
* (one-way and two-way random-effects and two-way mixed-effects intraclass correlations, absolute agreement and consistency, McGraw and Wong's intervals and F tests, testvalue(), level(); targets left out; Stata's errors)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 72
generate int target = 1 + mod(_n - 1, 12)
generate int judge = 1 + floor((_n - 1) / 12)
generate double score = 10 + target / 3 + judge / 5 + mod(_n * 37, 23) / 11
quietly icc score target judge
display "@ random " %24.17g r(icc_i) " " %24.17g r(icc_i_lb) " " %24.17g r(icc_i_ub) " " %24.17g r(icc_avg) " " %24.17g r(icc_avg_lb) " " %24.17g r(icc_avg_ub) " " %24.17g r(icc_i_F) " " r(icc_i_df2) " `r(type)'"
quietly icc score target judge, consistency
display "@ consistency " %24.17g r(icc_i) " " %24.17g r(icc_i_lb) " " %24.17g r(icc_avg_ub) " `r(model)'"
quietly icc score target judge, mixed
display "@ mixed " %24.17g r(icc_i) " " %24.17g r(icc_avg_lb) " `r(model)' `r(type)'"
quietly icc score target
display "@ oneway " %24.17g r(icc_i) " " %24.17g r(icc_i_lb) " " %24.17g r(icc_avg) " " %24.17g r(icc_avg_ub) " " %24.17g r(icc_i_F) " " r(icc_i_df2)
quietly icc score target judge, testvalue(.75)
display "@ test " %24.17g r(icc_i_F) " " %24.17g r(icc_i_df2) " " %24.17g r(icc_i_p) " " %24.17g r(icc_avg_F) " " %24.17g r(icc_avg_df2) " " %24.17g r(icc_avg_p)
quietly icc score target, testvalue(.2) level(90)
display "@ test oneway " %24.17g r(icc_i_F) " " %24.17g r(icc_avg_F) " " %24.17g r(icc_i_lb) " " r(level)
quietly icc score target judge, consistency testvalue(.1)
display "@ test consistency " %24.17g r(icc_i_F) " " %24.17g r(icc_avg_F) " " %24.17g r(icc_avg_p)
replace score = . in 5
quietly icc score target judge
display "@ omitted " r(N_target) " " %24.17g r(icc_i)
quietly icc score target
display "@ omitted oneway " r(N_target) " " r(N_rater) " " %24.17g r(icc_avg)
capture icc score
display "@ rc no target " _rc
capture icc score target judge, consistency absolute
display "@ rc both " _rc
capture icc score target, consistency
display "@ rc oneway consistency " _rc
quietly icc score target, absolute
display "@ oneway absolute " %24.17g r(icc_i) " `r(type)'"
capture icc score target judge [fw=judge]
display "@ rc weights " _rc
capture icc score target, mixed
display "@ rc oneway mixed " _rc
replace score = . in 9
quietly icc score target judge
display "@ omitted two " r(N_target) " " %24.17g r(icc_avg)
quietly icc score target
display "@ omitted two oneway " r(N_target) " " %24.17g r(icc_i)
capture icc score target judge if target == 1
display "@ rc insufficient targets " _rc
capture icc score target judge if judge == 2
display "@ rc insufficient raters " _rc
capture icc score target if score > 100
display "@ rc no observations " _rc
replace judge = 2 in 1
capture icc score target judge
display "@ rc repeated " _rc
