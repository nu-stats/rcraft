* RCraft's Stata corpus (P2C.16 continued): made-up data; Stata 19's results are in bsopts.recorded
* commands: bootstrap bsample
* (bsample, strata() and with a size; bootstrap with strata(), its standard errors, e(N_strata) and the next draw; saving() as floats and with double, read back with use; estat bootstrap after it; r(602) for a file there without replace)
* tolerance: 1e-9
version 19
set linesize 255
clear
set obs 12
generate id = _n
generate g = 1 + mod(_n, 3)
set seed 4
bsample, strata(g)
display "@ bsample_strata " id[1] " " id[2] " " id[5] " " id[9] " " id[12] " " g[1] " " g[12]
display "@ bsample_next " runiform()
clear
set obs 12
generate id = _n
generate g = 1 + (_n > 5) + (_n > 9)
set seed 2
bsample 2, strata(g)
display "@ bsample_n " _N " " id[1] " " id[2] " " id[3] " " id[4] " " id[5] " " id[6]
clear
set obs 30
generate x = mod(_n * 7, 11)
generate y = 2 + 0.5 * x + mod(_n * 3, 5)
generate g = 1 + (_n > 12) + (_n > 22)
bootstrap r(mean), reps(20) seed(5) nodots strata(g): summarize y
display "@ strata_mean " _se[_bs_1] " " e(N_strata) " " e(N_reps) " " ("`e(strata)'" == "g")
display "@ strata_next " runiform()
bootstrap, reps(20) seed(5) nodots strata(g): regress y x
display "@ strata_regress " _se[x] " " _se[_cons] " " el(e(ci_bc), 1, 1) " " el(e(ci_percentile), 2, 2)
estat bootstrap, all
capture erase bsopts_1.dta
capture erase bsopts_2.dta
bootstrap r(mean), reps(5) seed(11) nodots saving(bsopts_1, replace): summarize y
display "@ saved_se " _se[_bs_1]
capture bootstrap r(mean), reps(5) seed(11) nodots saving(bsopts_1): summarize y
display "@ saving_there " _rc
bootstrap m = r(mean) s = r(sd), reps(4) seed(11) nodots saving(bsopts_2, replace double): summarize y
display "@ double_se " _se[m] " " _se[s]
bootstrap, reps(4) seed(11) nodots saving(bsopts_3, replace): regress y x
estat bootstrap
preserve
use bsopts_1, clear
display "@ file_1 " _N " " _bs_1[1] " " _bs_1[5]
use bsopts_2, clear
display "@ file_2 " _N " " m[1] " " s[4]
use bsopts_3, clear
display "@ file_3 " _N " " _b_x[2] " " _b_cons[4]
restore
erase bsopts_1.dta
erase bsopts_2.dta
erase bsopts_3.dta
