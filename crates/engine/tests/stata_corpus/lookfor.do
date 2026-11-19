* RCraft's Stata corpus (P2C.12y, batch A2): made-up data; Stata 19's results are in lookfor.recorded
* commands: lookfor
* (lookfor: r(varlist) for words found in a name or a variable label, capitals ignored, several words (any one), a quoted phrase, nothing found, value labels not searched, and the error with its return code; the table it prints was read against Stata 19's by hand)
version 19
set linesize 255
clear
set obs 6
generate double x = _n
generate int g = 1 + mod(_n, 3)
generate str5 s = "a" + string(mod(_n, 4))
generate float weight_kg = _n
generate byte flag = 1
generate long id = _n
label variable x "The X variable"
label variable s "Name of thing"
label variable id "Identifier of THING"
label define gl 1 "low" 2 "mid" 3 "high"
label values g gl
lookfor weight
display "@ weight " ("`r(varlist)'" == "weight_kg")
lookfor thing
display "@ thing " ("`r(varlist)'" == "s id")
lookfor THING
display "@ THING " ("`r(varlist)'" == "s id")
lookfor x
display "@ x " ("`r(varlist)'" == "x")
lookfor the x
display "@ the_x " ("`r(varlist)'" == "x")
lookfor "the x"
display "@ phrase " ("`r(varlist)'" == "x")
lookfor "of thing"
display "@ phrase2 " ("`r(varlist)'" == "s id")
lookfor identifier name
display "@ any " ("`r(varlist)'" == "s id")
lookfor g
display "@ g " ("`r(varlist)'" == "g s weight_kg flag id")
lookfor nosuchthing
display "@ nothing " ("`r(varlist)'" == "")
lookfor low
display "@ valuelabel " ("`r(varlist)'" == "")
lookfor gl
display "@ valuelabelname " ("`r(varlist)'" == "")
quietly lookfor flag
display "@ quiet " ("`r(varlist)'" == "flag")
capture lookfor
display "@ rc_none " _rc
