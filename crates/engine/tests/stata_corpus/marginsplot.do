* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in marginsplot.recorded
* commands: marginsplot putexcel estimates eststo
* (margins' r(table) and its _at in the model's order, marginsplot leaving r() alone, z after xtreg; the _est_name variables estimates store and eststo keep, and their drops)
* tolerance: 1e-8
version 19
clear
set obs 200
generate id = ceil(_n / 10)
generate t = mod(_n - 1, 10)
generate double a = mod(_n * 0.37, 1)
generate double p = mod(_n * 0.61, 1)
generate double w = mod(_n * 0.23, 1)
generate double y = 0.5 + 0.2 * a - 0.1 * p + 0.3 * a * p + w + sin(_n) * 0.1
regress y w c.a##c.p
margins, at(p=(0 1) w=(0 1) a=(0 1))
matrix T = r(table)
display "@ a " T[1,1] " " T[1,2] " " T[1,3] " " T[1,5] " " T[2,2] " " T[3,2] " " T[7,1] " " T[8,1]
marginsplot
matrix T = r(table)
display "@ b " T[1,2] " " T[5,2] " " T[6,2] " " r(N)
putexcel set "marginsplot_t.xlsx", replace
putexcel A1 = matrix(r(table)), names
matrix T = r(table)
display "@ c " T[1,4] " " colsof(T) " " rowsof(T)
erase marginsplot_t.xlsx
xtset id t
xtreg y c.a##c.p, fe vce(cluster id)
quietly margins, at(a=(0(.5)1) p=(0 1))
matrix T = r(table)
display "@ d " T[1,2] " " T[2,2] " " T[3,2] " " T[8,2] " " missing(T[7,2])
estimates store a1
eststo b1: regress y a if t > 2
display "@ e " _est_a1[1] " " _est_b1[1] " " _est_b1[5] " " _est_b1[200]
estimates drop a1
capture confirm variable _est_a1
display "@ f " _rc
eststo clear
capture confirm variable _est_b1
display "@ g " _rc
