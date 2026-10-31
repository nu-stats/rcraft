* RCraft's Stata corpus (P2B.10): made-up data; Stata 19's results are in regress.recorded
* commands: import regress test lincom predict margins
* tolerance: 1e-8
version 19
import delimited corpus.csv, clear
regress y x w
display "@ b_x " %24.17g _b[x]
display "@ se_x " %24.17g _se[x]
display "@ b_cons " %24.17g _b[_cons]
display "@ r2 " %24.17g e(r2)
display "@ F " %24.17g e(F)
display "@ rmse " %24.17g e(rmse)
display "@ ll " %24.17g e(ll)
test x w
display "@ test_F " %24.17g r(F)
display "@ test_p " %24.17g r(p)
lincom x + w
display "@ lin_est " %24.17g r(estimate)
display "@ lin_se " %24.17g r(se)
predict yhat
predict res, residuals
summarize yhat
display "@ yhat_mean " %24.17g r(mean)
display "@ yhat_sd " %24.17g r(sd)
summarize res
display "@ res_sd " %24.17g r(sd)
regress y x w, robust
display "@ rob_se_x " %24.17g _se[x]
display "@ rob_se_w " %24.17g _se[w]
display "@ rob_F " %24.17g e(F)
regress y x w, vce(cluster panel)
display "@ cl_se_x " %24.17g _se[x]
display "@ cl_F " %24.17g e(F)
regress y x i.cat3
display "@ fac_b2 " %24.17g _b[2.cat3]
display "@ fac_se3 " %24.17g _se[3.cat3]
regress y c.x##i.g
display "@ int_b " %24.17g _b[1.g#c.x]
display "@ int_se " %24.17g _se[1.g#c.x]
regress y x w [fw=f]
display "@ fw_b_x " %24.17g _b[x]
display "@ fw_se_x " %24.17g _se[x]
display "@ fw_N " %24.17g e(N)
regress y x w [aw=a]
display "@ aw_b_x " %24.17g _b[x]
display "@ aw_se_x " %24.17g _se[x]
display "@ aw_ll " %24.17g e(ll)
regress y x w
margins, dydx(w) post
display "@ mg_b " %24.17g _b[w]
display "@ mg_se " %24.17g _se[w]
