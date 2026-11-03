* RCraft's Stata corpus (P2B.10): made-up data; Stata 19's results are in coefplot.recorded
* commands: coefplot
* (coefplot, Ben Jann's SSC package: the rows its gen() leaves, as positions, estimates and interval ends: defaults, baselevels, several plots, offset(), levels(), a logit's normal intervals, order() with empty rows, keep() and headings(), subgraphs)
* tolerance: 1e-7
* (gen() keeps floats: a value R works out from a logit equal to Stata's in all but the last bits can round to the next float, 1.5e-8 apart at 0.24)
version 19
set linesize 255
clear
set obs 60
gen x = mod(_n * 7, 11) / 3
gen w = mod(_n * 5, 13) / 4
gen z = 2 * x
gen g = 1 + mod(_n, 3)
label define gl 1 "low" 2 "mid" 3 "high"
label values g gl
gen h = 1 + mod(_n, 4)
gen y = 1 + 0.5 * x - 0.3 * w + g / 4 + mod(_n * 17, 23) / 10
gen d = y > 3
label variable x "Exposure"
regress y x w i.g
estimates store m1
regress y x i.g
estimates store m2
regress y w i.h
estimates store m3
logit d x w
estimates store m4
coefplot m1, nodraw gen(a_) replace
forvalues i = 1/6 {
  display "@ a`i' " %24.17g a_at[`i'] " " %24.17g a_b[`i'] " " %24.17g a_ll1[`i'] " " %24.17g a_ul1[`i']
}
coefplot m1, baselevels nodraw gen(b_) replace
forvalues i = 1/6 {
  display "@ b`i' " %24.17g b_at[`i'] " " %24.17g b_b[`i'] " " %24.17g b_ll1[`i']
}
coefplot m1 m2 m3, nodraw gen(c_) replace
forvalues i = 1/14 {
  display "@ c`i' " %24.17g c_at[`i'] " " %24.17g c_b[`i'] " " %24.17g c_plot[`i']
}
coefplot (m1, offset(.2)) m4, levels(90) nodraw gen(f_) replace
forvalues i = 1/8 {
  display "@ f`i' " %24.17g f_at[`i'] " " %24.17g f_b[`i'] " " %24.17g f_ll1[`i'] " " %24.17g f_ul1[`i']
}
coefplot m1, drop(_cons) order(. w x . *.g) nodraw gen(o_) replace
forvalues i = 1/6 {
  display "@ o`i' " %24.17g o_at[`i'] " " %24.17g o_b[`i']
}
coefplot m1, keep(x 2.g) headings(x = "H1" 2.g = "H2") nodraw gen(k_) replace
forvalues i = 1/3 {
  display "@ k`i' " %24.17g k_at[`i'] " " %24.17g k_b[`i']
}
coefplot m1 || m3, nodraw gen(s_) replace
forvalues i = 1/10 {
  display "@ s`i' " %24.17g s_at[`i'] " " %24.17g s_b[`i'] " " %24.17g s_by[`i']
}
