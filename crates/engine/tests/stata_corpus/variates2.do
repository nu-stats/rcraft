* RCraft's Stata corpus (P2C.11 continued): made-up draws; Stata 19's results are in variates2.recorded
* functions: rt rbeta
* (rt() by the ratio of uniforms at df 1, 2.5, 4 and 30, missing outside 1 to 2^53 - 1 with no draw; rbeta() with a shape of 1: a uniform, the smaller or larger of two, and one uniform inverted; the next uniform after each, which shows the stream is Stata's; rt() agrees to its last bit or so, hence the tolerance)
* tolerance: 1e-13
version 19
set linesize 255
clear
set obs 200
set seed 101
generate double t1 = rt(1)
generate double t4 = rt(4)
generate double t30 = rt(30)
generate double t25 = rt(2.5) in 1/50
display "@ rt " t1[1] " " t1[200] " " t4[7] " " t30[150] " " t25[50] " " (t25[51] >= .)
quietly summarize t4
display "@ rt_moments " r(mean) " " r(sd) " " r(N)
display "@ rt_next " runiform()
generate double r0 = rt(0.5)
display "@ rt_below_1 " (r0[1] >= .) " " runiform()
set seed 202
generate double b1 = rbeta(1, 1)
generate double b2 = rbeta(1, 2)
generate double b3 = rbeta(2, 1)
generate double b4 = rbeta(3, 1)
generate double b5 = rbeta(1, 0.4)
generate double b6 = rbeta(0.5, 1)
display "@ rbeta " b1[3] " " b2[5] " " b3[7] " " b4[9] " " b5[11] " " b6[13]
quietly summarize b2
display "@ rbeta_moments " r(mean) " " r(min) " " r(max)
display "@ rbeta_next " runiform()
generate double b0 = rbeta(0, 1)
display "@ rbeta_invalid " (b0[1] >= .) " " runiform()
set seed 303
generate double e1 = rbeta(1, 0.15) in 1
generate double e2 = rbeta(0.05, 1) in 1
generate double e3 = rbeta(1, 100000) in 1
generate double e4 = rbeta(1, 0.14999) in 1
generate double e5 = rbeta(0.049, 1) in 1
generate double e6 = rbeta(100001, 1) in 1
generate double e7 = rt(1e10) in 1
generate double e8 = rt(9007199254740992) in 1
display "@ domains " e1[1] " " e2[1] " " e3[1] " " (e4[1] >= .) " " (e5[1] >= .) " " (e6[1] >= .) " " e7[1] " " (e8[1] >= .) " " runiform()
