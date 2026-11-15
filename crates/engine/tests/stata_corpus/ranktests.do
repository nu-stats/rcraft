* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in ranktests.recorded
* commands: ranksum signrank signtest kwallis
* (ranksum and signrank with ties, zeros and their exact p-values, signtest, kwallis with and without the ties' correction; to 6 digits: kwallis's chi2 is Stata's to 2e-7)
* tolerance: 1e-6
version 19
clear
set linesize 255
set seed 105
set obs 60
generate g = runiform() < .5
generate k = 1 + floor(3*runiform())
generate x = round(rnormal() + .5*g, .1)
generate w = round(x + rnormal(), .1)
ranksum x, by(g)
display "@ rs " %24.17g r(z) " " %24.17g r(Var_a) " " %24.17g r(p) " " %24.17g r(p_exact) " " %24.17g r(p_u_exact) " " r(sum_obs) " " r(N_1)
signrank x = w
display "@ sr " %24.17g r(z) " " %24.17g r(Var_a) " " %24.17g r(p_exact) " " %24.17g r(p_l_exact) " " r(N_tie) " " r(sum_pos)
signrank x = 0
display "@ sr0 " %24.17g r(z) " " %24.17g r(p_exact)
signtest x = w
display "@ st " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(p_u) " " r(N_pos)
kwallis x, by(k)
display "@ kw " %24.17g r(chi2) " " %24.17g r(chi2_adj) " " %24.17g r(p_adj) " " r(df)
