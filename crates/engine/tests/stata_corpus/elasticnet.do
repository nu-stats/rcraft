* RCraft's Stata corpus (P2C.12 continued): made-up data; Stata 19's results are in elasticnet.recorded
* commands: elasticnet lassoselect
* (elasticnet linear with one alpha and several: selection none, bic and cv, the shared grid,
* the one-s.e. rule, always-in variables, penaltywt(), noconstant; lassoselect and predict; Stata's errors)
* (Stata's descent stops short of the optimum: at the path's end its L1 and L2 norms can differ in the 8th digit)
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
quietly elasticnet linear y x1-x8, selection(none) alpha(.5) grid(6)
lassoselect id = 4
display "@ none " e(n_lambda) " " e(ID_sel) " " %20.0g e(lambda_sel) " " %20.0g e(ll_sel) " " e(alpha_sel)
display "@ none_path " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max) " " %20.0g e(lambda_gmax)
display "@ none_b " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, colsof(e(b)))
display "@ none_sel " ("`e(cmd)'" == "elasticnet") " " ("`e(title)'" == "Elastic net") " " ("`e(crossgrid)'" == "augmented")
quietly elasticnet linear y x1-x8, selection(bic) alpha(.5)
display "@ bic " e(n_lambda) " " e(ID_bic) " " %20.0g e(lambda_bic) " " %20.0g e(bic_min) " " e(k_nonzero_sel) " " e(alpha_bic)
display "@ bic_path " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max) " " %20.0g e(ll_sel)
quietly elasticnet linear y x1-x8, selection(bic) alpha(.25 .5 1)
display "@ bic3 " e(n_lambda) " " e(ID_bic) " " %20.0g e(lambda_bic) " " %20.0g e(bic_min) " " e(alpha_sel) " " e(stop)
display "@ bic3_grid " %20.0g e(lambda_gmax) " " %20.0g e(lambda_gmin) " " %20.0g e(lambda_last) " " %20.0g e(L1_max)
quietly elasticnet linear y x1-x8, selection(cv, folds(5)) alpha(.5) rseed(12)
display "@ cv " e(n_lambda) " " e(ID_cv) " " e(ID_serule) " " %20.0g e(cvm_min) " " %20.0g e(cvm_serule) " " e(alpha_cv)
quietly elasticnet linear y x1-x8, selection(cv, folds(5)) rseed(12)
display "@ cv3 " e(n_lambda) " " e(ID_cv) " " e(ID_serule) " " %20.0g e(cvm_min) " " %20.0g e(lambda_sel) " " e(alpha_sel)
display "@ cv3_path " %20.0g e(L1_max) " " %20.0g e(lambda_last) " " %20.0g e(lambda_gmax)
quietly elasticnet linear y x1-x8, selection(cv, folds(5)) alpha(.3 .6 .9) rseed(7)
display "@ cv3b " e(n_lambda) " " e(ID_cv) " " %20.0g e(cvm_min) " " e(alpha_sel) " " %20.0g e(ll_sel)
quietly elasticnet linear y x1-x8, selection(cv, folds(5) serule) alpha(.5 1) rseed(12)
display "@ serule " e(ID_sel) " " e(alpha_sel) " " %20.0g e(lambda_sel) " " ("`e(sel_criterion)'" == "serule")
quietly elasticnet linear y (x7) x1-x6 x8, selection(bic) alpha(.5)
display "@ always " e(ID_bic) " " %20.0g e(bic_min) " " e(k_nonzero_sel)
quietly elasticnet linear y x1-x8, selection(bic) alpha(.5) penaltywt(2 1 1 1 1 1 1 1)
display "@ penaltywt " e(ID_bic) " " %20.0g e(bic_min) " " e(k_nonzero_sel)
quietly elasticnet linear y x1-x8, selection(bic) alpha(.5) noconstant
display "@ nocons " e(ID_bic) " " %20.0g e(bic_min) " " %20.0g e(ll_sel)
quietly elasticnet linear y x1-x8, selection(cv, folds(5)) alpha(.5) rseed(12)
predict double p1
predict double p2, postselection
summarize p1, meanonly
display "@ predict " %20.0g r(mean) " " %20.0g r(max)
summarize p2, meanonly
display "@ predict_post " %20.0g r(mean) " " %20.0g r(max)
capture elasticnet linear y x1-x8, alpha(1.5)
display "@ err_alpha " _rc
capture elasticnet linear y x1-x8, selection(adaptive) alpha(.5)
display "@ err_selection " _rc
* end on a fit that works (what a failed model leaves must not reach the next file in the same R session)
quietly elasticnet linear y x1-x8, selection(bic) alpha(.5)
