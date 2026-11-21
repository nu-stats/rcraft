* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in hausman.recorded
* commands: hausman
* (hausman after xtreg fe and re: plain, sigmamore, sigmaless, constant, the active results as the second set; r(chi2), r(df), r(p), r(rank); a negative chi2; a name not stored)
* tolerance: 1e-8
version 19
clear
set linesize 255
set seed 2468
set obs 180
generate id = ceil(_n / 6)
generate year = 2010 + mod(_n - 1, 6)
generate u = invnormal(runiform())
bysort id: replace u = u[1]
generate x1 = invnormal(runiform()) + 0.5 * u
generate x2 = runiform() * 3
generate byte z = mod(id, 3) == 0
generate y = 1 + 0.8 * x1 - 0.4 * x2 + 0.5 * z + u + invnormal(runiform())
drop if id == 4 & year >= 2013
drop if id == 7 & year == 2012
drop if id == 11 & year == 2010
xtset id year
quietly xtreg y x1 x2 z, fe
estimates store fe
quietly xtreg y x1 x2 z, re
estimates store re
hausman fe re
display "@ plain " %24.17g r(chi2) " " r(df) " " r(rank) " " %24.17g r(p)
hausman fe re, sigmamore
display "@ more " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
hausman fe re, sigmaless
display "@ less " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
hausman fe ., constant
display "@ const " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
quietly xtreg y x1, fe
estimates store fe1
quietly xtreg y x1, re
hausman fe1 .
display "@ one " %24.17g r(chi2) " " r(df) " " r(rank) " " %24.17g r(p)
capture noisily hausman nosuch .
display "@ rc " _rc
