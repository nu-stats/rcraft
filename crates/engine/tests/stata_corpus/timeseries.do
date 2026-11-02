* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in timeseries.recorded
* commands: tsset
* (tsset with a gap, in panels and with delta(); L., F., D. and S. with counts and chains, over every row whatever if keeps; replace that reads its own earlier times, row by row; in regress with lists of operators and a lagged outcome, and predict after; the errors for repeated and non-integer times and for data not in time order)
* tolerance: 1e-6
version 19
clear
set obs 8
generate t = _n
replace t = 10 in 8
generate x = _n^2
replace x = . in 4
generate g = mod(_n, 2)
tsset t
generate lx = L.x
generate fx = F.x
generate dx = D.x
generate sx = S.x
generate l2x = L2.x
generate d2x = D2.x
generate s2x = S2.x
generate ldx = LD.x
generate ldx2 = l.d.x
generate dfx = d.f.x
generate l0x = L0.x
foreach v in lx fx dx sx l2x d2x s2x ldx ldx2 dfx l0x {
  quietly summarize `v'
  display "@ s_`v' " r(N) " " r(sum)
}
generate lxif = L.x if g == 1
quietly summarize lxif
display "@ s_lxif " r(N) " " r(sum)
count if D.x > 0
display "@ count " r(N)
generate z = 1 if L.x > 5
quietly summarize z
display "@ cond " r(N)
gsort -t
capture generate bad = L.x
display "@ unsorted " _rc
sort t
capture regress x L.x
display "@ sorted_again " _rc
clear
set obs 30
generate t = _n
generate x = mod(_n * 7, 11) + _n / 10
generate y = 2 + 0.5 * x + mod(_n * 3, 5)
tsset t
regress y L.x D.x
display "@ reg " e(N) " " _b[L.x] " " _b[D.x] " " _b[_cons]
regress y l.d.x d.f.x
display "@ chain " e(N) " " _b[LD.x] " " _b[FD.x]
regress y L(0/2).x
display "@ list " e(N) " " _b[x] " " _b[L.x] " " _b[L2.x]
regress y L(1/2).(x y)
display "@ lists " e(N) " " _b[L.x] " " _b[L2.x] " " _b[L.y] " " _b[L2.y]
regress y L.x if t > 10
display "@ reg_if " e(N) " " _b[L.x]
regress y L2D.x
display "@ l2d " e(N) " " _b[L2D.x]
regress L.y x
display "@ lagged_y " e(N) " " _b[x]
regress y LD.x S2.x
display "@ seasonal " e(N) " " _b[LD.x] " " _b[S2.x]
predict p
quietly summarize p
display "@ predict " r(N) " " r(mean)
generate m = y
replace m = . in 10/12
replace m = L.m if m == .
display "@ carry " m[9] " " m[10] " " m[11] " " m[12]
generate k = .
replace k = 1 in 1
replace k = L.k + 1 if k == .
display "@ count_up " k[5] " " k[30]
generate mo = t
replace mo = . in 20/22
replace mo = L.mo + 1 if mo == . & L.mo != .
display "@ months " mo[19] " " mo[20] " " mo[22] " " mo[23]
generate w = y
replace w = L2.w if t > 25
display "@ two_back " w[24] " " w[26] " " w[28] " " w[30]
clear
set obs 7
generate id = cond(_n <= 3, 1, cond(_n <= 6, 2, 3))
generate t = _n
replace t = 4 in 3
replace t = _n - 3 if id == 2
replace t = 5 in 7
generate x = 1
replace x = 4 in 2
replace x = 16 in 3
replace x = 100 in 4
replace x = 200 in 5
replace x = 300 in 6
replace x = 7 in 7
gsort -x
tsset id t
display "@ p_sorted " x[1] " " x[4]
generate lx = L.x
generate fx = F.x
generate dx = D.x
foreach v in lx fx dx {
  quietly summarize `v'
  display "@ p_`v' " r(N) " " r(sum)
}
xtset id t
generate l2x = L2.x
quietly summarize l2x
display "@ x_l2x " r(N) " " r(sum)
clear
set obs 5
generate t = 2 * _n
replace t = 12 in 5
generate x = _n
tsset t, delta(2)
generate lx = L.x
quietly summarize lx
display "@ delta " r(N) " " r(sum)
clear
set obs 2
generate t = 1
generate x = _n
capture tsset t
display "@ repeated " _rc
replace t = 1.5 in 1
capture tsset t
display "@ fraction " _rc
