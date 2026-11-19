* RCraft's Stata corpus (P2C.12, batch I): made-up data; Stata 19's results are in jackknife.recorded
* commands: jackknife
* (the jackknife prefix: r() statistics named and not, _b after regress and logit, the estimation sample only, cluster(), level(), a statistic that never varies, and what it leaves in e() and r())
* tolerance: 1e-8
version 19
set linesize 255
import delimited corpus.csv, clear
jackknife mean=r(mean) sd=r(sd): summarize y
display "@ sum_b " %24.17g _b[mean] " " %24.17g _b[sd] " " %24.17g _se[mean] " " %24.17g _se[sd]
display "@ sum_e " e(N) " " e(N_reps) " " e(df_r) " " e(rank) " " e(k_exp) " " e(k_eexp)
display "@ sum_bjk " %24.17g el(e(b_jk), 1, 1) " " %24.17g el(e(b_jk), 1, 2)
display "@ sum_V " %24.17g el(e(V), 1, 1) " " %24.17g el(e(V), 1, 2) " " %24.17g el(e(V), 2, 2)
display "@ sum_tab " %24.17g el(r(table), 3, 1) " " %24.17g el(r(table), 4, 1) " " %24.17g el(r(table), 5, 1) " " %24.17g el(r(table), 6, 2)
display "@ sum_tab2 " %24.17g el(r(table), 7, 1) " " %24.17g el(r(table), 8, 1) " " %24.17g el(r(table), 9, 1)
jackknife r(mean), nodots: summarize x
display "@ one " %24.17g _b[_jk_1] " " %24.17g _se[_jk_1]
jackknife mean=r(mean), level(90) nodots: summarize x
display "@ lev " %24.17g el(r(table), 5, 1) " " %24.17g el(r(table), 6, 1) " " %24.17g el(r(table), 8, 1) " " %24.17g e(level)
jackknife n=r(N) mean=r(mean), nodots: summarize y
display "@ const " %24.17g _se[n] " " %24.17g _se[mean] " " e(rank)
jackknife _b: regress y x w
display "@ reg_b " %24.17g _b[x] " " %24.17g _b[w] " " %24.17g _b[_cons]
display "@ reg_se " %24.17g _se[x] " " %24.17g _se[w] " " %24.17g _se[_cons]
display "@ reg_e " e(N) " " e(N_reps) " " e(df_r) " " e(df_m) " " e(rank) " " %24.17g e(F) " " %24.17g e(p)
display "@ reg_fit " %24.17g e(r2) " " %24.17g e(rmse) " " %24.17g e(ll) " " %24.17g e(r2_a)
display "@ reg_bjk " %24.17g el(e(b_jk), 1, 1) " " %24.17g el(e(b_jk), 1, 3)
display "@ reg_Vm " %24.17g el(e(V_modelbased), 1, 1) " " %24.17g el(e(V_modelbased), 2, 3)
display "@ reg_V " %24.17g el(e(V), 1, 2) " " %24.17g el(e(V), 3, 3)
display "@ reg_tab " %24.17g el(r(table), 4, 1) " " %24.17g el(r(table), 5, 2) " " %24.17g el(r(table), 6, 3)
jackknife _b, cluster(cat3): regress y x w
display "@ cl_se " %24.17g _se[x] " " %24.17g _se[w] " " %24.17g _se[_cons]
display "@ cl_e " e(N) " " e(N_reps) " " e(df_r) " " e(N_clust) " " %24.17g e(F) " " %24.17g e(p)
replace y = . in 3/5
jackknife _b, nodots: regress y x
display "@ miss_se " %24.17g _se[x] " " %24.17g _se[_cons] " " e(N) " " e(N_reps)
jackknife _b, nodots: logit yb x w
display "@ logit_b " %24.17g _b[x] " " %24.17g _b[w] " " %24.17g _b[_cons]
display "@ logit_se " %24.17g _se[x] " " %24.17g _se[w] " " %24.17g _se[_cons]
display "@ logit_e " e(N) " " e(N_reps) " " e(df_r) " " %24.17g e(ll) " " %24.17g e(r2_p)
replace y = 2.5 in 3
replace y = 3.25 in 4
replace y = 0.75 in 5
regress y x w, vce(jackknife)
display "@ vce_reg " %24.17g _b[x] " " %24.17g _se[x] " " %24.17g _se[w] " " %24.17g _se[_cons]
display "@ vce_reg_e " e(N) " " e(N_reps) " " e(df_r) " " %24.17g e(F) " " %24.17g e(r2)
logit yb x w, vce(jackknife) nolog
display "@ vce_logit " %24.17g _b[x] " " %24.17g _se[x] " " %24.17g _se[w] " " %24.17g _se[_cons]
display "@ vce_logit_e " e(N) " " e(N_reps) " " e(df_r) " " %24.17g e(ll)
probit yb x w, vce(jack) nolog
display "@ vce_probit " %24.17g _b[x] " " %24.17g _se[x] " " %24.17g _se[w] " " %24.17g _se[_cons]
* a command with no e(sample): the observations its if picks are the units
import delimited corpus.csv, clear
jackknife r(mean), nodots: summarize y if x > 0
display "@ if_e " e(N) " " e(N_reps) " " e(df_r) " " %24.17g _b[_jk_1] " " %24.17g _se[_jk_1]
jackknife m=r(mean), nodots: summarize y if x < 0.5 & w > -1
display "@ if2_e " e(N) " " e(N_reps) " " %24.17g _b[m] " " %24.17g _se[m]
* a missing value of the cluster variable
replace cat3 = . in 1/3
capture noisily jackknife _b, cluster(cat3) nodots: regress y x
display "@ rc459 " _rc
