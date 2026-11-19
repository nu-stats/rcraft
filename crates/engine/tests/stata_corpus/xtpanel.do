* RCraft's Stata corpus (P2C.12, batch H): made-up data; Stata 19's results are in xtpanel.recorded
* commands: xtdescribe xtsum xttab
* (xtdescribe, xtsum and xttab on an unbalanced panel with gaps: r() of each (summarize of T_i, the 13 xtsum values, r(results) and r(n_panels)), if and in, one panel, no rows, missing panel and time values, a labelled variable, a float and a double variable, and the errors with their return codes)
* tolerance: 1e-9
version 19
set linesize 255
clear
set obs 40
generate int id = 1 + floor((_n - 1) / 5)
generate int t = 2000 + mod(_n - 1, 5)
drop if (id == 2 & t == 2002) | (id == 3 & t >= 2003) | (id == 5 & t == 2000) | (id == 7 & t == 2001) | (id == 8 & t < 2003)
generate double x = mod(id * 7 + t * 3, 11) + id / 3
generate double y = x * 2 + mod(id * t, 5)
generate float f = id / 7 + t / 100
generate byte g = mod(id, 2)
generate byte h = 1 + mod(id, 3) + 10 * (t == 2003)
label define hl 1 one 2 two 3 three 11 eleven
label values h hl
replace y = . if id == 4 & t == 2001
xtset id t
* xtdescribe: r() is summarize of the panels' T_i
local k = 0
foreach opt in "" ", patterns(3)" ", patterns(0)" "if id > 3" "if id == 1 | id == 8" "in 6/30" {
    local k = `k' + 1
    xtdescribe `opt'
    foreach s in N sum_w mean Var sd skewness kurtosis sum min max p1 p5 p10 p25 p50 p75 p90 p95 p99 {
        display "@ xd`k'_`s' " %24.17g r(`s')
    }
}
* the wide-span form, one column for several periods
xtdescribe, width(3)
display "@ xdw " r(N) " " r(max)
* xtsum: the 13 values of the last variable
local k = 0
foreach v in x y f g h id t {
    local k = `k' + 1
    xtsum `v'
    foreach s in N n Tbar mean sd min max sd_b min_b max_b sd_w min_w max_w {
        display "@ xs`k'_`s' " %24.17g r(`s')
    }
}
xtsum x y g
display "@ xsm " r(N) " " r(n) " " %24.17g r(Tbar) " " %24.17g r(sd_w)
xtsum
display "@ xsall " r(N) " " r(n) " " %24.17g r(Tbar) " " %24.17g r(mean)
local k = 0
foreach cond in "if id > 2" "in 1/9" "if id == 1" "if id > 99" "if t < 2003" {
    local k = `k' + 1
    xtsum x `cond'
    foreach s in N n Tbar mean sd min max sd_b min_b max_b sd_w min_w max_w {
        display "@ xi`k'_`s' " %24.17g r(`s')
    }
}
xtsum x, i(id)
display "@ xsi " r(N) " " r(n)
* xttab: r(results) and r(n_panels)
local k = 0
foreach v in g h x f t id {
    local k = `k' + 1
    xttab `v'
    matrix R = r(results)
    display "@ xtb`k'_n " r(n_panels) " " rowsof(R) " " colsof(R)
    forvalues i = 1/`=rowsof(R)' {
        forvalues j = 1/6 {
            display "@ xtb`k'_`i'_`j' " %24.17g R[`i', `j']
        }
    }
}
xttab x if id > 3
matrix R = r(results)
display "@ xtb_if " r(n_panels) " " rowsof(R) " " %24.17g R[1, 6]
xttab g in 6/20
matrix R = r(results)
display "@ xtb_in " r(n_panels) " " rowsof(R) " " %24.17g R[1, 4]
xttab g, i(id)
display "@ xtb_i " r(n_panels)
* missing panel and time values
preserve
replace id = . in 10
xtsum x y
display "@ xm1 " r(N) " " r(n) " " %24.17g r(Tbar) " " %24.17g r(sd_b)
xttab g
display "@ xm2 " r(n_panels)
xtdescribe
display "@ xm3 " r(N) " " r(sum) " " r(min)
restore
preserve
replace t = . in 1/2
xtdescribe
display "@ xm4 " r(N) " " r(sum) " " r(max)
xtsum x
display "@ xm5 " r(N) " " r(n)
restore
* one panel, and the times as dates
preserve
keep if id == 1
xtsum x
display "@ x1 " r(N) " " r(n) " " r(sd_b) " " %24.17g r(sd)
xtdescribe
display "@ x2 " r(N) " " %24.17g r(sd) " " r(skewness)
restore
preserve
format t %tq
xtset id t
xtdescribe
display "@ xq " r(N) " " r(p50)
xttab t if id < 3
display "@ xq2 " r(n_panels)
restore
* only the panel variable set
preserve
xtset id
xtsum x
display "@ xo1 " r(N) " " r(n)
xttab g
display "@ xo2 " r(n_panels)
capture xtdescribe
display "@ rc_o " _rc
restore
* errors
capture xtsum nosuch
display "@ rc1 " _rc
capture xttab
display "@ rc2 " _rc
capture xttab g x
display "@ rc3 " _rc
capture xttab nosuch
display "@ rc4 " _rc
capture xtdescribe, patterns(-1)
display "@ rc5 " _rc
capture xtdescribe, patterns(x)
display "@ rc6 " _rc
capture xtdescribe x
display "@ rc7 " _rc
capture xtdescribe, nosuch
display "@ rc8 " _rc
capture xtsum x [fweight = g]
display "@ rc9 " _rc
capture xtsum x, nosuch
display "@ rc10 " _rc
capture xttab g, missing
display "@ rc11 " _rc
capture xtdescribe if id > 99
display "@ rc12 " _rc
capture xttab g if id > 99
display "@ rc13 " _rc
generate str3 s = "abc"
capture xttab s
display "@ rc14 " _rc
xtsum s
display "@ xstr " r(N) " " r(n)
xtset, clear
capture xtdescribe
display "@ rc15 " _rc
capture xtsum x
display "@ rc16 " _rc
capture xttab g
display "@ rc17 " _rc
