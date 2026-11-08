* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in regress_exact.recorded
* commands: regress
* (a fit Stata takes as exact: residuals below about 1e-14 of the total sum of squares give rss 0, R-squared 1, no F or log likelihood and standard errors of 0, robust ones kept; a fit just above that keeps its residuals; as a replication package's program finds variables one of which another explains fully)
* tolerance: 1e-6
version 19
set linesize 255
clear
set seed 97
set obs 40
generate double x = runiform()
generate double z = rnormal()
generate double y = 3*x - 2 + 5e-8 * z
regress y x
display "@ zero " e(rss) " " e(r2) " " e(r2_a) " " e(rmse) " " missing(e(F)) " " missing(e(ll)) " " _se[x] " " _se[_cons] " " (e(r2) == 1)
display "@ zero_mss " %20.0g e(mss) " " %20.0g _b[x] " " %20.0g _b[_cons]
regress y x, vce(robust)
display "@ zero_robust " e(rss) " " e(r2) " " %20.0g _se[x] " " %20.0g _se[_cons]
generate float f = x
generate float g = 3 * f - 2
regress f g
display "@ float " e(rss) " " (e(r2) == 1) " " _se[g]
replace y = 3*x - 2 + 2e-7 * z
regress y x
display "@ kept " %20.0g e(rss) " " %20.0g e(rmse) " " %20.0g e(F) " " %20.0g e(ll) " " %20.0g _se[x]
