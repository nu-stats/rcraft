* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in lasso.recorded
* commands: lasso lassoselect
* (lasso linear with selection none, bic, cv, adaptive and plugin; the grid, stop(),
* always-in variables, penaltywt(), noconstant; lassoselect and predict; Stata's errors and r(430))
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
quietly lasso linear y x1-x8, selection(none) grid(6)
lassoselect id = 4
display "@ none " e(n_lambda) " " e(ID_sel) " " %20.0g e(lambda_sel) " " %20.0g e(ll_sel)
display "@ none_path " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max) " " %20.0g e(lambda_gmin)
display "@ none_sel " ("`e(sel_criterion)'" == "user") " " ("`e(allvars_sel)'" == "x1 x2 x3 x4 x5 x7")
display "@ none_b " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, colsof(e(b)))
quietly lasso linear y x1-x8, selection(bic)
display "@ bic " e(n_lambda) " " e(ID_bic) " " %20.0g e(lambda_bic) " " %20.0g e(bic_min) " " e(k_nonzero_sel)
display "@ bic_vars " wordcount("`e(othervars_sel)'") " " ("`e(sel_criterion)'" == "BIC min.")
display "@ bic_path " %20.0g e(L1_max) " " %20.0g e(L2_max) " " %20.0g e(devratio_max)
quietly lasso linear y x1-x8, selection(bic, postselection)
display "@ bicpost " e(ID_bic) " " %20.0g e(bic_min)
quietly lasso linear y x1-x8, selection(cv, folds(5)) rseed(12)
display "@ cv " e(n_lambda) " " e(ID_cv) " " e(ID_serule) " " %20.0g e(cvm_min) " " %20.0g e(cvm_serule)
display "@ cv_sel " %20.0g e(lambda_sel) " " e(k_nonzero_cv) " " e(n_fold)
display "@ cv_state " ("`e(rngstate)'" != "")
quietly lasso linear y x1-x8, selection(cv, folds(5) serule) rseed(12)
display "@ serule " e(ID_sel) " " ("`e(sel_criterion)'" == "serule")
quietly lasso linear y x1-x8, selection(cv) rseed(3) grid(40)
display "@ cv10 " e(n_lambda) " " e(ID_cv) " " %20.0g e(cvm_min)
quietly lasso linear y x1-x8, selection(adaptive) rseed(5)
display "@ adaptive " e(n_lambda) " " e(ID_cv) " " %20.0g e(lambda_sel) " " e(k_nonzero_max) " " %20.0g e(L1_max)
quietly lasso linear y x1-x8, selection(plugin)
display "@ plugin " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel)
quietly lasso linear y x1-x8, selection(plugin, homoskedastic)
display "@ plugin_h " %20.0g e(lambda_sel) " " e(k_nonzero_sel) " " %20.0g e(ll_sel)
quietly lasso linear y (x8) x1-x7, selection(bic)
display "@ always " e(ID_bic) " " wordcount("`e(allvars_sel)'") " " %20.0g e(lambda_gmax)
quietly lasso linear y x1-x8, selection(bic) penaltywt(1 1 .5 1 1 2 1 1)
display "@ penaltywt " e(ID_bic) " " %20.0g e(lambda_gmax) " " wordcount("`e(allvars_sel)'")
quietly lasso linear y x1-x8, selection(bic) noconstant
display "@ nocons " e(ID_bic) " " %20.0g e(bic_min) " " colsof(e(b))
quietly lasso linear y x1-x8 if x4 > -0.5, selection(bic) stop(1e-3)
display "@ ifstop " e(N) " " e(n_lambda) " " e(ID_sel)
quietly lasso linear y x1-x8, selection(bic)
predict double p1
predict double p2, postselection
quietly summarize p1
display "@ predict " %20.0g r(max) " " %20.0g r(sd)
quietly summarize p2
display "@ predict_post " %20.0g r(max) " " %20.0g r(sd)
capture lasso linear y x1-x8, selection(foo)
display "@ rc_sel " _rc
capture lasso y x1-x8
display "@ rc_model " _rc
capture lasso linear y, selection(bic)
display "@ rc_vars " _rc
capture lasso linear y x1-x8, selection(cv, folds(2))
display "@ rc_folds " _rc
capture lasso linear y x1-x8, grid(0)
display "@ rc_grid " _rc
capture lasso linear y x1-x8, bogus
display "@ rc_opt " _rc
capture lasso linear y x1-x8, selection(cv, folds(5)) rseed(2) grid(6, ratio(.5))
display "@ rc_nomin " _rc " " e(n_lambda) " " ("`e(sel_criterion)'" == "not sel.")
quietly lasso linear y x1-x8, selection(plugin)
capture lassoselect id = 1
display "@ rc_select " _rc
