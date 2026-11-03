* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in estout.recorded
* commands: eststo estadd
* (eststo and estadd from the SSC estout package, as the replication packages use them: eststo name, eststo name: and eststo alone, est1 est2 ...; estadd scalar and local, on the active results and their stored copy; eststo clear and drop)
* tolerance: 1e-8
version 19
clear
set obs 30
generate x = mod(_n * 7, 11)
generate y = x + mod(_n * 3, 5)
regress y x
eststo m1
estadd scalar a = 5
display "@ active " e(a) " " e(N)
regress y
display "@ other " e(a)
estimates restore m1
display "@ restored " e(a) " " _b[x]
eststo m2: regress y x if x > 2
display "@ prefix " e(N) " " _b[x] " " ("`e(_estimates_name)'" == "m2")
quietly summarize y
estadd scalar ym = r(mean)
estadd scalar ysd = r(sd)
display "@ ym " e(ym) " " e(ysd)
regress y
estimates restore m2
display "@ ym_stored " e(ym) " " e(ysd)
regress y
eststo
display "@ auto " ("`e(_estimates_name)'" == "est3")
quietly regress y x
eststo
display "@ auto2 " ("`e(_estimates_name)'" == "est4")
estadd local tag "yes"
display "@ local " ("`e(tag)'" == "yes")
estadd scalar a = 2, replace
display "@ replaced " e(a)
regress y x if x < 8
estimates store s1
estadd scalar b2 = 3
regress y
estimates restore s1
display "@ plain_store " e(b2) " " e(N)
eststo drop m1
capture estimates restore m1
display "@ dropped " _rc
eststo clear
capture estimates restore m2
display "@ cleared " _rc
