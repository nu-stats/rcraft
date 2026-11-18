* RCraft's Stata corpus (P2C.12, batch C): made-up data; Stata 19's results are in swilk.recorded
* commands: swilk
* (swilk: W, V, z and the p value of the Shapiro-Wilk test for 12 to 2000 untied values, several variables, if and in, rows of dots for fewer than 4 values, and the errors with their return codes. RCraft's W is R's, which differs from Stata's by about 1e-8 relative, hence the tolerance; tied values and 4 to 11 values are refused and not in this file)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 120
generate double a = mod(_n * 37, 101) / 7
generate double e = (_n^2) / 100
generate double c = exp(sin(_n))
generate double f = _n + mod(_n * 17, 11) / 20
generate double ten = 1
generate str2 s = "a"
generate double w2 = 1 + mod(_n, 3)
replace a = a + _n / 1000
replace c = c + _n / 100000
swilk a in 1/40
display "@ a40 " r(N) " " %24.17g r(W) " " %24.17g r(V) " " %24.17g r(z) " " %24.17g r(p)
swilk e in 1/40
display "@ e40 " r(N) " " %24.17g r(W) " " %24.17g r(V) " " %24.17g r(z) " " %24.17g r(p)
swilk a in 1/12
display "@ a12 " r(N) " " %24.17g r(W) " " %24.17g r(V) " " %24.17g r(z) " " %24.17g r(p)
swilk a in 1/25
display "@ a25 " r(N) " " %24.17g r(W) " " %24.17g r(V) " " %24.17g r(z) " " %24.17g r(p)
swilk a
display "@ a120 " r(N) " " %24.17g r(W) " " %24.17g r(V) " " %24.17g r(z) " " %24.17g r(p)
swilk c f
display "@ f120 " r(N) " " %24.17g r(W) " " %24.17g r(V) " " %24.17g r(z) " " %24.17g r(p)
swilk c
display "@ c120 " r(N) " " %24.17g r(W) " " %24.17g r(V) " " %24.17g r(z) " " %24.17g r(p)
swilk e a if _n > 20
display "@ ea100 " r(N) " " %24.17g r(W) " " %24.17g r(V) " " %24.17g r(z) " " %24.17g r(p)
swilk a in 1/2
display "@ n2 " r(N) " " r(W) " " r(V) " " r(z) " " r(p)
swilk s
display "@ str " r(N) " " r(W) " " r(p)
swilk a if a < 0
display "@ n0 " r(N) " " r(W)
capture swilk ten
display "@ rc_const " _rc
capture swilk
display "@ rc_none " _rc
capture swilk nosuch
display "@ rc_nosuch " _rc
capture swilk a, bad
display "@ rc_opt " _rc
capture swilk a [fweight=w2]
display "@ rc_w " _rc
