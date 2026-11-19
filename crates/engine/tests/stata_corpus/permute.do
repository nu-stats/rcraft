* RCraft's Stata corpus (P2C.12, batch I): made-up data; Stata 19's results are in permute.recorded
* commands: permute
* (permute: the permutations themselves (an encoded statistic shows each one), strata, a variable with ties and a missing value, two-valued variables, a command that sorts the data, the results table and what it leaves in r(), _b and _se, the draws of the command and after it, and the errors with their return codes)
* tolerance: 1e-8
version 19
set linesize 255
program define permcode, rclass
    tempvar w
    generate double `w' = x * 8^(id - 1)
    summarize `w', meanonly
    return scalar code = r(sum)
end
program define permdraw, rclass
    tempvar w
    generate double `w' = x * 8^(id - 1)
    summarize `w', meanonly
    return scalar code = r(sum)
    return scalar u = runiform()
end
program define permsort, rclass
    sort x
    tempvar w
    generate double `w' = x * 8^(id - 1)
    summarize `w', meanonly
    return scalar code = r(sum)
end
program define bincode, rclass
    tempvar w
    generate double `w' = treat * 2^(id - 1)
    summarize `w', meanonly
    return scalar code = r(sum)
end
* x = 0..5 in id order, the code is x in base 8
clear
set obs 6
generate id = _n
generate x = _n - 1
permute x code=r(code), reps(12) seed(3041) saving(permute_1, replace double) nodots: permcode
use permute_1, clear
display "@ g_n " _N
forvalues i = 1/`=_N' {
    display "@ g_`i' " %24.17g code[`i']
}
* the same draws by hand: one uniform each
clear
set obs 6
generate id = _n
generate x = _n - 1
permute x code=r(code) u=r(u), reps(5) seed(3041) saving(permute_2, replace double) nodots: permdraw
use permute_2, clear
forvalues i = 1/`=_N' {
    display "@ d_`i' " %24.17g code[`i'] " " %24.17g u[`i']
}
clear
set obs 6
generate id = _n
generate x = _n - 1
set seed 3041
permute x code=r(code), reps(1) nodots: permcode
display "@ d_next " %24.17g runiform()
* strata: within-stratum permutations, strata not sorted
clear
set obs 8
generate id = _n
generate x = _n - 1
generate g = mod(_n, 2)
permute x code=r(code), reps(8) seed(3042) strata(g) saving(permute_3, replace double) nodots: permcode
use permute_3, clear
forvalues i = 1/`=_N' {
    display "@ s_`i' " %24.17g code[`i']
}
* a variable with ties and a missing value
clear
input id x
1 3
2 1
3 3
4 .
5 2
end
permute x code=r(code), reps(6) seed(3043) saving(permute_4, replace double) nodots: permcode
use permute_4, clear
forvalues i = 1/`=_N' {
    display "@ t_`i' " %24.17g code[`i']
}
* a command that sorts the data
clear
set obs 6
generate id = _n
generate x = _n - 1
permute x code=r(code), reps(6) seed(3046) saving(permute_5, replace double) nodots: permsort
use permute_5, clear
forvalues i = 1/`=_N' {
    display "@ o_`i' " %24.17g code[`i']
}
* two values: 0 and 1, 20 and 20
clear
set seed 3044
set obs 40
generate id = _n
generate treat = _n > 20
generate y = round(rnormal() + 0.6*treat, .01)
permute treat code=r(code), reps(4) seed(30441) nodots saving(permute_6, replace double): bincode
use permute_6, clear
forvalues i = 1/`=_N' {
    display "@ b_`i' " %24.17g code[`i']
}
* two values in other sizes
local k = 0
foreach case in "mod(_n,2)" "cond(_n<=3,0,cond(_n<=5,1,2))" "mod(_n*5,3)" "_n>5" "cond(_n<=2,0,1)" "cond(_n<=4,1,0)" "cond(_n<=3,5,2)" "cond(_n<=6,.,1)" {
    local ++k
    clear
    set obs 8
    generate id = _n
    generate x = `case'
    permute x code=r(code), reps(8) seed(305`k') nodots saving(permute_7, replace double): permcode
    use permute_7, clear
    forvalues i = 1/`=_N' {
        display "@ w`k'_`i' " %24.17g code[`i']
    }
}
* two values and strata
clear
set obs 8
generate id = _n
generate g = _n > 4
generate x = cond(mod(_n,4)==1, 1, 0)
replace x = 2 if _n == 8
permute x code=r(code), reps(6) seed(3063) strata(g) nodots saving(permute_8, replace double): permcode
use permute_8, clear
forvalues i = 1/`=_N' {
    display "@ ws_`i' " %24.17g code[`i']
}
clear
set obs 8
generate id = _n
generate g = _n > 4
generate x = cond(mod(_n,4)==1, 1, 0)
permute x code=r(code), reps(6) seed(3064) strata(g) nodots saving(permute_9, replace double): permcode
use permute_9, clear
forvalues i = 1/`=_N' {
    display "@ wt_`i' " %24.17g code[`i']
}
* the results table and r()
clear
set seed 3044
set obs 40
generate treat = _n > 20
generate y = round(rnormal() + 0.6*treat, .01)
generate x = round(rnormal(), .01)
generate g = ceil(_n/10)
permute treat diff=(r(mu_2)-r(mu_1)), reps(200) seed(30441) nodots: ttest y, by(treat)
display "@ r_n " r(N) " " r(n_reps) " " r(k_exp) " " r(k_eexp) " " r(level)
display "@ r_b " %24.17g el(r(b), 1, 1) " " el(r(n), 1, 1) " " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 1)
display "@ r_p " %24.17g el(r(p_lower), 1, 1) " " %24.17g el(r(p_upper), 1, 1) " " %24.17g el(r(p_twosided), 1, 1)
display "@ r_se " %24.17g el(r(se_p_lower), 1, 1) " " %24.17g el(r(se_p_upper), 1, 1) " " %24.17g el(r(se_p_twosided), 1, 1)
display "@ r_ci " %24.17g el(r(ci_p_lower), 1, 1) " " %24.17g el(r(ci_p_lower), 2, 1) " " %24.17g el(r(ci_p_upper), 1, 1) " " %24.17g el(r(ci_p_upper), 2, 1) " " %24.17g el(r(ci_p_twosided), 1, 1) " " %24.17g el(r(ci_p_twosided), 2, 1)
display "@ r_e " e(N)
permute treat r(t) r(p), reps(75) seed(30442): ttest y, by(treat)
display "@ r2_b " %24.17g el(r(b), 1, 1) " " %24.17g el(r(b), 1, 2) " " el(r(c_lower), 1, 2) " " el(r(c_upper), 1, 1)
display "@ r2_ci " %24.17g el(r(ci_p_twosided), 1, 1) " " %24.17g el(r(ci_p_twosided), 2, 1) " " %24.17g el(r(ci_p_twosided), 1, 2) " " %24.17g el(r(ci_p_twosided), 2, 2)
permute y _b, reps(50) seed(30443) nodots: regress y x
display "@ b_b " %24.17g el(r(b), 1, 1) " " %24.17g el(r(b), 1, 2) " " r(k_eexp) " " r(k_exp)
display "@ b_p " %24.17g el(r(p_twosided), 1, 1) " " %24.17g el(r(p_lower), 1, 2) " " el(r(c_upper), 1, 2)
permute y b=_b[x], reps(50) seed(30443) strata(g) nodots: regress y x
display "@ st_b " r(N) " " r(n_strata) " " r(n_reps) " " %24.17g el(r(b), 1, 1) " " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 1)
permute y _b _se, reps(30) seed(30444) nodots: regress y x treat
display "@ bs_b " %24.17g el(r(b), 1, 1) " " %24.17g el(r(b), 1, 4) " " %24.17g el(r(b), 1, 6) " " r(k_eexp)
display "@ bs_c " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 3) " " el(r(c_lower), 1, 5) " " el(r(n), 1, 6)
permute x r(rho), reps(100) seed(30445) level(90) nodots: correlate y x
display "@ sp " %24.17g el(r(b), 1, 1) " " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 1) " " r(level)
display "@ sp_ci " %24.17g el(r(ci_p_lower), 1, 1) " " %24.17g el(r(ci_p_lower), 2, 1) " " %24.17g el(r(ci_p_twosided), 2, 1)
permute x r(mean), seed(30446) nodots: summarize x
display "@ df " r(n_reps) " " el(r(n), 1, 1)
permute y m=r(mean) _pm_2=r(sd), reps(10) seed(30452) strata(g) nodots: summarize y
display "@ sm " %24.17g el(r(b), 1, 1) " " %24.17g el(r(b), 1, 2) " " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 2) " " r(n_strata)
* a permutation that gives missing values or a result missing in the data of the permutations
display "@ next " %24.17g runiform()
* errors, with their return codes
capture noisily permute nosuch r(mean), reps(5): summarize y
display "@ rc111 " _rc
capture noisily permute y r(mean), reps(0): summarize y
display "@ rc125 " _rc
capture noisily permute y r(mean) reps(5): summarize y
display "@ rc133 " _rc
capture noisily permute y r(nosuch), reps(5) nodots: summarize y
display "@ rc322 " _rc
permute x r(rho) r(N), reps(60) seed(30448) nodots: quietly spearman y x
display "@ spq " %24.17g el(r(b), 1, 1) " " el(r(b), 1, 2) " " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 1) " " %24.17g el(r(p_twosided), 1, 1)
capture erase permute_1.dta
capture erase permute_2.dta
capture erase permute_3.dta
capture erase permute_4.dta
capture erase permute_5.dta
capture erase permute_6.dta
capture erase permute_7.dta
capture erase permute_8.dta
capture erase permute_9.dta
* the estimation sample of a model: rows with a missing value are not permuted
replace y = . in 3/5
replace x = . in 7
permute x _b, reps(20) seed(30447) nodots saving(permute_10, replace double): regress y x
display "@ es_b " r(N) " " %24.17g el(r(b), 1, 1) " " %24.17g el(r(b), 1, 2) " " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 2)
use permute_10, clear
forvalues i = 1/5 {
    display "@ es_`i' " %24.17g _b_x[`i'] " " %24.17g _b_cons[`i']
}
capture erase permute_10.dta
* a program that fits a model: its e(sample) is the sample, and _b is the model's
program define permreg, eclass
    regress y x
end
import delimited corpus.csv, clear
permute x _b, reps(15) seed(30449) nodots saving(permute_11, replace double): permreg
display "@ ep " r(N) " " %24.17g el(r(b), 1, 1) " " %24.17g el(r(b), 1, 2) " " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 2) " " r(k_eexp)
use permute_11, clear
forvalues i = 1/4 {
    display "@ ep_`i' " %24.17g _b_x[`i'] " " %24.17g _b_cons[`i']
}
capture erase permute_11.dta
* _b after logit: the variables and the table are named after the equation
import delimited corpus.csv, clear
permute yb _b, reps(12) seed(30450) nodots saving(permute_12, replace double): logit yb x
display "@ pl " r(N) " " %24.17g el(r(b), 1, 1) " " %24.17g el(r(b), 1, 2) " " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 2) " " r(k_eexp)
use permute_12, clear
forvalues i = 1/4 {
    display "@ pl_`i' " %24.17g yb_b_x[`i'] " " %24.17g yb_b_cons[`i']
}
capture erase permute_12.dta
* a replication that stops with an error is missing, and n counts the others
program define permfail, rclass
    tempvar w
    generate double `w' = x * 8^(id - 1)
    summarize `w', meanonly
    if x[1] == 3 {
        error 459
    }
    return scalar code = r(sum)
end
clear
set obs 6
generate id = _n
generate x = _n - 1
permute x code=r(code), reps(12) seed(3041) saving(permute_13, replace double) nodots: permfail
display "@ pf " el(r(n), 1, 1) " " el(r(c_lower), 1, 1) " " el(r(c_upper), 1, 1)
use permute_13, clear
forvalues i = 1/12 {
    display "@ pf_`i' " %24.17g code[`i']
}
capture erase permute_13.dta
