* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in lasso_glm.recorded
* commands: lasso lassoselect
* (lasso logit, probit and poisson with selection none, bic, cv, adaptive and plugin; always-in
* variables, noconstant, offset() and exposure(); lassoselect and predict; Stata's errors)
* (Stata's descent stops short of the optimum; RCraft solves to it, which Stata reaches at tolerance(1e-12):
* at small lambdas coefficients and predictions differ by up to 1e-7, extreme probabilities by 4e-7)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 160
forvalues j = 1/8 {
  generate double x`j' = sin(_n * (`j' + 0.41) * 1.379) + 0.4 * cos(_n * `j' * 0.733)
}
replace x2 = 0.5 * x1 + x2
generate double eta = 0.9 * x1 - 0.8 * x3 + 0.5 * x5 + 0.9 * sin(_n * 5.21)
generate byte yb = (eta + 0.8 * cos(_n * 2.9) > 0)
generate int yc = floor(exp(0.5 * eta + 0.4) * (1 + 0.6 * sin(_n * 3.7)))
generate double ex = exp(0.3 * cos(_n * 1.3))
generate double y = 1.6 * x1 - 1.1 * x3 + 0.7 * x5 + 1.3 * sin(_n * 5.21)
quietly lasso logit yb x1-x8, selection(none) grid(6)
lassoselect id = 4
display "@ lnone " e(n_lambda) " " %20.0g e(lambda_sel) " " %20.0g e(ll_sel) " " e(k_nonzero_sel)
display "@ lnone_path " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max) " " %20.0g e(lambda_gmax)
display "@ lnone_b " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, colsof(e(b))) " " %20.0g el(e(b_standardized), 1, colsof(e(b)))
display "@ lnone_sel " ("`e(model)'" == "logit") " " ("`e(sel_criterion)'" == "user")
quietly lasso logit yb x1-x8, selection(bic)
display "@ lbic " e(n_lambda) " " e(ID_bic) " " %20.0g e(lambda_bic) " " %20.0g e(bic_min) " " e(k_nonzero_sel)
display "@ lbic_path " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max) " " %20.0g e(ll_sel)
quietly lasso logit yb x1-x8, selection(cv, folds(5)) rseed(12)
display "@ lcv " e(n_lambda) " " e(ID_cv) " " e(ID_serule) " " %20.0g e(cvm_min) " " %20.0g e(cvm_serule)
display "@ lcv_sel " %20.0g e(lambda_sel) " " e(k_nonzero_cv) " " %20.0g e(ll_sel)
quietly lasso logit yb x1-x8, selection(adaptive) rseed(5)
display "@ ladaptive " e(n_lambda) " " e(ID_cv) " " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel)
quietly lasso logit yb x1-x8, selection(plugin)
display "@ lplugin " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel) " " ("`e(plugin_type)'" == "homoskedastic")
quietly lasso logit yb x1-x8, selection(plugin, heteroskedastic)
display "@ lplugin_het " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel)
quietly lasso logit yb (x7) x1-x6 x8, selection(bic)
display "@ lalways " e(ID_bic) " " %20.0g e(bic_min) " " e(k_nonzero_sel) " " %20.0g e(ll_sel)
quietly lasso logit yb x1-x8, selection(bic) noconstant
display "@ lnocons " e(ID_bic) " " %20.0g e(bic_min) " " %20.0g e(devratio_max) " " %20.0g e(ll_sel)
quietly lasso logit yb x1-x8, selection(bic) offset(x6)
display "@ loffset " e(ID_bic) " " %20.0g e(bic_min) " " %20.0g e(ll_sel) " " ("`e(offset)'" == "x6")
quietly lasso logit yb x1-x8, selection(cv, folds(5)) rseed(12)
predict double p1
predict double p2, xb
predict double p3, postselection
summarize p1, meanonly
display "@ lpredict " %20.0g r(mean) " " %20.0g r(min) " " %20.0g r(max)
summarize p2, meanonly
display "@ lpredict_xb " %20.0g r(mean) " " %20.0g r(max)
summarize p3, meanonly
display "@ lpredict_post " %20.0g r(mean) " " %20.0g r(max)
quietly lasso probit yb x1-x8, selection(none) grid(6)
lassoselect id = 4
display "@ pnone " %20.0g e(ll_sel) " " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max)
quietly lasso probit yb x1-x8, selection(bic)
display "@ pbic " e(n_lambda) " " e(ID_bic) " " %20.0g e(bic_min) " " %20.0g e(ll_sel)
quietly lasso probit yb x1-x8, selection(cv, folds(5)) rseed(12)
display "@ pcv " e(n_lambda) " " e(ID_cv) " " %20.0g e(cvm_min) " " %20.0g e(ll_sel)
quietly lasso probit yb x1-x8, selection(plugin)
display "@ pplugin " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel)
predict double p4
summarize p4, meanonly
display "@ ppredict " %20.0g r(mean) " " %20.0g r(max)
quietly lasso poisson yc x1-x8, selection(none) grid(6)
lassoselect id = 4
display "@ qnone " %20.0g e(ll_sel) " " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max)
quietly lasso poisson yc x1-x8, selection(bic)
display "@ qbic " e(n_lambda) " " e(ID_bic) " " %20.0g e(bic_min) " " %20.0g e(ll_sel)
quietly lasso poisson yc x1-x8, selection(cv, folds(5)) rseed(12)
display "@ qcv " e(n_lambda) " " e(ID_cv) " " %20.0g e(cvm_min) " " %20.0g e(ll_sel)
quietly lasso poisson yc x1-x8, selection(plugin)
display "@ qplugin " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel)
quietly lasso poisson yc x1-x8, selection(bic) exposure(ex)
display "@ qexposure " e(ID_bic) " " %20.0g e(bic_min) " " %20.0g e(ll_sel) " " ("`e(exposure)'" == "ex")
quietly lasso poisson yc x1-x8, selection(cv, folds(5)) rseed(12)
predict double p5
summarize p5, meanonly
display "@ qpredict " %20.0g r(mean) " " %20.0g r(max)
capture lasso logit y x1-x8
display "@ err_outcome " _rc
generate double yneg = -yc
capture lasso poisson yneg x1-x8
display "@ err_negative " _rc
capture lasso logit yb x1-x8, exposure(ex)
display "@ err_exposure " _rc
generate double exneg = ex - 1
capture lasso poisson yc x1-x8, exposure(exneg)
display "@ err_exposure0 " _rc
