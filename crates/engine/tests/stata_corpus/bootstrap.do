* RCraft's Stata corpus (P2C.16): made-up data; Stata 19's results are in bootstrap.recorded
* commands: bootstrap
* (the bootstrap prefix with Stata's own draws: r() statistics named and not, _b after regress, the estimation sample only, cluster(), e() after it and the draws after it)
* tolerance: 1e-9
version 19
import delimited corpus.csv, clear
bootstrap mean=r(mean), reps(25) seed(3): summarize y
display "@ mean " %24.17g _b[mean] " " %24.17g _se[mean]
display "@ mean_e " e(N) " " e(N_reps) " " %24.17g el(e(ci_normal), 1, 1)
display "@ mean_bs " %24.17g el(e(b_bs), 1, 1) " " %24.17g el(e(bias), 1, 1)
bootstrap _b, reps(30) seed(11) nodots: regress y x w
display "@ reg_se " %24.17g _se[x] " " %24.17g _se[_cons]
display "@ reg_b " %24.17g _b[w] " " %24.17g e(r2)
bootstrap r(mean) r(sd), reps(12) seed(5): summarize x
display "@ two " %24.17g _se[_bs_1] " " %24.17g _se[_bs_2]
replace y = . in 3/5
bootstrap _b, reps(10) seed(2) nodots: regress y x
display "@ sample " %24.17g _se[x] " " e(N)
bootstrap mean=r(mean), reps(10) seed(4) cluster(cat3): summarize x
display "@ cluster " %24.17g _se[mean] " " e(N)
display "@ next " %24.17g runiform()
