* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in xtile.recorded
* commands: xtile
* (xtile with nquantiles(), the default of 2, altdef, if, cutpoints() repeated and out of order, an expression, ties that repeat a percentile, and a variable that exists already)
* tolerance: 0
version 19
clear
set obs 30
generate x = mod(_n * 7, 11)
replace x = . in 5
generate g = mod(_n, 2)
generate c = .
replace c = 3 in 1
replace c = 7 in 2
replace c = 5 in 3
replace c = 5 in 4
xtile q4 = x, nquantiles(4)
xtile q3 = x, nq(3)
xtile q2 = x
xtile qa = x, nq(4) altdef
xtile qi = x if g == 1, nq(4)
xtile qc = x, cutpoints(c)
xtile qci = x if g == 1, cutpoints(c)
xtile qe = x * 2 + 1, nq(5)
generate t = 1
replace t = 2 in 1/3
xtile qt = t, nq(4)
xtile qn = x in 3/20, nq(3)
foreach v in q4 q3 q2 qa qi qc qci qe qt qn {
  quietly count if !missing(`v')
  local n = r(N)
  quietly summarize `v'
  display "@ `v' " `n' " " r(sum) " " r(min) " " r(max)
}
display "@ rows " q4[1] " " q3[1] " " qa[2] " " qc[3] " " qci[9]
capture xtile q4 = x
display "@ exists " _rc
