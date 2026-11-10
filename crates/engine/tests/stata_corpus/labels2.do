* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in labels2.recorded
* commands: label decode
* (TQN6ZN puts a second value label on a copy of a labelled variable that has no variable label, after replaces, then defines a label that exists: the new labels are used, and label define without add, modify or replace is r(110))
version 19
set linesize 255
clear
set obs 6
generate p = mod(_n, 3)
label define pl 0 "zero" 1 "one" 2 "two"
label values p pl
label variable p "the p"
generate c = p
label values c pl
replace c = 2 if _n == 1
replace c = . if _n == 6
label define cl 0 "nil" 1 "uno" 2 "dos"
label values c cl
decode c, generate(s)
display "@ second " s[1] "|" s[2] "|" s[3] "|" s[6] "|"
display "@ varlab " "`: variable label c'" "|" "`: variable label p'"
capture label define cl 0 "again"
display "@ exists " _rc
decode c, generate(t)
display "@ kept " t[2]
label define cl 0 "again", modify
label values c cl
decode c, generate(u)
display "@ modified " u[3]
