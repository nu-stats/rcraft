* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in tabm.recorded
* commands: tabm
* (tabm, tab_chi from SSC: the variables stacked into a table of variable by values, with if and missing; r(N), r(r) and r(c))
* tolerance: 0
version 19
clear
input a b c d
1 1 0 2
2 1 1 3
1 2 . 4
2 2 1 5
1 1 0 .
2 . 1 2
end
label define al 1 "Male" 2 "Female"
label values a al
label define cl 0 "Active worker" 1 "Other"
label values c cl
tabm a b c d
display "@ a " r(N) " " r(r) " " r(c)
tabm a b c d if c != . & a == 1
display "@ b " r(N) " " r(r) " " r(c)
tabm c d, missing
display "@ c " r(N) " " r(r) " " r(c)
quietly tabm a b
display "@ d " r(N) " " r(r) " " r(c)
