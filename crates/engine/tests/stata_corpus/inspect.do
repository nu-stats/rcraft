* RCraft's Stata corpus (P2C.12, batch C): made-up data; Stata 19's results are in inspect.recorded
* commands: inspect
* (inspect: the counts of negative, zero, positive, integer and non-integer values and of missing ones in r(), the number of distinct values (the limit of 99, doubles one unit apart counted once), labelled variables with and without every value in the label, a string variable, if and in, and the error with its return code. The histogram and the printed layout were compared with Stata's log by eye)
version 19
set linesize 255
clear
set obs 300
generate double v = mod(_n * 37, 211) / 3 - 20
generate double avery_long_variable_name = mod(_n, 7) - 3
label variable avery_long_variable_name "A long label for it"
generate double one = 4
generate double sk = cond(_n <= 290, 1, 50)
generate double sk2 = cond(_n <= 295, mod(_n, 10), 90 + _n)
generate byte lab = mod(_n, 4)
label define lab 0 "zero" 1 "one" 2 "two" 3 "three"
label values lab lab
generate byte lab2 = mod(_n, 6)
label define lab2 1 "one"
label values lab2 lab2
generate double allmiss = .
generate str5 s = "a" + string(mod(_n, 4))
generate double pm = mod(_n, 10) - 5 + (_n == 3) * 0.5
replace pm = . in 7/9
replace pm = .a in 10
foreach x in v avery_long_variable_name one sk sk2 lab lab2 allmiss s pm {
    inspect `x'
    display "@ `x' " r(N_neg) " " r(N_negint) " " r(N_0) " " r(N_pos) " " r(N_posint) " " r(N) " " r(N_unique) " " r(N_undoc)
}
inspect v in 1/50
display "@ v50 " r(N_neg) " " r(N_negint) " " r(N_0) " " r(N_pos) " " r(N_posint) " " r(N) " " r(N_unique) " " r(N_undoc)
inspect v in 1/99
display "@ v99 " r(N_neg) " " r(N_negint) " " r(N_0) " " r(N_pos) " " r(N_posint) " " r(N) " " r(N_unique) " " r(N_undoc)
inspect v in 1/100
display "@ v100 " r(N_neg) " " r(N_negint) " " r(N_0) " " r(N_pos) " " r(N_posint) " " r(N) " " r(N_unique) " " r(N_undoc)
inspect v if 0
display "@ v0 " r(N_neg) " " r(N_negint) " " r(N_0) " " r(N_pos) " " r(N_posint) " " r(N) " " r(N_unique) " " r(N_undoc)
inspect lab if lab > 1
display "@ labif " r(N_neg) " " r(N_negint) " " r(N_0) " " r(N_pos) " " r(N_posint) " " r(N) " " r(N_unique) " " r(N_undoc)
inspect sk lab2
display "@ two " r(N_neg) " " r(N_negint) " " r(N_0) " " r(N_pos) " " r(N_posint) " " r(N) " " r(N_unique) " " r(N_undoc)
inspect
display "@ all " r(N_neg) " " r(N_negint) " " r(N_0) " " r(N_pos) " " r(N_posint) " " r(N) " " r(N_unique) " " r(N_undoc)
capture inspect nosuch
display "@ rc1 " _rc
capture inspect v, bad
display "@ rc2 " _rc
