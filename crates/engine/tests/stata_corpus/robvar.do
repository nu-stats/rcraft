* RCraft's Stata corpus (P2C.12, batch C): made-up data; Stata 19's results are in robvar.recorded
* commands: robvar
* (robvar: Levene's W0, Brown and Forsythe's W50 and W10 (10% trimmed mean) with their p values in r(), two and more groups, groups of 20 or more, a string by-variable, if and in, missing values, groups of one, constant data, and the errors with their return codes)
* tolerance: 1e-9
version 19
set linesize 255
clear
set obs 80
generate double x = mod(_n * 37, 101) / 7 + (mod(_n, 5) == 0) * 3
generate int g = 1 + mod(_n, 3)
generate byte g2 = 1 + (_n > 40)
generate double w2 = 0.5 + mod(_n * 3, 7) / 4
generate str2 sg = "a" + string(mod(_n, 2))
replace x = . in 3
replace g = . in 6
label define gl 1 low 2 mid 3 high
label values g gl
robvar x, by(g)
display "@ r1 " r(N) " " %24.17g r(w0) " " %24.17g r(p_w0) " " %24.17g r(w50) " " %24.17g r(p_w50) " " %24.17g r(w10) " " %24.17g r(p_w10) " " r(df_1) " " r(df_2)
robvar x, by(g2)
display "@ r2 " r(N) " " %24.17g r(w0) " " %24.17g r(p_w0) " " %24.17g r(w50) " " %24.17g r(p_w50) " " %24.17g r(w10) " " %24.17g r(p_w10) " " r(df_1) " " r(df_2)
robvar x, by(sg)
display "@ r3 " r(N) " " %24.17g r(w0) " " %24.17g r(p_w0) " " %24.17g r(w50) " " %24.17g r(p_w50) " " %24.17g r(w10) " " %24.17g r(p_w10) " " r(df_1) " " r(df_2)
robvar x if g < 3, by(g)
display "@ r4 " r(N) " " %24.17g r(w0) " " %24.17g r(w50) " " %24.17g r(w10) " " r(df_1) " " r(df_2)
robvar x in 1/12, by(g)
display "@ r5 " r(N) " " %24.17g r(w0) " " %24.17g r(w50) " " %24.17g r(w10) " " r(df_1) " " r(df_2)
generate byte g5 = 1 + mod(_n, 5)
robvar x, by(g5)
display "@ r6 " r(N) " " %24.17g r(w0) " " %24.17g r(p_w0) " " %24.17g r(w50) " " %24.17g r(w10) " " %24.17g r(p_w10) " " r(df_1) " " r(df_2)
generate byte g8 = 1 + floor((_n - 1) / 10)
robvar x, by(g8)
display "@ r7 " r(N) " " %24.17g r(w0) " " %24.17g r(w50) " " %24.17g r(w10) " " r(df_1) " " r(df_2)
generate double xc = 5
robvar xc, by(g)
display "@ r8 " r(N) " " r(w0) " " r(p_w0) " " r(w50) " " r(w10) " " r(df_1) " " r(df_2)
robvar x if g == 1, by(g)
display "@ r9 " r(N) " " r(w0) " " r(p_w0) " " r(df_1) " " r(df_2)
generate byte one = _n == 7
robvar x if one, by(g)
display "@ r10 " r(N) " " r(w0) " " r(df_1) " " r(df_2)
generate double xs = sqrt(_n)
robvar xs, by(g5)
display "@ r11 " r(N) " " %24.17g r(w0) " " %24.17g r(w50) " " %24.17g r(w10)
* errors
capture robvar x
display "@ rc1 " _rc
capture robvar x, by(nosuch)
display "@ rc2 " _rc
capture robvar nosuch, by(g)
display "@ rc3 " _rc
capture robvar sg, by(g)
display "@ rc4 " _rc
capture robvar x, by(g sg)
display "@ rc5 " _rc
capture robvar x [aweight=w2], by(g)
display "@ rc6 " _rc
capture robvar x [fweight=g], by(g)
display "@ rc7 " _rc
capture robvar, by(g)
display "@ rc8 " _rc
