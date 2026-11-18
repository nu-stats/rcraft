* RCraft's Stata corpus (P2C.12, batch C): made-up data; Stata 19's results are in cumul.recorded
* commands: cumul
* (cumul: the ECDF in a new float variable, freq, equal, by() groups with missing values as a group, frequency and analytic weights, if and in, a string variable, r() left alone, and the errors with their return codes. Tied values without equal are checked only through what does not depend on their order: Stata orders them by its own sort)
version 19
set linesize 255
clear
input x k g w2 str2 s
5 3 1 .5 "a"
2 1 1 1.5 "b"
9 2 2 2.2 "c"
. 4 2 1 "d"
7 1 3 .3 "e"
1 5 1 2 "f"
12 2 2 1 "g"
3 3 3 .7 "h"
8 1 1 1.1 "i"
6 2 2 3 "j"
end
quietly summarize x
cumul x, generate(c1)
display "@ r_kept " r(N) " " r(sum)
forvalues i = 1/10 {
    display "@ c1_`i' " %21x c1[`i']
}
local l : variable label c1
display "@ lab `l'"
cumul x, generate(c1f) freq
forvalues i = 1/10 {
    display "@ c1f_`i' " c1f[`i']
}
cumul x [aweight=w2], generate(c2)
forvalues i = 1/10 {
    display "@ c2_`i' " %21x c2[`i']
}
cumul x [aweight=w2], generate(c2f) freq
forvalues i = 1/10 {
    display "@ c2f_`i' " %21x c2f[`i']
}
cumul x [fweight=k], generate(c3) freq
forvalues i = 1/10 {
    display "@ c3_`i' " c3[`i']
}
cumul x [fweight=k], generate(c3b)
forvalues i = 1/10 {
    display "@ c3b_`i' " %21x c3b[`i']
}
cumul x, generate(c4) by(g)
forvalues i = 1/10 {
    display "@ c4_`i' " %21x c4[`i']
}
cumul x, generate(c4f) by(g) freq
forvalues i = 1/10 {
    display "@ c4f_`i' " c4f[`i']
}
cumul x [fweight=k], generate(c5) by(g) equal
forvalues i = 1/10 {
    display "@ c5_`i' " %21x c5[`i']
}
cumul x if x > 3, generate(c6)
forvalues i = 1/10 {
    display "@ c6_`i' " %21x c6[`i']
}
cumul x in 2/6, generate(c7) equal
forvalues i = 1/10 {
    display "@ c7_`i' " %21x c7[`i']
}
cumul x [aweight=w2] if g == 2, generate(c8) by(g) freq
forvalues i = 1/10 {
    display "@ c8_`i' " %21x c8[`i']
}
cumul s, generate(c9)
quietly count if c9 < .
display "@ c9 " r(N)
cumul x if x > 100, generate(c10)
quietly count if c10 < .
display "@ c10 " r(N)
* tied values and equal
cumul g, generate(c11) equal
forvalues i = 1/10 {
    display "@ c11_`i' " %21x c11[`i']
}
cumul g, generate(c12) freq equal
forvalues i = 1/10 {
    display "@ c12_`i' " c12[`i']
}
cumul k, generate(c13)
quietly summarize c13
display "@ c13 " r(N) " " %21x r(mean) " " %21x r(min) " " %21x r(max)
cumul k, generate(c14) by(g)
quietly summarize c14
display "@ c14 " r(N) " " %21x r(mean) " " %21x r(min) " " %21x r(max)
* missing and string by groups, zero and missing weights
clear
input x g str2 s w2
5 1 "a" .5
2 1 "a" 1.5
9 2 "b" 2.2
7 . "b" 1
1 1 "a" .3
12 2 "" 2
3 3 "c" 1
8 . "c" 1.1
6 2 "b" 3
11 3 "c" 0
end
cumul x, generate(d1) by(g)
cumul x, generate(d2) by(s)
cumul x [aweight=w2], generate(d3) freq
cumul x, generate(d4) by(g s)
cumul x [fweight=g], generate(d5) by(g)
forvalues i = 1/10 {
    display "@ d_`i' " %21x d1[`i'] " " %21x d2[`i'] " " %21x d3[`i'] " " %21x d4[`i'] " " %21x d5[`i']
}
* errors
capture cumul x [pweight=w2], generate(e1)
display "@ rc1 " _rc
capture cumul x [iweight=w2], generate(e2)
display "@ rc2 " _rc
capture cumul x [fweight=w2], generate(e3)
display "@ rc3 " _rc
capture cumul x
display "@ rc4 " _rc
capture cumul nosuch, generate(e5)
display "@ rc5 " _rc
capture cumul x g, generate(e6)
display "@ rc6 " _rc
capture cumul x, generate(1x)
display "@ rc7 " _rc
capture cumul x, generate(d1)
display "@ rc8 " _rc
capture cumul x, generate(e9) equal equal
display "@ rc9 " _rc
capture cumul, generate(e10)
display "@ rc10 " _rc
capture cumul x, generate(e11) by(nosuch)
display "@ rc11 " _rc
replace w2 = -1 in 2
capture cumul x [aweight=w2], generate(e12)
display "@ rc12 " _rc
capture cumul x [aweight=nosuch], generate(e13)
display "@ rc13 " _rc
