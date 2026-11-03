* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in outreg2.recorded
* commands: outreg2
* (outreg2, the SSC package, as the replication packages run it: tstat bdec() tdec() parenthesis(tstat) with adjr2, r2, keep() and e(); a column per model, a variable a later model adds, the statistics rows in their order; the .txt read back cell by cell and the .xls's length)
* tolerance: 0
version 19
set linesize 255
clear
set seed 2026
set obs 600
generate id = ceil(_n / 6)
replace id = 101 in 600
generate t = mod(_n - 1, 6) + 1
generate c = ceil(id / 5)
generate q = ceil(runiform() * 15)
generate x = rnormal() + id / 50
generate w = rnormal() + t / 4
generate y = 1 + 0.5 * x - 0.2 * w + rnormal() * 2 + id / 20 + t / 3 + q / 5
generate big = x * 12345.6
generate yb = y * 100000
local o tstat bdec(3) tdec(2) parenthesis(tstat)
regress y x w
outreg2 using or_a.xls, `o' replace
regress yb big
outreg2 using or_a.xls, `o' adjr2
regress y w x if t > 2
outreg2 using or_a.xls, `o' r2 keep(w)
reghdfe y x, absorb(id) cluster(c q)
outreg2 using or_a.xls, `o' e(r2_a_within) adjr2
regress y x w, vce(cluster c)
outreg2 using or_a.xls, `o' adjr2 e(N_clust r2_a)
regress y x
outreg2 using or_b, tstat bdec(2) tdec(3) replace
import delimited using or_a.txt, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ rows " _N
generate len = strlen(v1) + 2 * strlen(v2) + 3 * strlen(v3) + 5 * strlen(v4) + 7 * strlen(v5) + 11 * strlen(v6)
generate stars = (strpos(v2, "***") > 0) + 2 * (strpos(v3, "***") > 0) + 4 * (strpos(v4, "**") > 0) + 8 * (strpos(v5, "*") > 0) + 16 * (strpos(v6, "*") > 0)
quietly summarize len
display "@ len " r(sum)
quietly summarize stars
display "@ stars " r(sum)
display "@ cells " (v3[10] == "536,186.716***") " " (v4[6] == "-0.298**") " " (v5[18] == "0.0396") " " (v6[16] == "21") " " (v1[19] == "t-statistics in parentheses")
import delimited using or_b.txt, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ b " _N " " (v2[4] == "1.10***") " " (v2[5] == "(11.333)")
erase or_a.txt
erase or_a.xls
erase or_b.txt
