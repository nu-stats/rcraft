* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in revrs.recorded
* commands: revrs
* (revrs, SSC: a labelled variable's labelled values in order take the codes k ... 1, a value without a label is missing, the new label set rev<name> keeps the texts; without a label (max + 1) - x, missing values kept; replace; Stata's _rc 111, 109 and 110, and rev<name> there already left as it is for a labelled variable without replace)
* tolerance: 0
version 19
clear
input a b c d e f
1 -2 1.5 3 10 2
2 0 2.5 5 20 2
5 3 4 . 30 2
2 . 1.5 4 . 2
end
label define al 1 "one" 2 "two" 3 "three" 4 "four" 5 "five"
label values a al
label define dl 3 "three" 5 "five"
label values d dl
label define el 10 "ten" 20 "twenty" 30 "thirty" 40 "forty"
label values e el
revrs a d e
display "@ a " reva[1] " " reva[2] " " reva[3] " " reva[4]
display "@ d " revd[1] " " revd[2] " " revd[3] " " revd[4]
display "@ e " reve[1] " " reve[2] " " reve[3] " " reve[4]
decode reva, generate(sa)
decode revd, generate(sd)
display "@ la [" sa[1] "][" sa[2] "][" sa[3] "][" sa[4] "][" sd[1] "][" sd[2] "][" sd[4] "]"
revrs b c f
display "@ b " revb[1] " " revb[2] " " revb[3] " " revb[4]
display "@ c " revc[1] " " revc[2] " " revc[3] " " revc[4]
display "@ f " revf[1] " " revf[4]
capture revrs nosuch
display "@ rc1 " _rc
generate str3 s = "x"
capture revrs s
display "@ rc2 " _rc
capture revrs a
display "@ rc3 " _rc
revrs b, replace
display "@ br " b[1] " " b[2] " " b[3] " " b[4]
capture revrs e, replace
display "@ rc4 " _rc
generate g = e
label values g el
revrs g, replace
decode g, generate(sg)
display "@ gr " g[1] " " g[2] " " g[3] " [" sg[1] "][" sg[3] "]"
