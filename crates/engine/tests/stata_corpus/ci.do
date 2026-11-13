* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in ci.recorded
* commands: ci cii
* (ci means, ci proportions with every method, at 0 and 1 and in between, and ci variances: the last variable's r(); frequency and analytic weights for means; level(); one and no observations)
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 21
set obs 60
generate x = 10 + 3*rnormal()
generate y = round(5*runiform(), .1)
replace y = . in 1/4
generate b = runiform() < .3
generate fw = 1 + mod(_n, 3)
generate z = 0
generate o = 1
ci means x
display "@ m " r(N) " " %24.17g r(mean) " " %24.17g r(se) " " %24.17g r(lb) " " %24.17g r(ub) " " r(level)
ci means x y, level(90)
display "@ m90 " r(N) " " %24.17g r(mean) " " %24.17g r(lb) " " %24.17g r(ub)
ci means x [fweight = fw]
display "@ mfw " r(N) " " %24.17g r(mean) " " %24.17g r(se) " " %24.17g r(lb)
ci means x [aweight = fw]
display "@ maw " r(N) " " %24.17g r(mean) " " %24.17g r(se) " " %24.17g r(ub)
ci means x in 1
display "@ mone " r(N) " " r(mean) " " r(se) " " r(lb)
ci means x if 0
display "@ mnone " r(N) " " r(mean)
foreach m in exact wald wilson agresti jeffreys {
    ci proportions b, `m'
    display "@ p_`m' " r(N) " " %24.17g r(proportion) " " %24.17g r(se) " " %24.17g r(lb) " " %24.17g r(ub)
    ci proportions z, `m' level(99)
    display "@ z_`m' " %24.17g r(lb) " " %24.17g r(ub)
    ci proportions o, `m' level(80)
    display "@ o_`m' " %24.17g r(lb) " " %24.17g r(ub)
}
ci proportions x
display "@ nobin " r(N)
ci variances x
display "@ v " r(N) " " %24.17g r(Var) " " %24.17g r(lb) " " %24.17g r(ub)
ci variances y, level(99)
display "@ v99 " r(N) " " %24.17g r(Var) " " %24.17g r(lb) " " %24.17g r(ub)
generate k = mod(_n*7, 5) + (_n > 40)
generate e = 1 + mod(_n, 4)
ci means k, poisson
display "@ pois " r(N) " " %24.17g r(mean) " " %24.17g r(se) " " %24.17g r(lb) " " %24.17g r(ub)
ci means k z, poisson exposure(e) level(90)
display "@ pois_e " r(N) " " %24.17g r(mean) " " %24.17g r(lb) " " %24.17g r(ub)
ci means x, poisson
display "@ pois_x " r(N) " " %24.17g r(mean) " " %24.17g r(se) " " %24.17g r(lb)
ci variances x, bonett
display "@ bon " %24.17g r(Var) " " %24.17g r(kurtosis) " " %24.17g r(lb) " " %24.17g r(ub)
ci variances x in 1/8, bonett level(90)
display "@ bon8 " %24.17g r(kurtosis) " " %24.17g r(lb) " " %24.17g r(ub)
ci variances x, sd
display "@ sd " %24.17g r(sd) " " %24.17g r(lb) " " %24.17g r(ub)
ci variances y, sd bonett
display "@ sdbon " %24.17g r(sd) " " %24.17g r(kurtosis) " " %24.17g r(lb) " " %24.17g r(ub)
ci proportions b [fweight = e]
display "@ fwp " r(N) " " %24.17g r(proportion) " " %24.17g r(lb) " " %24.17g r(ub)
ci variances x [fweight = e]
display "@ fwv " r(N) " " %24.17g r(Var) " " %24.17g r(lb) " " %24.17g r(ub)
ci means k [fweight = e], poisson
display "@ fwpois " r(N) " " %24.17g r(mean) " " %24.17g r(lb) " " %24.17g r(ub)
cii means 50 10 3
display "@ cii_m " r(N) " " %24.17g r(se) " " %24.17g r(lb) " " %24.17g r(ub)
cii means 1 10 3
display "@ cii_m1 " r(N) " " %24.17g r(se) " " r(lb)
cii proportions 60 18, wilson
display "@ cii_p " %24.17g r(se) " " %24.17g r(lb) " " %24.17g r(ub)
cii proportions 20 0, agresti level(90)
display "@ cii_p0 " %24.17g r(lb) " " %24.17g r(ub)
cii proportions 20 20
display "@ cii_pn " %24.17g r(lb) " " %24.17g r(ub)
cii means 120 30, poisson
display "@ cii_pois " %24.17g r(mean) " " %24.17g r(lb) " " %24.17g r(ub)
cii variances 50 9, sd
display "@ cii_sd " %24.17g r(sd) " " %24.17g r(lb) " " %24.17g r(ub)
cii variances 50 9 4, bonett
display "@ cii_bon " %24.17g r(kurtosis) " " %24.17g r(lb) " " %24.17g r(ub)
