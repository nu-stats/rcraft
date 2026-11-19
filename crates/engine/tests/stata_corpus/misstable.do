* RCraft's Stata corpus (P2C.12y, batch A2): made-up data; Stata 19's results are in misstable.recorded
* commands: misstable
* (misstable summarize: the counts of . and of .a to .z, the values present, minimum and maximum kept in r() for the last variable shown, nothing to show, all, a varlist, if and in, a float, a variable with no value, and the error with its return code; the table was read against Stata 19's by hand)
version 19
set linesize 255
clear
set obs 12
generate double x = _n
generate double y = mod(_n, 4)
generate int g = 1 + mod(_n, 3)
generate str5 s = "a" + string(mod(_n, 4))
generate double w = _n * 2
generate double v = 5
generate double e = .
generate float f = _n / 3
generate double c = _n
replace x = . in 2
replace x = .a in 3
replace x = .b in 4
replace y = . in 5/7
replace g = .z in 5
replace g = .a in 9
replace s = "" in 3
replace w = .a in 1
replace v = .a in 1/2
replace e = .a in 1
replace f = . in 1
misstable summarize
display "@ all " r(N_lt_dot) " " r(N_eq_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max) " " ("`r(vartype)'" == "numeric")
misstable summarize x y
display "@ xy " r(N_lt_dot) " " r(N_eq_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max)
misstable summarize x
display "@ x " r(N_lt_dot) " " r(N_eq_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max)
misstable summarize g
display "@ g " r(N_lt_dot) " " r(N_eq_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max)
misstable summarize w, all
display "@ w_all " r(N_lt_dot) " " r(N_eq_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max)
misstable summarize c, all
display "@ c_all " r(N_lt_dot) " " r(N_eq_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max)
misstable summarize c
display "@ c_none " r(N_lt_dot) " " r(N_eq_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max) " " ("`r(vartype)'" == "none")
misstable summarize s
display "@ s_none " r(N_lt_dot) " " r(K_uniq) " " ("`r(vartype)'" == "none")
misstable summarize e
display "@ e " r(N_lt_dot) " " r(N_eq_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max)
misstable summarize f
display "@ f " r(N_lt_dot) " " r(N_eq_dot) " " %21x r(min) " " %21x r(max)
misstable summarize v
display "@ v " r(N_lt_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max)
misstable summarize x if _n < 4
display "@ x_if " r(N_lt_dot) " " r(N_eq_dot) " " r(N_gt_dot) " " r(K_uniq) " " r(min) " " r(max)
misstable summarize x if _n < 2
display "@ x_if_none " r(N_lt_dot) " " ("`r(vartype)'" == "none")
misstable summarize y in 3/8
display "@ y_in " r(N_lt_dot) " " r(N_eq_dot) " " r(K_uniq) " " r(min) " " r(max)
quietly misstable summarize x
display "@ quiet " r(N_lt_dot) " " r(N_gt_dot)
capture misstable summarize nosuch
display "@ rc_nosuch " _rc
