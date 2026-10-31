* RCraft's Stata corpus (P2B.10): made-up data; Stata 19's results are in panel.recorded
* commands: import xtset xtreg ivregress anova
* tolerance: 1e-8
version 19
import delimited corpus.csv, clear
xtset panel t
xtreg y x w, fe
display "@ fe_b_x " %24.17g _b[x]
display "@ fe_se_x " %24.17g _se[x]
display "@ fe_sigma_u " %24.17g e(sigma_u)
display "@ fe_rho " %24.17g e(rho)
display "@ fe_r2_w " %24.17g e(r2_w)
xtreg y x w, re
display "@ re_b_x " %24.17g _b[x]
display "@ re_se_x " %24.17g _se[x]
display "@ re_sigma_u " %24.17g e(sigma_u)
xtreg y x w, be
display "@ be_b_x " %24.17g _b[x]
display "@ be_se_x " %24.17g _se[x]
xtreg y x w, fe vce(robust)
display "@ fer_se_x " %24.17g _se[x]
ivregress 2sls y w (x = z)
display "@ iv_b_x " %24.17g _b[x]
display "@ iv_se_x " %24.17g _se[x]
ivregress 2sls y w (x = z), small
display "@ ivs_se_x " %24.17g _se[x]
ivregress liml y w (x = z)
display "@ liml_b_x " %24.17g _b[x]
anova y g cat3
display "@ an_F " %24.17g e(F)
display "@ an_r2 " %24.17g e(r2)
display "@ an_rmse " %24.17g e(rmse)
