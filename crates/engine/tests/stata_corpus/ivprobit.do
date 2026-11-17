* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in ivprobit.recorded
* commands: ivprobit
* (one endogenous regressor: the ML fit with oim, robust and clustered variances, first, level(),
* Newey's two-step estimator, replay, predict xb, pr and stdp; Stata's errors)
* tolerance: 1e-7
version 19
set linesize 255
clear
set obs 300
generate double x = round(sin(_n * 1.13) + 0.3 * cos(_n * 0.71), .01)
generate double z1 = round(cos(_n * 2.07) + 0.2 * sin(_n * 0.37), .01)
generate double z2 = round(sin(_n * 3.31 + 1), .01)
generate double e = sin(_n * 5.17) + 0.5 * cos(_n * 1.93)
generate double y1 = round(0.6 * z1 + 0.5 * z2 + 0.3 * x + 0.7 * e + 0.4 * sin(_n * 0.53), .001)
generate byte y = (0.5 * y1 + 0.4 * x + 0.8 * e + cos(_n * 7.7) > 0.1)
generate int g = ceil(_n / 15)
quietly ivprobit y x (y1 = z1 z2)
display "@ ml " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2) " " %20.0g el(e(b), 1, 8) " " %20.0g el(e(b), 1, 9) " " %20.0g e(ll)
display "@ ml_v " %20.0g el(e(V), 1, 1) " " %20.0g el(e(V), 2, 1) " " %20.0g el(e(V), 8, 8) " " %20.0g e(chi2) " " %20.0g e(chi2_exog) " " e(ic)
quietly ivprobit y x (y1 = z1 z2), vce(robust)
display "@ robust " %20.0g el(e(V), 1, 1) " " %20.0g el(e(V), 3, 3) " " %20.0g e(chi2)
quietly ivprobit y x (y1 = z1 z2), vce(cluster g)
display "@ cluster " %20.0g el(e(V), 1, 1) " " %20.0g el(e(V), 9, 9) " " %20.0g e(chi2_exog) " " e(N_clust)
quietly ivprobit y (y1 = z1 z2 x)
display "@ noexog " %20.0g el(e(b), 1, 1) " " %20.0g el(e(V), 1, 1) " " e(df_m)
quietly ivprobit y x (y1 = z1 z2), twostep
display "@ twostep " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 3) " " %20.0g el(e(V), 1, 1) " " %20.0g e(chi2) " " %20.0g e(chi2_exog)
quietly ivprobit y x (y1 = z1 z2) if g < 15
display "@ subset " %20.0g el(e(b), 1, 1) " " e(N)
quietly ivprobit y x (y1 = z1 z2)
predict double p1
predict double p2, pr
predict double p3, stdp
summarize p1, meanonly
display "@ p_xb " %20.0g r(mean)
summarize p2, meanonly
display "@ p_pr " %20.0g r(mean)
summarize p3, meanonly
display "@ p_stdp " %20.0g r(mean)
capture predict double p9, scores
display "@ err_scores " _rc
capture predict double p9, pr xb
display "@ err_two " _rc
capture ivprobit y x (y1 = )
display "@ err_noinst " _rc
capture ivprobit y x (y1 = x)
display "@ err_onlyexog " _rc
capture ivprobit y x (x = z1)
display "@ err_dropendog " _rc
