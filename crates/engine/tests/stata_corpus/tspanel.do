* RCraft's Stata corpus (P2C.12, batch H): made-up data; Stata 19's results are in tspanel.recorded
* commands: tsreport tsappend
* (tsreport: the span of the data and its gaps with and without panels, a varlist (one report each, or casewise), detail with r(table), the panel option, if and in, a delta, dates as the time, and r() with its text macros; tsappend: add() and last() with tsfmt() on panels and on one series, the gaps it fills as tsfill does, a delta, text variables, r(add), and the errors with their return codes)
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
generate byte g = mod(id, 2)
replace y = . if id == 4 & t == 2001
xtset id t
* tsreport without a varlist
local k = 0
foreach opt in "" ", panel" "if id > 3" "in 1/12" "in 10/12" {
    local k = `k' + 1
    tsreport `opt'
    display "@ tr`k' " r(N_gaps) " " r(N_obs) " " r(start) " " r(end)
}
local k = 10
foreach opt in ", detail" ", panel detail" "if id > 3, detail" "in 1/11, detail" "if t < 2003, detail" "if t > 2001, detail" "if id != 2, detail" "in 6/20, detail" "in 29/32, detail" {
    local k = `k' + 1
    tsreport `opt'
    display "@ tr`k' " r(N_gaps) " " r(N_obs) " " r(start) " " r(end)
    matrix T = r(table)
    display "@ tr`k'_dim " rowsof(T) " " colsof(T)
    forvalues i = 1/`=rowsof(T)' {
        forvalues j = 1/6 {
            display "@ tr`k'_`i'_`j' " T[`i', `j']
        }
    }
}
display "@ trt [" r(panelvar) "] [" r(timevar) "] [" r(tsfmt) "]"
* a varlist
tsreport x y
display "@ tv1 " r(N_gaps1) " " r(N_obs1) " " r(start1) " " r(end1) " " r(N_gaps2) " " r(N_obs2) " " r(start2) " " r(end2)
display "@ tv2 [" r(var1) "] [" r(var2) "] [" r(panelvar) "] [" r(timevar) "] [" r(tsfmt) "]"
tsreport x y, detail
display "@ tv3 " r(N_gaps1) " " r(N_gaps2) " " r(N_obs2)
matrix T = r(table1)
display "@ tv4 " rowsof(T) " " colsof(T) " " T[2, 2] " " T[2, 6]
matrix T = r(table2)
display "@ tv5 " rowsof(T) " " colsof(T) " " T[3, 1] " " T[3, 2] " " T[3, 6]
tsreport y, panel
display "@ tv6 " r(N_gaps1) " " r(N_obs1)
tsreport y, panel detail
matrix T = r(table1)
display "@ tv7 " rowsof(T) " " T[1, 1] " " T[1, 2] " " T[1, 4] " " T[1, 5]
tsreport x y, casewise
display "@ tv8 " r(N_gaps) " " r(N_obs) " " r(start) " " r(end)
tsreport x y, casewise detail
matrix T = r(table)
display "@ tv9 " rowsof(T) " " colsof(T) " " T[4, 1] " " T[4, 2]
tsreport y if id > 3
display "@ tv10 " r(N_gaps1) " " r(N_obs1)
tsreport x y g, detail
display "@ tv11 " r(N_gaps3) " " r(N_obs3) " " r(N_gaps2)
* one series, and the times as dates
preserve
keep if id == 2
tsset t
tsreport
display "@ ts1 " r(N_gaps) " " r(N_obs) " " r(start) " " r(end) " [`r(panelvar)']"
tsreport, detail
matrix T = r(table)
display "@ ts2 " rowsof(T) " " colsof(T) " " T[1, 1] " " T[1, 2] " " T[1, 5]
tsreport x y
display "@ ts3 " r(N_gaps1) " " r(N_obs2) " [" r(var1) "]"
tsreport x y, casewise
display "@ ts4 " r(N_gaps) " " r(N_obs)
restore
preserve
keep if id == 1
tsset t
tsreport, detail
display "@ ts5 " r(N_gaps) " " r(N_obs)
restore
preserve
format t %td
xtset id t
tsreport x, detail
display "@ td1 " r(N_gaps1) " " r(N_obs1) " [" r(tsfmt) "]"
restore
preserve
format t %tq
xtset id t
tsreport, detail
display "@ tq1 " r(N_gaps) " " r(N_obs) " " r(start) " " r(end) " [" r(tsfmt) "]"
restore
preserve
replace t = t * 2
xtset id t, delta(2)
tsreport, detail
display "@ tdl " r(N_gaps) " " r(N_obs) " " r(start) " " r(end)
matrix T = r(table)
display "@ tdl2 " rowsof(T) " " T[2, 4] " " T[2, 5] " " T[2, 6]
restore
* tsappend
preserve
tsappend, add(2)
display "@ ta1 " r(add) " " _N
quietly count if missing(x)
display "@ ta1m " r(N)
display "@ ta1t " t[6] " " t[7] " " id[6] " " id[16] " " id[17]
quietly summarize t
display "@ ta1s " r(sum) " " r(min) " " r(max)
xtset
restore
preserve
tsappend, add(1)
display "@ ta2 " r(add) " " _N
restore
preserve
format t %td
xtset id t
tsappend, last(28jun1965) tsfmt(td)
display "@ ta3 " r(add) " " _N
tsappend, last(27jun1965) tsfmt(td)
display "@ ta4 " r(add) " " _N
tsappend, last(30jun1965) tsfmt(td)
display "@ ta5 " r(add) " " _N
quietly summarize t
display "@ ta5s " r(sum) " " r(max)
restore
preserve
format t %tq
xtset id t
tsappend, last(2461q3) tsfmt(tq)
display "@ ta6 " r(add) " " _N
tsappend, last(2460q2) tsfmt(tq)
display "@ ta7 " r(add) " " _N
restore
preserve
keep if id == 2
tsset t
tsappend, add(2)
display "@ ta8 " r(add) " " _N " " t[3] " " id[3] " " t[7]
quietly count if missing(id)
display "@ ta8m " r(N)
restore
preserve
keep if id == 2
format t %td
tsset t
tsappend, last(28jun1965) tsfmt(td)
display "@ ta9 " r(add) " " _N
tsappend, last(25jun1965) tsfmt(td)
display "@ ta10 " r(add) " " _N
restore
preserve
replace t = t * 2
xtset id t, delta(2)
tsappend, add(1)
display "@ ta11 " r(add) " " _N " " t[6] " " t[7]
quietly summarize t
display "@ ta11s " r(sum)
restore
preserve
generate str3 s = "abc"
tsappend, add(1)
display "@ ta12 " _N
quietly count if s == ""
display "@ ta12s " r(N)
restore
preserve
tsappend, add(3)
tsreport
display "@ ta13 " r(N_gaps) " " r(N_obs) " " r(end)
restore
* errors
capture tsappend
display "@ rc1 " _rc
capture tsappend, add(0)
display "@ rc2 " _rc
capture tsappend, add(-1)
display "@ rc3 " _rc
capture tsappend, add(1.5)
display "@ rc4 " _rc
capture tsappend, add(1) last(2007) tsfmt(td)
display "@ rc5 " _rc
capture tsappend, last(2007)
display "@ rc6 " _rc
capture tsappend if id > 2, add(1)
display "@ rc7 " _rc
capture tsappend x, add(1)
display "@ rc8 " _rc
capture tsappend, add(1) nosuch
display "@ rc9 " _rc
capture tsreport nosuch
display "@ rc10 " _rc
capture tsreport x, nosuch
display "@ rc11 " _rc
capture tsreport, casewise
display "@ rc12 " _rc
capture tsreport x, casewise
display "@ rc13 " _rc
generate str3 s = "abc"
capture tsreport s
display "@ rc14 " _rc
capture tsreport x [fweight = g]
display "@ rc15 " _rc
xtset id
capture tsreport
display "@ rc16 " _rc
capture tsappend, add(1)
display "@ rc17 " _rc
xtset, clear
capture tsreport
display "@ rc18 " _rc
capture tsappend, add(1)
display "@ rc19 " _rc
