* RCraft's Stata corpus (P2C.12, batch I follow-up): made-up data; Stata 19's results are in spearman.recorded
* commands: spearman
* (spearman of two variables: rho and N (the printed p-value is a symmetric beta with the permutation distribution's variance and kurtosis, Stata's to 4 decimals; its r(p) differs in the 7th digit for some samples and is not kept), if, and rho with ties under quietly)
* tolerance: 1e-8
version 19
set linesize 255
foreach n in 4 5 6 7 8 9 10 12 15 18 20 27 30 37 50 60 100 {
    forvalues s = 1/2 {
        clear
        set seed `=`n'*10+`s''
        set obs `n'
        generate x = runiform()
        generate y = x * `s' + runiform()
        spearman y x
        display "@ sp`n'_`s' " r(N) " " %24.17g r(rho)
    }
}
clear
set seed 77
set obs 40
generate x = runiform()
generate y = runiform() - x
spearman x y if _n > 5
display "@ spif " r(N) " " %24.17g r(rho)
replace x = round(x, .1)
quietly spearman y x
display "@ sptie " r(N) " " %24.17g r(rho)
