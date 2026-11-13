* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in savedfloat.recorded
* commands: merge use
* (a float the do-file made stays a float through save, use and merge: replace then rounds it to float, as Stata does)
* tolerance: 0
version 19
clear
set obs 2
generate double q = 9.12
generate k = _n
recast float q, force
replace q = q * 0.01
display "@ a " %21.0g q[1]
save savedfloat_t, replace
clear
set obs 2
generate k = _n
merge 1:1 k using savedfloat_t
replace q = floor((q * 100) * 100) / 100
display "@ b " %21.0g q[1] " " %21.0g q[2]
use savedfloat_t, clear
replace q = q * 3
display "@ c " %21.0g q[1]
erase savedfloat_t.dta
