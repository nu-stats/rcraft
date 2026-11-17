* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in lasso_infer.recorded
* commands: dsregress poregress xporegress dslogit
* (double selection, partialing out and cross-fit for a linear outcome, double selection for a binary
* one; the plug-in, cv, bic and adaptive lassos, lasso(), sqrtlasso, always-in controls; robust, ols,
* hc3 and cluster variances; predict; Stata's errors)
* (all at 1e-8 but dslogit's variance, 1.4e-8 off: where Stata's logit fit stops)
* tolerance: 1e-7
version 19
set linesize 255
clear
set obs 160
forvalues j = 1/8 {
  generate double x`j' = sin(_n * (`j' + 0.41) * 1.379) + 0.4 * cos(_n * `j' * 0.733)
}
replace x2 = 0.5 * x1 + x2
generate double d1 = 0.5 * x1 + 0.3 * x2 + cos(_n * 1.91)
generate double d2 = -0.4 * x3 + sin(_n * 2.37)
generate double y = 1.6 * x1 - 1.1 * x3 + 0.7 * x5 + 0.7 * d1 + 1.3 * sin(_n * 5.21)
generate byte yb = (0.9 * x1 - 0.8 * x3 + 0.5 * d1 + 0.9 * sin(_n * 5.21) > 0)
generate byte g = mod(_n, 7)
quietly dsregress y d1, controls(x1-x8)
display "@ ds " %20.0g el(e(b), 1, 1) " " %20.0g el(e(V), 1, 1) " " %20.0g e(chi2) " " e(k_controls_sel)
display "@ ds_names " ("`e(controls_sel)'" == "x1 x3 x5") " " ("`e(cmd)'" == "dsregress") " " ("`e(vce)'" == "robust")
quietly dsregress y d1 d2, controls(x1-x8)
display "@ ds2 " %20.0g el(e(b), 1, 2) " " %20.0g el(e(V), 1, 2) " " %20.0g e(chi2) " " e(k_controls_sel)
quietly poregress y d1 d2, controls(x1-x8)
display "@ po " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2) " " %20.0g el(e(V), 2, 2) " " %20.0g e(chi2)
quietly xporegress y d1, controls(x1-x8) rseed(4) xfolds(5)
display "@ xpo " %20.0g el(e(b), 1, 1) " " %20.0g el(e(V), 1, 1) " " e(k_controls_sel) " " e(n_xfolds)
quietly xporegress y d1 d2, controls(x1-x8) rseed(4) xfolds(4) resample(2)
display "@ xpo2 " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 2) " " %20.0g el(e(V), 1, 2) " " e(n_resample)
quietly dsregress y d1, controls(x1-x8) vce(ols)
display "@ ols " %20.0g el(e(V), 1, 1)
quietly dsregress y d1, controls(x1-x8) vce(hc3)
display "@ hc3 " %20.0g el(e(V), 1, 1)
quietly dsregress y d1, controls(x1-x8) vce(cluster g)
display "@ cluster " %20.0g el(e(V), 1, 1) " " %20.0g e(chi2)
quietly dsregress y d1, controls(x1-x8) selection(cv) rseed(9)
display "@ cv " %20.0g el(e(b), 1, 1) " " e(k_controls_sel)
quietly dsregress y d1, controls(x1-x8) selection(bic)
display "@ bic " %20.0g el(e(b), 1, 1) " " e(k_controls_sel)
quietly dsregress y d1, controls(x1-x8) selection(adaptive) rseed(9)
display "@ adaptive " %20.0g el(e(b), 1, 1) " " e(k_controls_sel)
quietly dsregress y d1, controls(x1-x8) lasso(d1, selection(cv)) rseed(9)
display "@ lasso_d1 " %20.0g el(e(b), 1, 1) " " e(k_controls_sel)
quietly dsregress y d1, controls(x1-x8) sqrtlasso
display "@ sqrt " %20.0g el(e(b), 1, 1) " " e(k_controls_sel)
quietly dsregress y d1, controls((x7) x1-x6 x8)
display "@ always " %20.0g el(e(b), 1, 1) " " e(k_controls_sel)
quietly dslogit yb d1, controls(x1-x8)
display "@ dslogit " %20.0g el(e(b), 1, 1) " " %20.0g el(e(V), 1, 1) " " %20.0g e(chi2) " " e(k_controls_sel)
quietly dslogit yb d1 d2, controls(x1-x8) vce(cluster g)
display "@ dslogit2 " %20.0g el(e(b), 1, 2) " " %20.0g el(e(V), 2, 2)
quietly dsregress y d1 d2, controls(x1-x8)
predict double p1
summarize p1, meanonly
display "@ predict " %20.0g r(mean) " " %20.0g r(max)
capture dsregress y d1
display "@ err_controls " _rc
capture dsregress y, controls(x1-x8)
display "@ err_vars " _rc
capture dsregress y d1, controls(d1 x1-x8)
display "@ err_both " _rc
capture dslogit y d1, controls(x1-x8)
display "@ err_outcome " _rc
capture xporegress y d1, controls(x1-x8) resample(0)
display "@ err_resample " _rc
quietly dsregress y d1, controls(x1-x8)
