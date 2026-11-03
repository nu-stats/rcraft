* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in fre.recorded
* commands: fre
* (fre, the SSC package's frequency table: value labels, missing values, a string's empty value, if, several variables; its r())
* tolerance: 0
version 19
clear
set obs 40
generate a = mod(_n, 4)
replace a = . in 5/7
label define al 0 "zero" 1 "one" 3 "three"
label values a al
generate b = mod(_n * 3, 5)
fre a
display "@ a " r(N) " " r(N_valid) " " r(N_missing) " " r(r) " " r(r_valid) " " r(r_missing) " " ("`r(depvar)'" == "a")
fre a b
display "@ ab " r(N) " " r(N_valid) " " r(N_missing) " " r(r) " " r(r_valid) " " ("`r(depvar)'" == "b")
generate str3 s = "b"
replace s = "" in 2/3
replace s = "c" in 30/40
fre s if _n > 1
display "@ s " r(N) " " r(N_valid) " " r(N_missing) " " r(r) " " r(r_valid) " " r(r_missing)
fre b in 1/10
display "@ in " r(N) " " r(r_valid)
