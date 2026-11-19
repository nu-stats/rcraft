* RCraft's Stata corpus (P2C.12, batch I): made-up data; Stata 19's results are in statsby.recorded
* commands: statsby rolling
* (statsby: r() statistics named and not, by() of one and two variables, _b and _se, total, missing, a group the command fails in; rolling: windows, recursive, stepsize, _b)
* tolerance: 1e-8
version 19
set linesize 255
import delimited corpus.csv, clear
preserve
statsby mean=r(mean) n=r(N), by(cat3) clear nodots: summarize x
display "@ s1_n " _N
forvalues i = 1/`=_N' {
    display "@ s1_row`i' " cat3[`i'] " " %24.17g mean[`i'] " " n[`i']
}
restore
preserve
statsby sd=r(sd), by(cat3 g) clear nodots: summarize y
display "@ s2_n " _N
forvalues i = 1/`=_N' {
    display "@ s2_row`i' " cat3[`i'] " " g[`i'] " " %24.17g sd[`i']
}
restore
preserve
statsby _b _se, by(cat3) clear nodots: regress y x
display "@ s3_n " _N
forvalues i = 1/`=_N' {
    display "@ s3_row`i' " cat3[`i'] " " %24.17g _b_x[`i'] " " %24.17g _b_cons[`i'] " " %24.17g _se_x[`i'] " " %24.17g _se_cons[`i']
}
restore
preserve
statsby r(mean) r(max), by(g) total clear nodots: summarize x
display "@ s4_n " _N
forvalues i = 1/`=_N' {
    display "@ s4_row`i' " g[`i'] " " %24.17g _stat_1[`i'] " " %24.17g _stat_2[`i']
}
restore
preserve
statsby b=_b[x] r2=e(r2) n=e(N), clear nodots: regress y x w
display "@ s5 " _N " " %24.17g b[1] " " %24.17g r2[1] " " n[1]
restore
preserve
replace cat3 = . in 1/3
statsby m=r(mean), by(cat3) clear nodots: summarize x
display "@ s6_n " _N
restore, preserve
statsby m=r(mean), by(cat3, missing) clear nodots: summarize x
display "@ s6m_n " _N " " %24.17g m[_N]
restore
preserve
statsby bx=_b[x] ll=e(ll), by(cat3) clear nodots: logit yb x
display "@ s7_n " _N
forvalues i = 1/`=_N' {
    display "@ s7_row`i' " %24.17g bx[`i'] " " %24.17g ll[`i']
}
restore
generate tt = _n
tsset tt
preserve
rolling _b, window(20) clear nodots: regress y x
display "@ r1_n " _N
display "@ r1_first " start[1] " " end[1] " " %24.17g _b_x[1] " " %24.17g _b_cons[1]
display "@ r1_last " start[_N] " " end[_N] " " %24.17g _b_x[_N] " " %24.17g _b_cons[_N]
restore
preserve
rolling mean=r(mean) sd=r(sd), window(12) recursive clear nodots: summarize x
display "@ r2_n " _N
display "@ r2_a " start[1] " " end[1] " " %24.17g mean[1] " " %24.17g sd[1]
display "@ r2_b " start[_N] " " end[_N] " " %24.17g mean[_N] " " %24.17g sd[_N]
restore
preserve
rolling sd=r(sd), window(15) stepsize(7) clear nodots: summarize y
display "@ r3_n " _N
forvalues i = 1/`=_N' {
    display "@ r3_row`i' " start[`i'] " " end[`i'] " " %24.17g sd[`i']
}
restore
* saving() with and without clear: the data stay as they were without it
import delimited corpus.csv, clear
statsby m=r(mean) n=r(N), by(cat3) saving(statsby_1, replace) nodots: summarize x
display "@ sv_data " _N " " cat3[1] " " cat3[_N]
use statsby_1, clear
display "@ sv_file " _N
forvalues i = 1/`=_N' {
    display "@ sv_`i' " cat3[`i'] " " %24.17g m[`i'] " " n[`i']
}
import delimited corpus.csv, clear
capture noisily statsby m=r(mean), by(cat3) clear saving(statsby_2, replace) nodots: summarize x
display "@ rc198 " _rc
capture erase statsby_1.dta
capture erase statsby_2.dta
* rolling over time with gaps: a window is periods of the time variable
import delimited corpus.csv, clear
generate tt = _n
drop if inlist(tt, 4, 5, 6, 13, 40)
tsset tt
rolling mean=r(mean) n=r(N), window(8) clear nodots: summarize y
display "@ rg_n " _N
forvalues i = 1/`=_N' {
    display "@ rg_`i' " start[`i'] " " end[`i'] " " %24.17g mean[`i'] " " n[`i']
}
import delimited corpus.csv, clear
generate tt = _n
drop if inlist(tt, 4, 5, 6, 13, 40)
tsset tt
rolling mean=r(mean), window(8) recursive stepsize(5) clear nodots: summarize y
display "@ rr_n " _N
forvalues i = 1/`=_N' {
    display "@ rr_`i' " start[`i'] " " end[`i'] " " %24.17g mean[`i']
}
import delimited corpus.csv, clear
generate tt = _n
tsset tt
rolling mean=r(mean), window(20) stepsize(10) saving(rolling_1, replace) nodots: summarize y
display "@ rs_data " _N
use rolling_1, clear
display "@ rs_file " _N
forvalues i = 1/`=_N' {
    display "@ rs_`i' " start[`i'] " " end[`i'] " " %24.17g mean[`i']
}
capture erase rolling_1.dta
* rrecursive: the window ends at the last period and its start moves on
import delimited corpus.csv, clear
generate tt = _n
tsset tt
rolling mean=r(mean), window(15) rrecursive stepsize(7) clear nodots: summarize y
display "@ rq_n " _N
forvalues i = 1/`=_N' {
    display "@ rq_`i' " start[`i'] " " end[`i'] " " %24.17g mean[`i']
}
