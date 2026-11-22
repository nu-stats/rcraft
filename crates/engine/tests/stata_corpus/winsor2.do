* RCraft's Stata corpus (P2C.12as, batch N): made-up data; Stata 19's results are in winsor2.recorded
* commands: winsor2
* (winsor2 from SSC: cuts() at Stata's own percentile, trim, replace, by() with missing groups, if and in, suffix(), label, byte, int, long, float and double variables (a whole-number type takes the cut cut short), a replaced integer variable taking a fractional cut, and its error codes)
* tolerance: 1e-13
version 19
set linesize 255
clear
set seed 31
set obs 80
generate id = _n
generate g = mod(_n, 3)
replace g = . in 1/5
generate double x = rnormal()^3
replace x = . in 7/8
replace x = 1.5 if inlist(id, 20, 21, 22)
generate float z = runiform() * 40 - 10
generate byte b = floor(runiform() * 24)
generate int n = floor(runiform() * 1000)
generate long l = floor(runiform() * 100000)
generate str3 s = "abc"
label variable x "the x"
format z %7.2f
preserve
winsor2 x
summarize x_w
display "@ default " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(mean) " " r(N) " " ("`: variable label x_w'" == "the x")
restore
preserve
winsor2 x z, cuts(5 95)
summarize x_w
display "@ cuts5 " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum) " " r(N)
summarize z_w
display "@ cuts5z " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum)
restore
preserve
winsor2 x, cuts(10 90) trim
summarize x_tr
display "@ trim " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum) " " r(N)
restore
preserve
winsor2 x, cuts(0 95) suffix(_c)
summarize x_c
display "@ lo0 " %20.0g r(min) " " %20.0g r(max) " " r(N)
winsor2 x, cuts(5 100) suffix(_d)
summarize x_d
display "@ hi100 " %20.0g r(min) " " %20.0g r(max) " " r(N)
winsor2 x, cuts(50 50) suffix(_e)
summarize x_e
display "@ cut50 " %20.0g r(min) " " %20.0g r(max) " " r(N)
restore
preserve
winsor2 x, cuts(5 95) replace
summarize x
display "@ replace " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum) " " r(N) " " ("`: variable label x'" == "the x")
restore
preserve
winsor2 z if id > 10, cuts(10 90) replace
summarize z
display "@ replif " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum) " " r(N) " " ("`: variable label z'" == "z")
restore
preserve
winsor2 z in 1/50, cuts(10 90) trim replace
summarize z
display "@ replin " %20.0g r(min) " " %20.0g r(max) " " r(N)
restore
preserve
winsor2 x z, cuts(15 85) by(g)
summarize x_w
display "@ by " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum) " " r(N)
summarize z_w
display "@ byz " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum) " " r(N)
restore
preserve
winsor2 x if id > 6, cuts(15 85) by(g) trim suffix(_q)
summarize x_q
display "@ byif " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum) " " r(N)
winsor2 x in 10/60, cuts(10 90) suffix(_r)
summarize x_r
display "@ in " %20.0g r(min) " " %20.0g r(max) " " r(N)
restore
preserve
winsor2 b n l, cuts(10 90)
summarize b_w
display "@ byte " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum)
summarize n_w
display "@ int " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum)
summarize l_w
display "@ long " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum)
restore
preserve
winsor2 b n, cuts(15 85) by(g)
summarize n_w
display "@ intby " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum)
summarize b_w
display "@ byteby " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum)
restore
preserve
winsor2 n, cuts(15 85) replace
summarize n
display "@ intrep " %20.0g r(min) " " %20.0g r(max) " " %20.0g r(sum)
restore
preserve
winsor2 x z, cuts(1.5 98.5) label suffix(_a)
display "@ label " ("`: variable label x_a'" == "the x-Winsor(p1.5,p98.5)") " " ("`: variable label z_a'" == "z-Winsor(p1.5,p98.5)")
winsor2 x, cuts(0 95) trim label suffix(_b)
display "@ label0 " ("`: variable label x_b'" == "the x-Trim(p00,p95)")
winsor2 x, cuts(0.5 99.5) suffix(_u)
summarize x_u
display "@ small " %20.0g r(min) " " %20.0g r(max) " " r(N)
winsor2 x, cuts(10 90) label replace
display "@ labelrep " ("`: variable label x'" == "the x-Winsor(p10,p90)")
restore
preserve
winsor2 x, cuts(0 100) suffix(_v)
capture confirm variable x_v
display "@ nop " _rc
capture winsor2 x, suffix(_w)
display "@ rc_ok " _rc
capture winsor2 x, suffix(_w)
display "@ rc_exists " _rc
capture winsor2 x z, cuts(5 95) suffix(_w)
display "@ rc_exists2 " _rc
capture confirm variable z_w
display "@ rc_noz " _rc
capture winsor2 x, cuts(5) suffix(_p)
display "@ rc_few " _rc
capture winsor2 x, cuts(-1 99) suffix(_p)
display "@ rc_neg " _rc
capture winsor2 x, cuts(5 105) suffix(_p)
display "@ rc_big " _rc
capture winsor2 x, cuts(95 5) suffix(_p)
display "@ rc_dec " _rc
capture winsor2 nosuch
display "@ rc_nov " _rc
capture winsor2
display "@ rc_none " _rc
capture winsor2 s, cuts(5 95) suffix(_p)
display "@ rc_str " _rc
capture winsor2 x, cuts(5 95) replace suffix(_p)
display "@ rc_repsuf " _rc
capture winsor2 x, by(nosuch) suffix(_p)
display "@ rc_by " _rc
restore
