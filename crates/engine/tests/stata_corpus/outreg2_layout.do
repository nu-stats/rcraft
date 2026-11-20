* RCraft's Stata corpus (P2C.12ap, batch N): made-up data; Stata 19's results are in outreg2_layout.recorded
* commands: outreg2
* (outreg2 in its own layout: no tstat or se (standard errors), its own number format by size, bdec() alone, sdec(), nocons, noobs, nor2, noaster, alpha() with symbol(), stats(), tex alone, word, a .txt and a .doc name; each file read back whole: its length, digits and 11 characters counted, where its first ( and first * stand)
* tolerance: 0
version 19
set linesize 255
clear
program define o2dig
    args name file delim
    preserve
    if "`delim'" == "tab" {
        import delimited using "`file'", delimiters(tab) varnames(nonames) stringcols(_all) clear
    }
    else {
        import delimited using "`file'", delimiters("@") varnames(nonames) stringcols(_all) clear
    }
    local rows = _N
    quietly generate strL all = ""
    foreach v of varlist v* {
        quietly replace all = all + char(9) + `v'
    }
    quietly generate double n = strlen(all)
    quietly generate double pos = n * _n
    quietly summarize n
    local total = r(sum)
    quietly summarize pos
    local line "@ `name' `rows' `total' `=r(sum)'"
    foreach c in "." "," "-" "(" "*" "e" "+" "<" "\" {
        quietly generate double c = n - strlen(subinstr(all, "`c'", "", .))
        quietly summarize c
        local line "`line' `=r(sum)'"
        quietly drop c
    }
    forvalues d = 1/9 {
        quietly generate double c = n - strlen(subinstr(all, "`d'", "", .))
        quietly summarize c
        local line "`line' `=`d' * r(sum)'"
        quietly drop c
    }
    display "`line'"
    restore
end
set obs 60
generate price = 3000 + mod(_n * 137, 97) * 61
generate mpg = 12 + mod(_n * 7, 23)
generate weight = 1800 + mod(_n * 53, 89) * 30
generate byte foreign = mod(_n, 3) == 0
label variable price "Price"
label variable mpg "Mileage (mpg)"
quietly regress price mpg
outreg2 using d1, replace
quietly regress price mpg weight
outreg2 using d1, append
quietly regress price mpg weight foreign
outreg2 using d1, append ctitle(Full)
o2dig d1 d1.txt tab
quietly regress price mpg weight
outreg2 using d2, replace label bdec(2) sdec(3)
o2dig d2 d2.txt tab
outreg2 using d3, replace nocons keep(mpg) addstat(Adj R2, e(r2_a))
o2dig d3 d3.txt tab
outreg2 using d4, replace stats(coef se tstat pval) noaster
o2dig d4 d4.txt tab
outreg2 using d5, replace alpha(0.01, 0.05) symbol(**, *) nor2 noobs
o2dig d5 d5.txt tab
outreg2 using d5b, replace alpha(0.001, 0.01, 0.1) symbol(***, **, *)
o2dig d5b d5b.txt tab
quietly logit foreign mpg weight
outreg2 using d6, replace
o2dig d6 d6.txt tab
quietly regress price mpg weight
outreg2 using d7, replace tex
quietly regress price mpg
outreg2 using d7, append tex
o2dig d7 d7.tex "@"
quietly regress price mpg weight
outreg2 using d8, replace word
o2dig d8 d8.rtf "@"
outreg2 using d8b, replace word tstat bdec(2) tdec(1)
quietly regress price mpg weight foreign
outreg2 using d8b, append word ctitle(Second long title)
o2dig d8b d8b.rtf "@"
o2dig d8c d8b.txt tab
outreg2 using d8d, replace word nocons noobs nor2
o2dig d8d d8d.rtf "@"
quietly regress price mpg weight
outreg2 using d9, replace bdec(2)
o2dig d9 d9.txt tab
outreg2 using d9b, replace sdec(2)
o2dig d9b d9b.txt tab
outreg2 using d9c, replace tstat
o2dig d9c d9c.txt tab
outreg2 using d9d, replace se bdec(1)
o2dig d9d d9d.txt tab
outreg2 using d9e, replace se tdec(1)
o2dig d9e d9e.txt tab
outreg2 using d10, replace stats(coef tstat)
o2dig d10 d10.txt tab
outreg2 using d10b, replace stats(coef ci)
o2dig d10b d10b.txt tab
outreg2 using d10c, replace stats(coef)
o2dig d10c d10c.txt tab
outreg2 using d10d, replace stats(coef pval)
o2dig d10d d10d.txt tab
outreg2 using d11.txt, replace
o2dig d11 d11.txt tab
outreg2 using d11b.doc, replace word
o2dig d11b d11b.doc "@"
outreg2 using d11c.xls, replace
o2dig d11c d11c.txt tab
* the number format over sizes: coefficients and standard errors from 1e-8 to 1e8
clear
set obs 40
generate x = mod(_n * 7, 23) + 1
generate e0 = mod(_n * 13, 17) - 8 + mod(_n, 3)
quietly regress e0 x
predict e, residuals
quietly summarize e
replace e = e / r(sd)
local k = 0
foreach c in 0.00000123456 0.0000999 0.000123456 0.00099996 0.0123456 0.123456 0.99996 0.9995 1.23456 9.9996 12.3456 99.996 123.456 999.96 1234.56 9999.6 12345.6 99999.5 123456.7 1234567.8 12345678.9 {
    local k = `k' + 1
    local a = `c' * 3.7 + 0.011
    generate y`k' = `a' + `c' * x + e * `c' * 0.5
    quietly regress y`k' x
    if `k' == 1 outreg2 using d12, replace
    else outreg2 using d12, append
}
o2dig d12 d12.txt tab
local k = 0
foreach c in 0.1 0.01 0.0001 0.00009996 1000 999999.6 1000000 10000000000 -0.5 -12345.6 -1234567 -0.00005 {
    local k = `k' + 1
    local a = `c' * 3.7 + 0.011
    generate z`k' = `a' + `c' * x + e * `c' * 0.5
    quietly regress z`k' x
    if `k' == 1 outreg2 using d13, replace
    else outreg2 using d13, append
}
o2dig d13 d13.txt tab
local k = 0
foreach c in 1e-6 1e-8 1e-9 1e-11 -1e-8 {
    local k = `k' + 1
    generate w`k' = `c' * x + e * abs(`c') * 0.5
    quietly regress w`k' x
    if `k' == 1 outreg2 using d14, replace
    else outreg2 using d14, append
}
o2dig d14 d14.txt tab
foreach f in d1 d2 d3 d4 d5 d5b d6 d7 d8 d8b d8d d9 d9b d9c d9d d9e d10 d10b d10c d10d d11 d11b d11c d12 d13 d14 {
    capture erase `f'.txt
    capture erase `f'.tex
    capture erase `f'.rtf
    capture erase `f'.doc
    capture erase `f'.xls
}
