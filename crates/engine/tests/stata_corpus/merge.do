* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in merge.recorded
* commands: merge
* (merge 1:1, m:1, 1:m and m:m, and 1:1 _n: the pairing within a key, the rows only one file has, a variable in both files; update and replace; keep() and its short words; assert(), checked before keep(); generate() and nogenerate; keepusing(); string keys; keys that do not identify the rows)
* tolerance: 0
version 19
clear
set obs 7
generate k = cond(_n <= 3, 1, cond(_n <= 4, 2, 4))
generate u = _n * 10
generate v = 100 + _n
save mg_um, replace
clear
set obs 6
generate k = cond(_n <= 2, 1, cond(_n <= 5, 2, 3))
generate m = _n
generate v = _n
merge m:m k using mg_um
quietly count if _merge == 3
local matched = r(N)
quietly count if _merge == 1
local master = r(N)
quietly count if _merge == 2
local using = r(N)
quietly summarize u if _merge == 3
display "@ mm " _N " " `matched' " " `master' " " `using' " " r(sum)
quietly summarize v
display "@ mm_v " r(sum) " " v[7] " " u[7] " " m[7] " " k[7]
display "@ mm_rows " u[3] " " u[5] " " m[6] " " k[8]
clear
set obs 6
generate k = cond(_n <= 2, 1, cond(_n <= 5, 2, 3))
generate m = _n
merge m:m k using mg_um, keep(match) nogenerate
quietly summarize u
display "@ mm_keep " _N " " r(sum)
clear
set obs 7
generate k = 1 + mod(_n * 3, 5)
generate u = 10 * k + _n
save mg_u1m, replace
clear
set obs 4
generate k = _n + 1
generate m = 100 * k
merge 1:m k using mg_u1m
quietly summarize u if _merge == 3
display "@ one_many " _N " " r(N) " " r(sum)
quietly summarize m
display "@ one_many_m " r(N) " " r(sum) " " k[1] " " k[_N]
clear
set obs 3
generate k = _n
generate w = 10 * _n
replace k = 4 in 3
save mg_um1, replace
clear
set obs 5
generate k = 1 + mod(_n, 3)
generate m = _n
merge m:1 k using mg_um1
quietly summarize w if _merge == 3
display "@ many_one " _N " " r(N) " " r(sum) " " k[1] " " k[_N]
clear
set obs 4
generate a = _n
save mg_un, replace
clear
set obs 6
generate b = _n * 2
merge 1:1 _n using mg_un
quietly summarize a
display "@ by_row " _N " " r(N) " " r(sum) " " a[4] " " b[6]
clear
set obs 5
generate id = _n
generate x = _n * 10
replace x = . in 2
generate y = 1
save mg_uu, replace
clear
set obs 4
generate id = _n
generate x = .
replace x = 1 in 1
replace x = 3 in 3
replace x = 40 in 4
replace id = 6 in 2
merge 1:1 id using mg_uu, update
display "@ update " _N " " x[1] " " x[3] " " x[4] " " _merge[1] " " _merge[3] " " _merge[4]
clear
set obs 4
generate id = _n
generate x = .
replace x = 1 in 1
replace x = 40 in 4
merge 1:1 id using mg_uu, update replace
display "@ replace " x[1] " " x[3] " " x[4] " " _merge[1] " " _merge[3] " " _merge[4]
clear
set obs 4
generate id = _n
generate x = .
replace x = 1 in 1
merge 1:1 id using mg_uu, update keep(match match_update)
display "@ update_keep " _N " " x[1]
clear
set obs 3
generate id = _n + 3
generate z = 1
capture merge 1:1 id using mg_uu, assert(match) keep(match)
display "@ assert " _rc " " _N
drop _merge
merge 1:1 id using mg_uu, keep(mat mas) gen(how)
quietly count if how == 3
display "@ keep_short " _N " " r(N)
clear
set obs 3
generate id = _n
merge 1:1 id using mg_uu, keep(us) keepusing(x)
quietly summarize x
display "@ keep_using " _N " " r(sum)
clear
set obs 2
generate str3 s = ""
replace s = "a" in 1
generate v = _n
save mg_us, replace
clear
set obs 3
generate str3 s = "b"
replace s = "" in 1
replace s = "a" in 3
generate w = 10 * _n
merge 1:1 s using mg_us
quietly count if _merge == 3
display "@ strings " _N " " r(N) " " w[1] " " v[1]
clear
set obs 3
generate k = 1
capture merge 1:1 k using mg_um
display "@ not_unique " _rc
erase mg_um.dta
erase mg_u1m.dta
erase mg_um1.dta
erase mg_un.dta
erase mg_uu.dta
erase mg_us.dta
