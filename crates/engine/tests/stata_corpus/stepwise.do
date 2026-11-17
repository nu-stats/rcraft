* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in stepwise.recorded
* commands: stepwise
* (the search: backward, forward, both, hierarchical, lockterm1, lr, groups of terms, if, missing values;
* the model it ends with, listed in the order the terms were entered; Stata's errors as return codes)
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
generate double y = 0.5 * x1 - 0.4 * x2 + 0.3 * x3 + 0.1 * x4 + 1.4 * e
generate double w = 1 + 0.5 * sin(_n * 2.3)
generate double x6 = x5
replace x6 = . in 1/12
stepwise, pr(.2): regress y x1 x2 x3 x4 x5
display "@ back " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.05): regress y x1 x2 x3 x4 x5
display "@ back05 " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.1): regress y x1 x2 x3 x4 x5
display "@ fwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y x1 x2 x3 x4 x5
display "@ fwd2 " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) pe(.1): regress y x1 x2 x3 x4 x5
display "@ both " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3) pe(.15) forward: regress y x1 x2 x3 x4 x5
display "@ bothfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) hierarchical: regress y x1 x2 x3 x4 x5
display "@ hierback " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2) hierarchical: regress y x1 x2 x3 x4 x5
display "@ hierfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2) lockterm1: regress y x5 x4 x3 x2 x1
display "@ lockback " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.15) lockterm1: regress y x5 x4 x3 x2 x1
display "@ lockfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.25) lr: regress y x1 x2 x3 x4 x5
display "@ lrback " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2) lr: regress y x1 x2 x3 x4 x5
display "@ lrfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.3): regress y (x1 x2) x3 (x4 x5)
display "@ group " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y (x1 x2) x3 (x4 x5)
display "@ groupfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y x1 x2 x3 x4 x5 if _n > 40
display "@ ifs " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y x1 x2 x3 x4 x6
display "@ missing " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pe(.2): regress y x1 x2 x3 x4 x6
display "@ missingfwd " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2)
stepwise, pr(.2): regress y x1 x2 x3 x4 x5, level(90)
display "@ level " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss)
stepwise, pr(.2): regress y x1 x2 x3 x4 x5, beta
display "@ beta " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss)
quietly stepwise, pr(.2): regress y x1 x2 x3 x4 x5
display "@ quiet " e(N) " " colsof(e(b)) " " e(df_m) " " %20.0g e(rss)
predict double yhat
summarize yhat, meanonly
display "@ predicted " %20.0g r(mean)
capture stepwise: regress y x1 x2
display "@ err_none " _rc
capture stepwise, pr(.1) pe(.2): regress y x1 x2
display "@ err_order " _rc
capture stepwise, pr(.2) forward: regress y x1 x2
display "@ err_forward " _rc
capture stepwise, pr(1.5): regress y x1 x2
display "@ err_range " _rc
capture stepwise, pe(0): regress y x1 x2
display "@ err_range0 " _rc
capture stepwise, pr(.2) b: regress y x1 x2
display "@ err_b " _rc
capture stepwise, pe(.1) pr(.2) hierarchical: regress y x1 x2
display "@ err_hier " _rc
capture stepwise, pr(.2): regress y
display "@ err_novars " _rc
