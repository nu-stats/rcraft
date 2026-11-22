* RCraft's Stata corpus (P2C.12ao): made-up data; Stata 19's results are in runiform_turns.recorded
* (several runiform() in one expression: a row takes its draws in turn, the operands read left to right; in generate, replace, keep, drop, display, scalars and macros)
* tolerance: 0
version 19
clear
set obs 8
set seed 7
generate id = _n
generate double a = runiform() + 1000 * runiform()
generate double b = (runiform() < .5) + 10 * (runiform() < .5)
generate double c = 1 + (runiform() < .5) + (runiform() < invlogit(.3 * a))
generate double d = floor(exp(.5) * 3 * runiform() * (runiform() < .6))
generate double e = runiform() - 1000 * runiform() + 1000000 * runiform()
generate double f = invnormal(runiform()) + 10 * invnormal(runiform())
generate double g = runiform() / (1000 + runiform())
generate double h = runiform() ^ 2 + 1000 * runiform() ^ 2
forvalues i = 1/8 {
    display "@ a`i' " %24.17g a[`i']
    display "@ b`i' " %24.17g b[`i']
    display "@ c`i' " %24.17g c[`i']
    display "@ d`i' " %24.17g d[`i']
    display "@ e`i' " %24.17g e[`i']
    display "@ f`i' " %24.17g f[`i']
    display "@ g`i' " %24.17g g[`i']
    display "@ h`i' " %24.17g h[`i']
}
display "@ next " %24.17g runiform()
* only the selected rows draw
generate double v = 0
replace v = runiform() + 1000 * runiform() if id > 2
forvalues i = 1/8 {
    display "@ v`i' " %24.17g v[`i']
}
display "@ next_v " %24.17g runiform()
* a larger data set: the first, middle and last rows, and the draw after
clear
set obs 500
set seed 20261119
generate double w = runiform() + 1000 * runiform()
display "@ w1 " %24.17g w[1]
display "@ w250 " %24.17g w[250]
display "@ w500 " %24.17g w[500]
display "@ next_w " %24.17g runiform()
* keep and drop
clear
set obs 12
generate id = _n
set seed 7
keep if runiform() + 1000 * runiform() > 500
display "@ keep_n " _N
forvalues i = 1/`=_N' {
    display "@ keep`i' " id[`i']
}
display "@ next_keep " %24.17g runiform()
clear
set obs 12
generate id = _n
set seed 7
drop if runiform() + 1000 * runiform() > 500
display "@ drop_n " _N
forvalues i = 1/`=_N' {
    display "@ drop`i' " id[`i']
}
* display, scalars and macros
set seed 11
display "@ disp " %24.17g runiform() + 1000 * runiform()
set seed 11
scalar s = runiform() - 1000 * runiform()
display "@ scal " %24.17g s
set seed 11
local l = runiform() * 1000 + runiform()
display "@ loc " %24.17g `l'
display "@ next_loc " %24.17g runiform()
