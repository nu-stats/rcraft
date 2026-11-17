* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in stepwise_glm.recorded
* Stata stops iterating at its tolerance, so the likelihood models agree to about 7 digits
* commands: stepwise
* (the search with logit, probit and poisson: Wald and likelihood-ratio tests, backward and forward)
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
generate double y = 0.9 * x1 - 0.8 * x2 + 0.5 * x3 + 0.1 * x4 + 1.4 * e
generate byte z = (y + 0.6 * cos(_n * (9.1 + `k' * 0.17)) > 0.2)
generate int cnt = floor(exp(0.8 + 0.5 * x1 - 0.4 * x2 + 0.3 * x3) * (1.2 + 0.8 * sin(_n * 3.7)))
stepwise, pr(.2): logit z x1 x2 x3 x4 x5
display "@ logit_back " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lr: logit z x1 x2 x3 x4 x5
display "@ logit_lr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): logit z x1 x2 x3 x4 x5
display "@ logit_fwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3) lr: logit z x1 x2 x3 x4 x5
display "@ logit_fwdlr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): probit z x1 x2 x3 x4 x5
display "@ probit_back " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): probit z x1 x2 x3 x4 x5
display "@ probit_fwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lr: probit z x1 x2 x3 x4 x5
display "@ probit_lr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): poisson cnt x1 x2 x3 x4 x5
display "@ pois_back " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.3): poisson cnt x1 x2 x3 x4 x5
display "@ pois_fwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) lr: poisson cnt x1 x2 x3 x4 x5
display "@ pois_lr " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(ll) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
