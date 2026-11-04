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
* labels in the rules: a set named after the new variable, the later text
* for a value winning
recode ord (1 = 1 "low") (2 = 2 "mid") (3 = 2 "high"), generate(ord3)
summarize ord3
display "@ ord3_mean " %24.17g r(mean)
decode ord3, generate(ord3s)
generate byte ord3h = ord3s == "high"
summarize ord3h
display "@ ord3_high " %24.17g r(mean)
* destring takes its variable in parentheses too
tostring g, generate(gs)
destring(gs), generate(gn)
summarize gn
display "@ destring_paren_mean " %24.17g r(mean)
egen rmed = rowmedian(x w z yb)
summarize rmed
display "@ rowmedian_mean " %24.17g r(mean)
egen rp25 = rowpctile(x w z yb), p(25)
summarize rp25
display "@ rowpctile_sum " %24.17g r(sum)
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
