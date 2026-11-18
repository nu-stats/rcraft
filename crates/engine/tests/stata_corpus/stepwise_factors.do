* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in stepwise_factors.recorded
* (stepwise with factor variables: each level of i.g and each column of a c.x#c.y interaction is a term of its
* own, the base left out; the order Stata leaves the terms in; groups, hierarchical, lockterm1, weights, vce())
* commands: stepwise
* tolerance: 1e-8
version 19
set linesize 255
local k = 12
clear
set obs 150
generate double u1 = sin(_n * (1.13 + `k' * 0.37))
generate double u2 = cos(_n * (2.07 + `k' * 0.19) + `k')
generate double u3 = sin(_n * (3.31 + `k' * 0.23) + 1 + `k')
generate double u4 = cos(_n * (4.17 + `k' * 0.41) + 2)
generate double u5 = sin(_n * (5.29 + `k' * 0.13) + 3)
generate double u6 = cos(_n * (6.61 + `k' * 0.29) + `k')
generate double e = sin(_n * (7.7 + `k' * 0.11)) + 0.5 * cos(_n * (1.93 + `k' * 0.07))
generate double x1 = u1 + 0.3 * u6
generate double x2 = 0.7 * u1 + 0.5 * u2
generate double x3 = 0.6 * u2 + 0.6 * u3 + 0.2 * u6
generate double x4 = 0.5 * u3 + 0.7 * u4
generate double x5 = u5 - 0.3 * u4
generate int g = 1 + mod(_n * 7, 4)
generate int g3 = 1 + mod(_n * 5, 3)
generate double y = 0.5 * x1 - 0.4 * x2 + 0.3 * x3 + 0.1 * x4 + 0.4 * (g == 2) - 0.5 * (g == 4) + 1.4 * e
generate double w = 1 + 0.5 * sin(_n * 2.3)
generate int fw = 1 + mod(_n * 3, 4)
generate int cl = 1 + mod(_n, 15)
generate byte z = (y + 0.6 * cos(_n * (9.1 + `k' * 0.17)) > 0.2)
generate int cnt = floor(exp(0.8 + 0.5 * x1 - 0.4 * x2 + 0.3 * x3) * (1.2 + 0.8 * sin(_n * 3.7)))
generate int g20 = mod(_n * 7, 20)
stepwise, pr(.2): regress y i.g x1 x2 x3 x4 x5
display "@ reg_back " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y i.g x1 x2 x3 x4 x5
display "@ reg_fwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lr: regress y i.g x1 x2 x3 x4 x5
display "@ reg_lr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2) lr: regress y i.g x1 x2 x3 x4 x5
display "@ reg_lrfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y ib3.g x1 x2 x3
display "@ reg_base3 " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y x1 x2 i.g3 x3 x4 x5
display "@ reg_mid " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y i.g3 x1 x2 x3 x4 x5
display "@ reg_g3 " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y (i.g x1) x2 x3 x4 x5
display "@ reg_group " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y (i.g3 x1) x2 x3 x4 x5
display "@ reg_groupfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) hierarchical: regress y x1 x2 x3 x4 i.g
display "@ reg_hier " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.5) hierarchical: regress y x1 x2 x3 i.g
display "@ reg_hierfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lockterm1: regress y x5 x4 i.g3 x3
display "@ reg_lock " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3) lockterm1: regress y x1 x2 i.g3 x3 x4 x5
display "@ reg_lockfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y c.x1##c.x2 x3 x4 x5
display "@ reg_cross " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y c.x1##c.x2 x3 x4 x5
display "@ reg_crossfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lr: regress y c.x1##c.x2 x3 x4 x5
display "@ reg_crosslr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y c.x1#c.x2 x3 x4 x5
display "@ reg_crossone " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y (c.x1##c.x2) x3 x4 x5
display "@ reg_crossgroup " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lockterm1: regress y c.x1##c.x2 x3 x4 x5
display "@ reg_crosslock " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) hierarchical: regress y x3 x4 c.x1##c.x2
display "@ reg_crosshier " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y i.g x1 c.x2##c.x3 x4
display "@ reg_mixed " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y i.g20 x1 x2
display "@ reg_g20 " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y i.g x1 x2 x3 [aweight = w]
display "@ reg_aw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y i.g x1 x2 x3 [pweight = w]
display "@ reg_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y i.g x1 x2 x3 [fweight = fw]
display "@ reg_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y i.g x1 x2 x3 [iweight = w]
display "@ reg_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y i.g x1 x2 x3, vce(cluster cl)
display "@ reg_cluster " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y i.g x1 x2 x3, robust
display "@ reg_robust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) level(90): regress y i.g x1 x2 x3
display "@ reg_level " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
capture stepwise, pr(.2) lockterm1: regress y i.g3 x1 x2 x3
display "@ err_lock_factor " _rc
capture stepwise, pe(.2) lockterm1: regress y ib2.g3 (x1 x2) x3
display "@ err_lock_factor_b " _rc
replace g = . in 1/10
stepwise, pr(.2): regress y i.g x1 x2
display "@ reg_missing " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y i.g x1 x2
display "@ reg_missingfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
