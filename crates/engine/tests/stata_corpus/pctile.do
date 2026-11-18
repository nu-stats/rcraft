* RCraft's Stata corpus (P2C.12, batch C): made-up data; Stata 19's results are in pctile.recorded
* commands: pctile
* (pctile: cut points of nquantiles() groups in the first rows, genp(), altdef, weights, if and in, the storage type, r(r1) ..., label and format, and the errors with their return codes)
version 19
set linesize 255
clear
input x k g w2 str3 s
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
pctile p = x, nq(4)
display "@ p4 " p[1] " " p[2] " " p[3] " " p[4]
display "@ r4 " r(r1) " " r(r2) " " r(r3)
display "@ hx4 " %21x p[1] " " %21x r(r1)
local l : variable label p
display "@ meta4 `l'"
pctile q = x, nq(10) genp(pp)
display "@ q10 " q[1] " " q[2] " " q[3] " " q[4] " " q[5] " " q[6] " " q[7] " " q[8] " " q[9] " " q[10]
display "@ pp10 " pp[1] " " pp[2] " " pp[9] " " pp[10]
display "@ r10 " r(r1) " " r(r4) " " r(r9)
local l : variable label pp
display "@ ppmeta `l'"
pctile q2 = x, nq(4) altdef
display "@ q2 " q2[1] " " q2[2] " " q2[3] " " q2[4]
pctile q2b = x, nq(10) altdef
display "@ q2b " q2b[1] " " q2b[2] " " q2b[3] " " q2b[5] " " q2b[9]
pctile q3 = x [fweight=k], nq(4)
display "@ q3 " q3[1] " " q3[2] " " q3[3] " " q3[4]
display "@ r3 " r(r1) " " r(r2) " " r(r3)
pctile q3b = x [fweight=k], nq(10)
display "@ q3b " q3b[1] " " q3b[2] " " q3b[3] " " q3b[4] " " q3b[5] " " q3b[6] " " q3b[7] " " q3b[8] " " q3b[9]
pctile q4 = x if g < 3, nq(5)
display "@ q4 " q4[1] " " q4[2] " " q4[3] " " q4[4] " " q4[5]
pctile q5 = x [aweight=w2], nq(4)
display "@ q5 " q5[1] " " q5[2] " " q5[3] " " q5[4]
pctile q5b = x [aweight=w2], nq(10)
display "@ q5b " q5b[1] " " q5b[2] " " q5b[3] " " q5b[4] " " q5b[5] " " q5b[6] " " q5b[7] " " q5b[8] " " q5b[9]
pctile q5c = x [pweight=w2], nq(4)
display "@ q5c " q5c[1] " " q5c[2] " " q5c[3]
pctile q6 = x
display "@ q6 " q6[1] " " q6[2] " " r(r1)
pctile q7 = x in 3/7, nq(2)
display "@ q7 " q7[1] " " q7[2] " " r(r1)
pctile q8 = x+1, nq(4)
display "@ q8 " q8[1] " " q8[2] " " q8[3]
local l : variable label q8
display "@ meta8 `l'"
pctile double q9 = x, nq(3)
display "@ q9 " q9[1] " " q9[2]
pctile int q10 = x, nq(4)
display "@ q10i " q10[1] " " q10[2] " " q10[3]
* extra values: ties of exact fractions, doubles, an empty sample
clear
set obs 9
generate x = _n
generate double xd = _n / 7
generate k = 1 + mod(_n, 3)
pctile a3 = x, nq(3)
pctile a6 = x, nq(6)
pctile a7 = x, nq(7)
pctile a9 = x, nq(9)
display "@ a3 " a3[1] " " a3[2] " " a3[3]
display "@ a6 " a6[1] " " a6[2] " " a6[3] " " a6[4] " " a6[5]
display "@ a7 " a7[1] " " a7[2] " " a7[3] " " a7[4] " " a7[5] " " a7[6]
display "@ a9 " a9[1] " " a9[2] " " a9[3] " " a9[4] " " a9[5] " " a9[6] " " a9[7] " " a9[8]
pctile b3 = x [fweight=k], nq(3)
pctile b6 = x [fweight=k], nq(6)
pctile b10 = x [fweight=k], nq(10)
display "@ b3 " b3[1] " " b3[2] " " b3[3]
display "@ b6 " b6[1] " " b6[2] " " b6[3] " " b6[4] " " b6[5]
display "@ b10 " b10[1] " " b10[2] " " b10[3] " " b10[4] " " b10[5] " " b10[6] " " b10[7] " " b10[8] " " b10[9]
pctile d3 = xd, nq(3)
pctile double d3d = xd, nq(3)
display "@ d3 " %21x d3[1] " " %21x d3[2]
display "@ d3d " %21x d3d[1] " " %21x d3d[2]
display "@ rd " %21x r(r1) " " %21x r(r2)
pctile d5 = xd, nq(5) altdef
display "@ d5 " %21x d5[1] " " %21x d5[2] " " %21x d5[4]
pctile y1 = x if x < 3, nq(7)
display "@ y1 " y1[1] " " y1[2] " " y1[3] " " y1[4] " " y1[5] " " y1[6] " " y1[7]
pctile y3 = x if x < 3, nq(10)
display "@ y3 " y3[1] " " y3[5] " " y3[9]
pctile y4 = x in 2/5, nq(10)
display "@ y4 " y4[1] " " y4[2] " " y4[5] " " y4[9]
* errors
capture pctile e1 = x, nq(1)
display "@ rc1 " _rc
capture pctile e2 = x, nq(0)
display "@ rc2 " _rc
capture pctile e3 = x, nq(2.5)
display "@ rc3 " _rc
capture pctile e4 = x, nq(11)
display "@ rc4 " _rc
capture pctile e5 = x, nq(10)
display "@ rc5 " _rc
capture pctile x = x
display "@ rc6 " _rc
capture pctile e7 = x, genp(x)
display "@ rc7 " _rc
capture pctile e8 = x if x > 100, nq(4)
display "@ rc8 " _rc
capture pctile e9 = x if x > 100, nq(11)
display "@ rc9 " _rc
capture pctile e10 = x [fweight=xd], nq(4)
display "@ rc10 " _rc
capture pctile e11 = x [iweight=k], nq(4)
display "@ rc11 " _rc
capture pctile e12 = x [fweight=k], nq(4) altdef
display "@ rc12 " _rc
capture pctile e13 = nosuch, nq(4)
display "@ rc13 " _rc
capture pctile 1e = x
display "@ rc14 " _rc
capture pctile e15 = x, nq(3) altdef altdef
display "@ rc15 " _rc
capture pctile e16 = x, nq(3) genp(e16)
display "@ rc16 " _rc
generate str2 st = "a"
capture pctile e17 = st
display "@ rc17 " _rc
capture pctile e18 = x [fweight=k] if x > 100
display "@ rc18 " _rc
