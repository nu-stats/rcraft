* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in stepwise_weights_glm.recorded
* (stepwise with weights, vce(), robust and noconstant: regress, logit, probit and poisson; Wald and
* likelihood-ratio tests, backward and forward, hierarchical, and Stata's errors as return codes)
* (the likelihood models: Stata stops iterating at its tolerance, so the results agree to about 7 digits)
* commands: stepwise
* tolerance: 1e-6
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
stepwise, pr(.3): logit z x1 x2 x3 x4 x5 [pweight = w]
display "@ logit_back_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): logit z x1 x2 x3 x4 x5 [pweight = w]
display "@ logit_fwd_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): logit z x1 x2 x3 x4 x5 [fweight = fw]
display "@ logit_back_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): logit z x1 x2 x3 x4 x5 [fweight = fw]
display "@ logit_fwd_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): logit z x1 x2 x3 x4 x5 [iweight = w]
display "@ logit_back_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): logit z x1 x2 x3 x4 x5 [iweight = w]
display "@ logit_fwd_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: logit z x1 x2 x3 x4 x5 [fweight = fw]
display "@ logit_lr_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: logit z x1 x2 x3 x4 x5 [iweight = w]
display "@ logit_lr_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): probit z x1 x2 x3 x4 x5 [pweight = w]
display "@ probit_back_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): probit z x1 x2 x3 x4 x5 [pweight = w]
display "@ probit_fwd_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): probit z x1 x2 x3 x4 x5 [fweight = fw]
display "@ probit_back_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): probit z x1 x2 x3 x4 x5 [fweight = fw]
display "@ probit_fwd_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): probit z x1 x2 x3 x4 x5 [iweight = w]
display "@ probit_back_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): probit z x1 x2 x3 x4 x5 [iweight = w]
display "@ probit_fwd_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: probit z x1 x2 x3 x4 x5 [fweight = fw]
display "@ probit_lr_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: probit z x1 x2 x3 x4 x5 [iweight = w]
display "@ probit_lr_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): poisson cnt x1 x2 x3 x4 x5 [pweight = w]
display "@ pois_back_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): poisson cnt x1 x2 x3 x4 x5 [pweight = w]
display "@ pois_fwd_pw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): poisson cnt x1 x2 x3 x4 x5 [fweight = fw]
display "@ pois_back_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): poisson cnt x1 x2 x3 x4 x5 [fweight = fw]
display "@ pois_fwd_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): poisson cnt x1 x2 x3 x4 x5 [iweight = w]
display "@ pois_back_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): poisson cnt x1 x2 x3 x4 x5 [iweight = w]
display "@ pois_fwd_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: poisson cnt x1 x2 x3 x4 x5 [fweight = fw]
display "@ pois_lr_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: poisson cnt x1 x2 x3 x4 x5 [iweight = w]
display "@ pois_lr_iw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): logit z x1 x2 x3 x4 x5, robust
display "@ logit_robust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): logit z x1 x2 x3 x4 x5, vce(robust)
display "@ logit_vcerobust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): logit z x1 x2 x3 x4 x5, vce(cluster cl)
display "@ logit_cluster " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): logit z x1 x2 x3 x4 x5, vce(cluster cl)
display "@ logit_clusterfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: logit z x1 x2 x3 x4 x5, vce(cluster cl)
display "@ logit_clusterlr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): logit z x1 x2 x3 x4 x5, vce(oim)
display "@ logit_oim " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): logit z x1 x2 x3 x4 x5, noconstant
display "@ logit_nocons " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): logit z x1 x2 x3 x4 x5, noconstant
display "@ logit_noconsfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: logit z x1 x2 x3 x4 x5, noconstant
display "@ logit_noconslr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): probit z x1 x2 x3 x4 x5, robust
display "@ probit_robust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): probit z x1 x2 x3 x4 x5, vce(robust)
display "@ probit_vcerobust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): probit z x1 x2 x3 x4 x5, vce(cluster cl)
display "@ probit_cluster " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): probit z x1 x2 x3 x4 x5, vce(cluster cl)
display "@ probit_clusterfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: probit z x1 x2 x3 x4 x5, vce(cluster cl)
display "@ probit_clusterlr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): probit z x1 x2 x3 x4 x5, vce(oim)
display "@ probit_oim " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): probit z x1 x2 x3 x4 x5, noconstant
display "@ probit_nocons " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): probit z x1 x2 x3 x4 x5, noconstant
display "@ probit_noconsfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: probit z x1 x2 x3 x4 x5, noconstant
display "@ probit_noconslr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): poisson cnt x1 x2 x3 x4 x5, robust
display "@ pois_robust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): poisson cnt x1 x2 x3 x4 x5, vce(robust)
display "@ pois_vcerobust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): poisson cnt x1 x2 x3 x4 x5, vce(cluster cl)
display "@ pois_cluster " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): poisson cnt x1 x2 x3 x4 x5, vce(cluster cl)
display "@ pois_clusterfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: poisson cnt x1 x2 x3 x4 x5, vce(cluster cl)
display "@ pois_clusterlr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): poisson cnt x1 x2 x3 x4 x5, vce(oim)
display "@ pois_oim " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): poisson cnt x1 x2 x3 x4 x5, noconstant
display "@ pois_nocons " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): poisson cnt x1 x2 x3 x4 x5, noconstant
display "@ pois_noconsfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: poisson cnt x1 x2 x3 x4 x5, noconstant
display "@ pois_noconslr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
