* RCraft's Stata corpus (P2C.12y, batch A1): made-up data; Stata 19's results are in clonevar.recorded
* commands: clonevar
* (clonevar: the values, the storage type (a float keeps its rounding, a byte its range), the variable label and value label, strings, missing values of every kind, if and in, and the errors with their return codes)
version 19
set linesize 255
clear
input byte g float x double y str5 s
1 1.1 10.1 "a"
2 . 20.2 "b"
1 3.5 . ""
2 .c 40.4 "dd"
end
label variable x "The x"
label define gl 1 "one" 2 "two"
label values g gl
clonevar x2 = x
display "@ x2 " %20.15g x2[1] " " %20.15g x2[3] " " (x2[2] == .) " " (x2[4] == .c) " " ("`: variable label x2'" == "The x")
clonevar y2 = y
display "@ y2 " %20.15g y2[1] " " y2[3] " " y2[4]
clonevar g2 = g
display "@ g2 " g2[1] " " g2[2] " " ("`: label (g2) 1'" == "one") " " ("`: label (g2) 2'" == "two")
clonevar s2 = s
display "@ s2 " (s2[1] == "a") " " (s2[3] == "") " " (s2[4] == "dd")
clonevar g3 = g if g == 1
display "@ g3 " g3[1] " " g3[2] " " g3[3] " " g3[4] " " ("`: label (g3) 1'" == "one")
clonevar s3 = s if _n > 1
display "@ s3 " (s3[1] == "") " " (s3[2] == "b") " " (s3[3] == "") " " (s3[4] == "dd")
clonevar y3 = y in 2/3
display "@ y3 " y3[1] " " y3[2] " " y3[3] " " y3[4]
clonevar x3 = x if x > 1 in 1/2
display "@ x3 " x3[1] " " x3[2] " " x3[3] " " x3[4]
clonevar x4 = x if 0
display "@ x4 " x4[1] " " x4[2] " " x4[3] " " x4[4]
display "@ same " (x2[1] == x[1]) " " (y2[1] == y[1])
replace x2 = 99 in 1
display "@ indep " x[1] " " x2[1]
capture clonevar x2 = x
display "@ rc_exists " _rc
capture clonevar z = nosuch
display "@ rc_notfound " _rc
capture clonevar z = x if nosuch > 1
display "@ rc_ifnotfound " _rc
capture clonevar z = x y
display "@ rc_two " _rc
capture clonevar x2 = nosuch y
display "@ rc_two_exists " _rc
capture clonevar z
display "@ rc_noeq " _rc
capture clonevar z =
display "@ rc_empty " _rc
capture clonevar z = x [aw=g]
display "@ rc_weight " _rc
capture clonevar 1z = x
display "@ rc_badname " _rc
capture clonevar x2 = x y
display "@ rc_exists_two " _rc
* by and bysort are not allowed (r(190)), after a name the data lack (r(111)) and rows not in order (r(5))
clear
input byte g double x
1 1
1 2
2 3
end
sort g
capture by g: clonevar z = x
display "@ rc_clone_by " _rc " " _N
capture by nosuch: clonevar z = x
display "@ rc_clone_nosuch " _rc
capture bysort x: clonevar z = x
display "@ rc_clone_bysort " _rc " " _N
clear
input byte g double x
2 1
1 2
end
capture by g: clonevar z = x
display "@ rc_clone_unsorted " _rc
capture bysort g: clonevar z = x
display "@ rc_clone_sorted " _rc " " g[1] " " x[1]
