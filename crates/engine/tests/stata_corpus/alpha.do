* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in alpha.recorded
* commands: alpha
* (alpha as a replication package builds survey indexes with generate(): a reversed item, missing items, min(), casewise and its alpha and average covariance, std, asis, if, and which items it reverses)
* tolerance: 1e-13
version 19
set linesize 255
clear
set seed 5
set obs 60
generate f = rnormal()
generate a = round(3 + f + rnormal(), 1)
generate b = round(2 + f + rnormal(), 1)
generate double c = 5 - f + rnormal()
generate d = round(f + 2 * rnormal(), 1)
replace a = . in 3
replace b = . in 3/4
replace c = . in 3
replace c = . in 7
replace d = . in 3
replace d = . in 7
replace d = . in 9
alpha a b c d, gen(s)
summarize s
display "@ scale " %20.0g s[1] " " %20.0g s[2] " " %20.0g s[7] " " missing(s[3]) " " %20.0g r(mean) " " r(N)
display "@ scale_lab " ("`: variable label s'" == "mean(unstandardized items)")
alpha a b c d, gen(s2) min(3)
display "@ min3 " missing(s2[7]) " " %20.0g s2[4] " " %20.0g s2[9]
alpha a b c d, gen(s3) casewise
display "@ case " %20.0g r(alpha) " " %20.0g r(cov) " " r(k) " " missing(s3[4]) " " %20.0g s3[1]
alpha a b c d, gen(s4) std
summarize s4
display "@ std " %20.0g s4[1] " " %20.0g s4[8] " " %20.0g r(mean)
alpha a b c d, gen(s5) asis
display "@ asis " %20.0g s5[1] " " %20.0g s5[8]
alpha a b c d if _n > 10, gen(s6)
display "@ if " missing(s6[5]) " " %20.0g s6[11] " " %20.0g s6[40]
alpha a b c d, std casewise
display "@ stdcase " %20.0g r(alpha) " " %20.0g r(rho)
* which items alpha reverses: the first principal factor decides, not the correlation with the rest
clear
set seed 13
set obs 25
generate f = rnormal()
generate g = rnormal()
generate x1 = round(3 + 0.4*f + rnormal(), 1)
generate x2 = round(3 + 0.3*f - 0.3*g + rnormal(), 1)
generate x3 = round(3 - 0.2*f + 0.4*g + rnormal(), 1)
generate x4 = round(3 + 0.1*f + 0.3*g + rnormal(), 1)
generate x5 = round(3 - 0.3*f + rnormal(), 1)
replace x2 = . if runiform() < .1
replace x4 = . if runiform() < .1
alpha x1 x2 x3 x4 x5, gen(t)
display "@ factor " %20.0g t[1] " " %20.0g t[2] " " %20.0g t[3] " " %20.0g t[25]
