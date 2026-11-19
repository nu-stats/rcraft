* RCraft's Stata corpus (P2C.12, batch I): made-up data; Stata 19's results are in simulate.recorded
* commands: simulate
* (simulate: r() statistics named and not, _b and _se of an e-class program, e() values, each replication on the data as they were, what it leaves in the data and in r(), the draws of the program and after it)
* tolerance: 1e-8
version 19
set linesize 255
program define sim1, rclass
    drop _all
    set obs 30
    generate x = rnormal()
    summarize x
    return scalar mean = r(mean)
    return scalar sd = r(sd)
end
program define sim2, eclass
    drop _all
    set obs 40
    generate x = rnormal()
    generate y = 1 + 2*x + rnormal()
    regress y x
end
program define simkeep, rclass
    generate z = rnormal() + q
    summarize z
    return scalar m = r(mean)
    drop z
end
clear
simulate mean=r(mean) sd=r(sd), reps(25) seed(303) nodots: sim1
display "@ s1_n " _N
forvalues i = 1/6 {
    display "@ s1_`i' " %24.17g mean[`i'] " " %24.17g sd[`i']
}
display "@ s1_last " %24.17g mean[25] " " %24.17g sd[25]
display "@ s1_r " %24.17g r(mean) " " %24.17g r(sd)
display "@ s1_next " %24.17g runiform()
simulate r(mean), reps(5) seed(1) nodots: sim1
forvalues i = 1/5 {
    display "@ s2_`i' " %24.17g _sim_1[`i']
}
simulate _b _se, reps(12) seed(9) nodots: sim2
display "@ s3_n " _N
forvalues i = 1/3 {
    display "@ s3_`i' " %24.17g _b_x[`i'] " " %24.17g _b_cons[`i'] " " %24.17g _se_x[`i'] " " %24.17g _se_cons[`i']
}
simulate b=_b[x] n=e(N), reps(3) seed(9) nodots: sim2
forvalues i = 1/3 {
    display "@ s4_`i' " %24.17g b[`i'] " " n[`i']
}
simulate b=_b[x] r2=e(r2) rmse=e(rmse), reps(8) seed(11): sim2
display "@ s5_n " _N
forvalues i = 1/8 {
    display "@ s5_`i' " %24.17g b[`i'] " " %24.17g r2[`i'] " " %24.17g rmse[`i']
}
* the data in memory are the data the program works on
clear
set obs 5
generate q = _n
simulate m=r(m), reps(3) seed(5) nodots: simkeep
display "@ k_n " _N
forvalues i = 1/3 {
    display "@ k_`i' " %24.17g m[`i']
}
* a replication that stops with an error leaves a row of missing values
program define simfail, rclass
    generate z = rnormal()
    summarize z
    return scalar m = r(mean)
end
clear
set obs 5
simulate m=r(m), reps(4) seed(5) nodots: simfail
display "@ f_n " _N
forvalues i = 1/4 {
    display "@ f_`i' " %24.17g m[`i']
}
