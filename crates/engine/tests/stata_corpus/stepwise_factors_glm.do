* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in stepwise_factors_glm.recorded
* (stepwise with factor variables: each level of i.g and each column of a c.x#c.y interaction is a term of its
* own, the base left out; the order Stata leaves the terms in; groups, hierarchical, lockterm1, weights, vce())
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
generate int g20 = mod(_n * 7, 20)
stepwise, pr(.2): logit z i.g x1 x2 x3 x4 x5
display "@ logit_back " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): logit z i.g x1 x2 x3 x4 x5
display "@ logit_fwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lr: logit z i.g x1 x2 x3 x4 x5
display "@ logit_lr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): probit z i.g x1 x2 x3 x4 x5
display "@ probit_back " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): probit z i.g x1 x2 x3 x4 x5
display "@ probit_fwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lr: probit z i.g x1 x2 x3 x4 x5
display "@ probit_lr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): poisson cnt i.g x1 x2 x3 x4 x5
display "@ pois_back " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): poisson cnt i.g x1 x2 x3 x4 x5
display "@ pois_fwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lr: poisson cnt i.g x1 x2 x3 x4 x5
display "@ pois_lr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): logit z i.g x1 x2 x3 [fweight = fw]
display "@ logit_fw " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): logit z i.g x1 x2 x3, robust
display "@ logit_robust " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): logit z i.g x1 x2 x3, vce(cluster cl)
display "@ logit_cluster " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
