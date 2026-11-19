* RCraft's Stata corpus (P2C.12, batch I follow-up): made-up data; Stata 19's results are in resample_logit.recorded
* commands: statsby rolling simulate
* (logit and probit name the variables the resampling commands make after the equation (yb_b_x, labelled [yb]_b[x]); the results are floats, and Stata's and RCraft's maximum likelihood estimates agree to about 1e-8, so a float rounds differently now and then: compared to 1e-6)
* tolerance: 1e-6
version 19
set linesize 255
import delimited corpus.csv, clear
* logit and probit name the variables after the equation
preserve
statsby _b _se, clear nodots: logit yb x
display "@ s8 " _N " " %24.17g yb_b_x[1] " " %24.17g yb_b_cons[1] " " %24.17g yb_se_x[1] " " %24.17g yb_se_cons[1]
restore
preserve
statsby _b, by(cat3) clear nodots: probit yb x
display "@ s9_n " _N
forvalues i = 1/`=_N' {
    display "@ s9_`i' " %24.17g yb_b_x[`i'] " " %24.17g yb_b_cons[`i']
}
restore
preserve
generate tt = _n
tsset tt
rolling _b, window(30) stepsize(10) clear nodots: logit yb x
display "@ r4_n " _N
forvalues i = 1/`=_N' {
    display "@ r4_`i' " start[`i'] " " end[`i'] " " %24.17g yb_b_x[`i'] " " %24.17g yb_b_cons[`i']
}
restore
* _b after logit: the variables are named after the equation
program define simlogit, eclass
    drop _all
    set obs 40
    generate x = rnormal()
    generate yb = runiform() < invlogit(x)
    logit yb x
end
simulate _b _se, reps(6) seed(31) nodots: simlogit
display "@ l_n " _N
forvalues i = 1/6 {
    display "@ l_`i' " %24.17g yb_b_x[`i'] " " %24.17g yb_b_cons[`i'] " " %24.17g yb_se_x[`i']
}
