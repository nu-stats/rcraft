* RCraft's Stata corpus (P2C.12y, batch A2): made-up data; Stata 19's results are in ds.recorded
* commands: ds
* (ds: r(varlist) for a varlist, the data's order and alpha, not (the variables the varlist leaves out), has() and not() with type string, type numeric, varlabel with and without a pattern and vallab, nothing found, and the errors with their return codes; the columns it prints were read against Stata 19's by hand)
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
label variable flag "the other"
label define gl 1 "low" 2 "mid" 3 "high"
label values g gl
ds
display "@ all " ("`r(varlist)'" == "x g s weight_kg flag id")
ds, alpha
display "@ alpha " ("`r(varlist)'" == "flag g id s weight_kg x")
ds x g
display "@ list " ("`r(varlist)'" == "x g")
ds x-s
display "@ range " ("`r(varlist)'" == "x g s")
ds *e*
display "@ star " ("`r(varlist)'" == "weight_kg")
ds x*, not
display "@ not " ("`r(varlist)'" == "g s weight_kg flag id")
ds x g, not
display "@ not2 " ("`r(varlist)'" == "s weight_kg flag id")
ds, has(type string)
display "@ string " ("`r(varlist)'" == "s")
ds, has(type numeric)
display "@ numeric " ("`r(varlist)'" == "x g weight_kg flag id")
ds, not(type string)
display "@ notstring " ("`r(varlist)'" == "x g weight_kg flag id")
ds s x, has(type numeric)
display "@ withlist " ("`r(varlist)'" == "x")
ds, has(type numeric) alpha
display "@ numalpha " ("`r(varlist)'" == "flag g id weight_kg x")
ds, has(varlabel)
display "@ varlabel " ("`r(varlist)'" == "x s flag")
ds, not(varlabel)
display "@ novarlabel " ("`r(varlist)'" == "g weight_kg id")
ds, has(varlabel *X*)
display "@ varl_x " ("`r(varlist)'" == "x")
ds, has(varlabel *x*)
display "@ varl_lower_x " ("`r(varlist)'" == "")
ds, has(varl "*the*")
display "@ varl_the " ("`r(varlist)'" == "flag")
ds, has(varl "The*")
display "@ varl_The " ("`r(varlist)'" == "x")
ds, has(varl "the other")
display "@ varl_exact " ("`r(varlist)'" == "flag")
ds, has(varl the)
display "@ varl_part " ("`r(varlist)'" == "")
ds, has(varl "N?me*")
display "@ varl_q " ("`r(varlist)'" == "s")
ds, not(varl *a*)
display "@ varl_not " ("`r(varlist)'" == "g weight_kg flag id")
ds, has(vallab)
display "@ vallab " ("`r(varlist)'" == "g")
ds, not(vallab)
display "@ novallab " ("`r(varlist)'" == "x s weight_kg flag id")
ds, has(type string) alpha
display "@ stringalpha " ("`r(varlist)'" == "s")
generate byte onlynum = 1
drop s
ds, has(type string)
display "@ nostring " ("`r(varlist)'" == "")
quietly ds x g
display "@ quiet " ("`r(varlist)'" == "x g")
capture ds nosuch
display "@ rc_nosuch " _rc
capture ds, has(type numeric) not(type string)
display "@ rc_both " _rc
