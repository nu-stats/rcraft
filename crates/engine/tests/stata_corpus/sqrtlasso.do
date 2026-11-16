* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in sqrtlasso.recorded
* commands: sqrtlasso lassoselect
* (sqrtlasso with selection none, bic, cv and plugin (both kinds); always-in variables, penaltywt(),
* noconstant; lassoselect and predict; Stata's errors, after which e() stays)
* (as with lasso linear, at the path's end the L1 and L2 norms and coefficients differ by up to 4e-8)
* tolerance: 1e-7
version 19
set linesize 255
clear
set obs 150
forvalues j = 1/8 {
  generate double x`j' = sin(_n * (`j' + 0.41) * 1.379) + 0.4 * cos(_n * `j' * 0.733)
}
replace x2 = 0.5 * x1 + x2
generate double y = 1.6 * x1 - 1.1 * x3 + 0.7 * x5 + 1.3 * sin(_n * 5.21) + 0.6 * cos(_n * 2.9)
quietly sqrtlasso y x1-x8, selection(none) grid(6)
lassoselect id = 4
display "@ none " e(n_lambda) " " e(ID_sel) " " %20.0g e(lambda_sel) " " %20.0g e(ll_sel)
display "@ none_path " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max) " " %20.0g e(lambda_gmax)
display "@ none_b " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, colsof(e(b)))
display "@ none_sel " ("`e(cmd)'" == "sqrtlasso") " " ("`e(title)'" == "Square-root lasso") " " ("`e(predict)'" == "sqrtlasso_p")
quietly sqrtlasso y x1-x8, selection(bic)
display "@ bic " e(n_lambda) " " e(ID_bic) " " %20.0g e(lambda_bic) " " %20.0g e(bic_min) " " e(k_nonzero_sel)
display "@ bic_path " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max) " " %20.0g e(ll_sel)
quietly sqrtlasso y x1-x8, selection(cv, folds(5)) rseed(12)
display "@ cv " e(n_lambda) " " e(ID_cv) " " e(ID_serule) " " %20.0g e(cvm_min) " " %20.0g e(cvm_serule)
display "@ cv_sel " %20.0g e(lambda_sel) " " e(k_nonzero_cv) " " %20.0g e(ll_sel)
quietly sqrtlasso y x1-x8, selection(plugin)
display "@ plugin " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel) " " ("`e(plugin_type)'" == "heteroskedastic")
quietly sqrtlasso y x1-x8, selection(plugin, homoskedastic)
display "@ plugin_homo " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel)
quietly sqrtlasso y (x7) x1-x6 x8, selection(plugin)
display "@ always " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel)
quietly sqrtlasso y (x7) x1-x6 x8, selection(bic)
display "@ always_bic " e(ID_bic) " " %20.0g e(lambda_gmax) " " %20.0g e(bic_min)
quietly sqrtlasso y x1-x8, selection(bic) penaltywt(2 1 1 1 1 1 1 1)
display "@ penaltywt " e(ID_bic) " " %20.0g e(bic_min) " " e(k_nonzero_sel)
quietly sqrtlasso y x1-x8, selection(none) grid(4) noconstant
lassoselect id = 3
display "@ nocons " %20.0g e(ll_sel) " " %20.0g e(L1_max)
quietly sqrtlasso y x1-x8, selection(cv, folds(5)) rseed(12)
capture lasso sqrt y x1-x8
display "@ err_sqrt " _rc
capture sqrtlasso y x1-x8, selection(adaptive)
display "@ err_adaptive " _rc
predict double p1
predict double p2, postselection
summarize p1, meanonly
display "@ predict " %20.0g r(mean) " " %20.0g r(max)
summarize p2, meanonly
display "@ predict_post " %20.0g r(mean) " " %20.0g r(max)
