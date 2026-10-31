* RCraft's Stata corpus (P2B.10): made-up data; Stata 19's results are in data.recorded
* commands: import generate replace recode sort gsort collapse
* (asdouble: Stata stores a collapse of a float variable as a float, which R does not follow)
* tolerance: 1e-8
version 19
import delimited corpus.csv, clear asdouble
generate x2 = x^2
summarize x2
display "@ x2_sum " %24.17g r(sum)
replace x2 = 0 if x < 0
summarize x2
display "@ x2_mean " %24.17g r(mean)
recode ord (1 = 10) (2 3 = 20), generate(ord2)
summarize ord2
display "@ ord2_mean " %24.17g r(mean)
sort y
display "@ first_y " %24.17g y[1]
display "@ first_id " %24.17g id[1]
gsort -x
display "@ top_x " %24.17g x[1]
display "@ top_id " %24.17g id[1]
collapse (mean) y x (sd) sy = y, by(g)
display "@ c_y1 " %24.17g y[1]
display "@ c_sy2 " %24.17g sy[2]
display "@ c_x2 " %24.17g x[2]
