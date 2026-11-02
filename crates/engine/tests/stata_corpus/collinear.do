* RCraft's Stata corpus (P2C.13): made-up data; Stata 19's results are in collinear.recorded
* Stata stops iterating at its tolerance, so the likelihood models agree to about 7 digits
* commands: logit mlogit nbreg ologit oprobit poisson probit regress
* (collinear regressors: which one each model omits, whatever the order written; its zero in e(b); e(rank) and e(df_m))
* tolerance: 1e-6
version 19
import delimited corpus.csv, clear
generate x3 = 0.5 * x
generate ten = x + 10
generate v = x + w
generate d2 = cat3 == 2
regress y x x3
display "@ r_x_x3 " %24.17g _b[x] " " %24.17g _b[x3] " " e(rank) " " e(df_m)
regress y x3 x
display "@ r_x3_x " %24.17g _b[x] " " %24.17g _b[x3] " " e(rank) " " e(df_m)
regress y ten x
display "@ r_ten_x " %24.17g _b[x] " " %24.17g _b[ten]
display "@ r_ten_cons " %24.17g _b[_cons]
regress y x w v
display "@ r_xwv " %24.17g _b[x] " " %24.17g _b[w]
display "@ r_xwv_v " %24.17g _b[v]
regress y v w x
display "@ r_vwx " %24.17g _b[x] " " %24.17g _b[w]
display "@ r_vwx_v " %24.17g _b[v]
regress y x x3 w v
display "@ r_four_b " %24.17g _b[x] " " %24.17g _b[x3]
display "@ r_four_w " %24.17g _b[w] " " %24.17g _b[v]
display "@ r_four_v " %24.17g _se[v] " " e(rank) " " e(df_m)
regress y d2 i.cat3
display "@ r_dummy " %24.17g _b[d2] " " %24.17g _b[2.cat3]
display "@ r_dummy_3 " %24.17g _b[3.cat3] " " e(df_m)
display "@ r_dummy_fit " %24.17g e(r2) " " %24.17g e(rmse)
regress y x x3, noconstant
display "@ r_nocons " %24.17g _b[x] " " %24.17g _b[x3] " " e(rank)
logit yb x3 x
display "@ l_x3_x " %24.17g _b[x3] " " %24.17g _b[x] " " e(rank) " " e(df_m)
logit yb x ten
display "@ l_x_ten " %24.17g _b[x] " " %24.17g _se[x]
display "@ l_x_ten_b " %24.17g _b[ten] " " e(df_m)
display "@ l_x_ten_fit " %24.17g e(ll) " " %24.17g e(chi2)
logit yb v w x
display "@ l_vwx " %24.17g _b[v] " " %24.17g _b[w]
display "@ l_vwx_x " %24.17g _b[x]
probit yb x x3 w
display "@ p_x_x3 " %24.17g _b[x] " " %24.17g _b[x3]
display "@ p_w " %24.17g _se[w]
poisson cnt x ten w
display "@ q_x_ten " %24.17g _b[x] " " %24.17g _b[ten]
display "@ q_w " %24.17g _se[w]
display "@ q_fit " %24.17g e(ll) " " e(rank) " " e(df_m)
generate od = cnt * (1 + mod(_n, 4))
nbreg od x x3 w
display "@ nb_x " %24.17g _b[x] " " %24.17g _b[x3]
display "@ nb_w " %24.17g _se[w] " " e(rank) " " e(df_m)
ologit ord x3 x w
display "@ ol_x " %24.17g _b[x3] " " %24.17g _b[x]
display "@ ol_w " %24.17g _se[w] " " e(rank) " " e(df_m)
oprobit ord x ten
display "@ op_x " %24.17g _b[x] " " %24.17g _b[ten]
display "@ op_n " e(rank) " " e(df_m)
mlogit cat3 x x3 w
display "@ ml_1 " %24.17g _b[1:x] " " %24.17g _b[1:x3]
display "@ ml_3 " %24.17g _b[3:w] " " %24.17g _b[2:x3]
display "@ ml_n " e(rank) " " e(df_m)
