* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in stepwise_weights.recorded
* (stepwise with weights, vce(), robust and noconstant: regress, logit, probit and poisson; Wald and
* likelihood-ratio tests, backward and forward, hierarchical, and Stata's errors as return codes)
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
stepwise, pr(.2): regress y x1 x2 x3 x4 x5 [aweight = w]
display "@ reg_back_aw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y x1 x2 x3 x4 x5 [aweight = w]
display "@ reg_fwd_aw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) hierarchical: regress y x1 x2 x3 x4 x5 [aweight = w]
display "@ reg_hier_aw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y x1 x2 x3 x4 x5 [pweight = w]
display "@ reg_back_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y x1 x2 x3 x4 x5 [pweight = w]
display "@ reg_fwd_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) hierarchical: regress y x1 x2 x3 x4 x5 [pweight = w]
display "@ reg_hier_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y x1 x2 x3 x4 x5 [fweight = fw]
display "@ reg_back_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y x1 x2 x3 x4 x5 [fweight = fw]
display "@ reg_fwd_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) hierarchical: regress y x1 x2 x3 x4 x5 [fweight = fw]
display "@ reg_hier_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y x1 x2 x3 x4 x5 [iweight = w]
display "@ reg_back_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y x1 x2 x3 x4 x5 [iweight = w]
display "@ reg_fwd_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) hierarchical: regress y x1 x2 x3 x4 x5 [iweight = w]
display "@ reg_hier_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.25) lr: regress y x1 x2 x3 x4 x5 [aweight = w]
display "@ reg_lr_aw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2) lr: regress y x1 x2 x3 x4 x5 [aweight = w]
display "@ reg_lrfwd_aw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.25) lr: regress y x1 x2 x3 x4 x5 [fweight = fw]
display "@ reg_lr_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2) lr: regress y x1 x2 x3 x4 x5 [fweight = fw]
display "@ reg_lrfwd_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.25) lr: regress y x1 x2 x3 x4 x5 [iweight = w]
display "@ reg_lr_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2) lr: regress y x1 x2 x3 x4 x5 [iweight = w]
display "@ reg_lrfwd_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5, robust
display "@ reg_robust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5, vce(robust)
display "@ reg_vcerobust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5, vce(cluster cl)
display "@ reg_cluster " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): regress y x1 x2 x3 x4 x5, vce(cluster cl)
display "@ reg_clusterfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: regress y x1 x2 x3 x4 x5, vce(cluster cl)
display "@ reg_clusterlr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5, vce(ols)
display "@ reg_vceols " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5, noconstant
display "@ reg_nocons " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): regress y x1 x2 x3 x4 x5, noconstant
display "@ reg_noconsfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: regress y x1 x2 x3 x4 x5, noconstant
display "@ reg_noconslr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5, robust noconstant
display "@ reg_noconsrobust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5 [aweight = w], robust
display "@ reg_robustaw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5 [aweight = w], vce(cluster cl)
display "@ reg_clusteraw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5 [fweight = fw], vce(cluster cl)
display "@ reg_clusterfw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y x1 x2 x3 x4 x5 [fweight = fw], robust
display "@ reg_robustfw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) level(90): regress y x1 x2 x3 x4 x5
display "@ reg_level " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
capture stepwise, pr(.2) lr: regress y x1 x2 x3 [pweight = w]
display "@ err_lr_pweight " _rc
capture stepwise, pr(.2) lr: regress y x1 x2 x3, robust
display "@ err_lr_robust " _rc
capture stepwise, pr(.2) lr: logit z x1 x2 x3, vce(robust)
display "@ err_lr_vcerobust " _rc
capture stepwise, pr(.2): logit z x1 x2 x3 [aweight = w]
display "@ err_logit_aweight " _rc
capture stepwise, pr(.2): poisson cnt x1 x2 x3 [aweight = w]
display "@ err_pois_aweight " _rc
capture stepwise, pr(.2): logit z x1 x2 x3, vce(ols)
display "@ err_logit_ols " _rc
capture stepwise, pr(.2): logit z x1 x2 x3, vce(hc3)
display "@ err_logit_hc3 " _rc
capture stepwise, pr(.2): regress y x1 x2 x3, vce(oim)
display "@ err_reg_oim " _rc
capture stepwise, pr(.2): regress y x1 x2 x3, vce(opg)
display "@ err_reg_opg " _rc
capture stepwise, pr(.2) level(90): regress y x1 x2 x3, level(80)
display "@ err_level_both " _rc
capture stepwise, pr(.2) level(190): regress y x1 x2 x3
display "@ err_level_range " _rc
