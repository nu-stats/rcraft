* RCraft's Stata corpus (P2B.10): made-up data; Stata 19's results are in made.recorded
* commands: expand preserve restore set tokenize
* (data the do-file makes itself: set obs, expand, preserve and restore, tokenize, one-line if and else, _rc after capture, quietly)
* tolerance: 1e-8
version 19
clear
set obs 5
generate double v = mod(_n * 7, 5) / 4 + _n / 8
summarize v
display "@ obs_sum " %24.17g r(sum)
set obs 7
count if missing(v)
display "@ obs_new_missing " %24.17g r(N)
replace v = _n / 3 if missing(v)
generate k = cond(_n == 2, 0, cond(_n == 3, 2.6, cond(_n == 6, ., 2)))
expand k if _n != 5, generate(dup)
display "@ exp_N " %24.17g _N
summarize v if dup == 1
display "@ exp_dup_sum " %24.17g r(sum)
display "@ exp_8th_v " %24.17g v[8]
display "@ exp_9th_v " %24.17g v[9]
display "@ exp_last_v " %24.17g v[_N]
preserve
keep if dup == 1
display "@ pre_N " %24.17g _N
restore
display "@ post_N " %24.17g _N
summarize v
display "@ post_sum " %24.17g r(sum)
tokenize "3 1.5 , 2"
display "@ tok_sum " %24.17g `1' + `2' + `4'
local n = 0
if _N > 10 local n = 1
else local n = 2
display "@ if_else " %24.17g `n'
if _N < 10 local m = 1
else local m = 2
display "@ else_ran " %24.17g `m'
capture summarize nosuchvar
local failed = (_rc != 0)
display "@ rc_failed " %24.17g `failed'
capture summarize v
display "@ rc_ok " %24.17g _rc
quietly summarize v if dup == 0
display "@ quiet_mean " %24.17g r(mean)
