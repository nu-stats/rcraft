* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in tabstat.recorded
* commands: tabstat
* (tabstat's statistics through save: every statistic, unweighted and with fweights and aweights; by() groups with a missing value, missing, casewise, nototal, if and in; q and median; the matrices' sizes)
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 41
set obs 60
generate g = 1 + mod(_n, 3)
replace g = . in 7
label define gl 1 "first" 2 "second group" 3 "A rather long label here"
label values g gl
generate double x = round(100*runiform(), .01)
generate double y = 50 + 20*rnormal()
replace y = . in 3/6
generate f = 1 + mod(_n, 4)
generate double a = .5 + runiform()
tabstat x y, statistics(n mean sd variance min max range sum cv semean skewness kurtosis) save
forvalues i = 1/12 {
    display "@ all`i' " %24.17g el(r(StatTotal), `i', 1) " " %24.17g el(r(StatTotal), `i', 2)
}
tabstat x y, statistics(p1 p5 p10 p25 median p75 p90 p95 p99 iqr q) save
forvalues i = 1/13 {
    display "@ pct`i' " %24.17g el(r(StatTotal), `i', 1) " " %24.17g el(r(StatTotal), `i', 2)
}
display "@ size " rowsof(r(StatTotal)) " " colsof(r(StatTotal))
tabstat x y [fweight = f], statistics(n mean sd variance sum skewness kurtosis semean p10 p50 p90 iqr) save
forvalues i = 1/12 {
    display "@ fw`i' " %24.17g el(r(StatTotal), `i', 1) " " %24.17g el(r(StatTotal), `i', 2)
}
tabstat x y [aweight = a], statistics(n mean sd variance sum skewness kurtosis semean p10 p50 p90 iqr) save
forvalues i = 1/12 {
    display "@ aw`i' " %24.17g el(r(StatTotal), `i', 1) " " %24.17g el(r(StatTotal), `i', 2)
}
tabstat x y, by(g) statistics(mean sd n p50) save
forvalues k = 1/3 {
    forvalues i = 1/4 {
        display "@ g`k'_`i' " %24.17g el(r(Stat`k'), `i', 1) " " %24.17g el(r(Stat`k'), `i', 2)
    }
}
forvalues i = 1/4 {
    display "@ gtot`i' " %24.17g el(r(StatTotal), `i', 1) " " %24.17g el(r(StatTotal), `i', 2)
}
tabstat x, by(g) statistics(n mean) missing save
display "@ miss " el(r(Stat4), 1, 1) " " %24.17g el(r(Stat4), 2, 1) " " el(r(StatTotal), 1, 1) " " %24.17g el(r(StatTotal), 2, 1)
tabstat x y [aweight = a], by(g) statistics(mean sd) columns(statistics) save
display "@ gaw " %24.17g el(r(Stat2), 1, 1) " " %24.17g el(r(Stat2), 2, 2) " " %24.17g el(r(StatTotal), 2, 1)
tabstat x y, casewise statistics(n mean) save
display "@ case " el(r(StatTotal), 1, 1) " " el(r(StatTotal), 1, 2) " " %24.17g el(r(StatTotal), 2, 2)
tabstat x y if g == 2 in 10/50, statistics(n mean max) save
display "@ ifin " el(r(StatTotal), 1, 1) " " %24.17g el(r(StatTotal), 2, 1) " " %24.17g el(r(StatTotal), 3, 2)
tabstat x, by(g) nototal statistics(sum) save
display "@ notot " %24.17g el(r(Stat1), 1, 1) " " %24.17g el(r(Stat3), 1, 1)
generate double one = cond(_n == 1, 5, .)
tabstat one, statistics(n mean sd skewness sum) save
display "@ one " el(r(StatTotal), 1, 1) " " el(r(StatTotal), 2, 1) " " el(r(StatTotal), 3, 1) " " el(r(StatTotal), 4, 1) " " el(r(StatTotal), 5, 1)
quietly tabstat x, statistics(max) save
display "@ quiet " %24.17g el(r(StatTotal), 1, 1)
tabstat x y, statistics(mean sd) by(g) longstub
tabstat x y, statistics(mean) columns(statistics) format(%9.2f)
