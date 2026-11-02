* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in checks.recorded
* commands: assert tabulate
* (assert that holds and that is false, r(9), with if and in, rc0, no rows, and missing values; tabulate, generate() with numbers, text, value labels, missing values, if and the missing option, and the indicators' labels)
* tolerance: 0
version 19
import delimited corpus.csv, clear
assert id > 0
capture assert x < 0.5
display "@ a_false " _rc
capture assert y > 0 if g == 1
display "@ a_if " _rc
capture assert x < 0.5 in 1/3
display "@ a_in " _rc
capture assert x < 0.5, rc0
display "@ a_rc0 " _rc
capture assert id > 0 if id > 100
display "@ a_null " _rc
replace x = . in 5
capture assert x < 3
display "@ a_missing " _rc
capture assert x > -10
display "@ a_missing_big " _rc
replace cat3 = . in 4
generate s = cond(g == 1, "b", "a")
replace s = "" in 7
label define cl 1 "one" 2 "two" 3 "three"
label values f cl
tabulate cat3, generate(d)
summarize d1
display "@ t_d1_n " r(N)
display "@ t_d1_sum " r(sum)
summarize d3
display "@ t_d3_sum " r(sum)
local l : variable label d2
display "@ t_d2_label " ("`l'" == "cat3==     2.0000")
tabulate s, generate(t)
summarize t2
display "@ t_s_n " r(N)
display "@ t_s_sum " r(sum)
local l : variable label t1
display "@ t_s_label " ("`l'" == "s==a")
quietly tabulate f if id > 10, generate(fd)
summarize fd1
display "@ t_if_n " r(N)
display "@ t_if_sum " r(sum)
local l : variable label fd3
display "@ t_f_label " ("`l'" == "f==three")
tabulate cat3, generate(m) missing
summarize m4
display "@ t_miss_n " r(N)
display "@ t_miss_sum " r(sum)
local l : variable label m4
display "@ t_miss_label " ("`l'" == "cat3==      .")
clear
set obs 4
generate double v = 12.5
replace v = 100000 in 2
replace v = 123456789 in 3
replace v = -0.001 in 4
tabulate v, generate(e)
local l : variable label e1
display "@ t_neg_label " ("`l'" == "v==    -0.0010")
local l : variable label e2
display "@ t_num_label " ("`l'" == "v==    12.5000")
local l : variable label e3
display "@ t_wide_label " ("`l'" == "v==100000.0000")
local l : variable label e4
display "@ t_e_label " ("`l'" == "v== 1.2346e+08")
