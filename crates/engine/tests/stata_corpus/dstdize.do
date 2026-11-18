* RCraft's Stata corpus (P2C.12, batch C): made-up data; Stata 19's results are in dstdize.recorded
* commands: dstdize
* (dstdize: direct standardization with one by() variable and one stratum variable: the standard population, the crude and adjusted rates, their standard errors and intervals, level(), a group lacking a stratum, a zero-case stratum, numeric and string groups, r() including the group names, and the errors with their return codes)
* tolerance: 1e-6
version 19
set linesize 255
clear
input str5 group byte agegrp int deaths long pop
"A" 1 5 10000
"A" 2 20 12000
"A" 3 60 8000
"B" 1 8 15000
"B" 2 25 10000
"B" 3 40 5000
"C" 1 0 3000
"C" 2 30 20000
end
dstdize deaths pop agegrp, by(group)
display "@ s1 " r(k) " " colsof(r(adj))
forvalues j = 1/3 {
    display "@ g`j' " %24.17g el(r(adj),1,`j') " " %24.17g el(r(crude),1,`j') " " %24.17g el(r(se),1,`j') " " %24.17g el(r(lb_adj),1,`j') " " %24.17g el(r(ub_adj),1,`j') " " el(r(Nobs),1,`j')
}
local by `r(by)'
local c1 `r(c1)'
local c3 `r(c3)'
display `"@ names `by' `c1' `c3'"'
set obs 9
replace group = "C" in 9
replace agegrp = 3 in 9
replace deaths = 55 in 9
replace pop = 21000 in 9
dstdize deaths pop agegrp, by(group)
forvalues j = 1/3 {
    display "@ h`j' " %24.17g el(r(adj),1,`j') " " %24.17g el(r(crude),1,`j') " " %24.17g el(r(se),1,`j') " " %24.17g el(r(lb_adj),1,`j') " " %24.17g el(r(ub_adj),1,`j') " " el(r(Nobs),1,`j')
}
dstdize deaths pop agegrp, by(group) level(90)
display "@ l90 " %24.17g el(r(lb_adj),1,1) " " %24.17g el(r(ub_adj),1,2) " " %24.17g el(r(lb_adj),1,3)
generate byte g2 = 1 + (group == "C")
dstdize deaths pop agegrp, by(g2)
display "@ n1 " r(k) " " %24.17g el(r(adj),1,1) " " %24.17g el(r(adj),1,2) " " %24.17g el(r(se),1,2)
local c1 `r(c1)'
display `"@ nn `c1'"'
dstdize deaths pop agegrp if group != "C", by(group)
display "@ if " r(k) " " %24.17g el(r(adj),1,1) " " %24.17g el(r(adj),1,2)
dstdize deaths pop agegrp in 1/6, by(group)
display "@ in " r(k) " " %24.17g el(r(adj),1,1)
capture dstdize deaths pop agegrp
display "@ rc1 " _rc
capture dstdize deaths pop, by(group)
display "@ rc2 " _rc
capture dstdize deaths pop agegrp, by(group) level(1)
display "@ rc3 " _rc
capture dstdize nosuch pop agegrp, by(group)
display "@ rc4 " _rc
capture dstdize deaths pop agegrp, by(nosuch)
display "@ rc5 " _rc
replace deaths = 30000 in 2
capture dstdize deaths pop agegrp, by(group)
display "@ rc6 " _rc
replace deaths = 20 in 2
replace deaths = . in 3
capture dstdize deaths pop agegrp, by(group)
display "@ rc7 " _rc
