* RCraft's Stata corpus (P2C.12y, batch A1): made-up data; Stata 19's results are in expandcl.recorded
* commands: expandcl
* (expandcl: copies of whole clusters, the new cluster number, counts that are zero, negative, missing or fractional, if and in, two cluster variables, and the errors with their return codes)
version 19
set linesize 255
clear
input id byte g double w str3 s
3 2 1 "c"
1 1 2 "a"
4 3 3 "d"
2 1 2 "b"
5 3 3 "e"
6 3 3 "f"
7 4 0 "g"
8 5 . "h"
9 5 . "i"
10 6 2.7 "j"
11 7 -1 "k"
12 . 2 "l"
13 . 2 "m"
end
preserve
expandcl 2, cluster(g) generate(ng)
display "@ n1 " _N
sort id ng
local N = _N
forvalues i = 1/`N' {
    display "@ r1_`i' " id[`i'] " " ng[`i']
}
restore
preserve
expandcl w, cluster(g) generate(ng)
display "@ n2 " _N
sort id ng
local N = _N
forvalues i = 1/`N' {
    display "@ r2_`i' " id[`i'] " " ng[`i']
}
restore
preserve
expandcl 3 if g <= 2, cluster(g) generate(ng)
display "@ n3 " _N
sort id ng
local N = _N
forvalues i = 1/`N' {
    display "@ r3_`i' " id[`i'] " " ng[`i']
}
restore
preserve
expandcl 2 in 1/3, cluster(g) generate(ng)
display "@ n4 " _N
sort id ng
local N = _N
forvalues i = 1/`N' {
    display "@ r4_`i' " id[`i'] " " ng[`i']
}
restore
preserve
expandcl 2, cluster(g s) generate(ng)
display "@ n5 " _N
sort id ng
local N = _N
forvalues i = 1/`N' {
    display "@ r5_`i' " id[`i'] " " ng[`i']
}
restore
preserve
expandcl 1, cluster(g) generate(ng)
display "@ n6 " _N
local N = _N
forvalues i = 1/`N' {
    display "@ r6_`i' " id[`i'] " " ng[`i']
}
restore
preserve
expandcl 2.5 if g == 1, cluster(g) generate(ng)
display "@ n7 " _N
sort id ng
local N = _N
forvalues i = 1/`N' {
    display "@ r7_`i' " id[`i'] " " ng[`i']
}
restore
capture expandcl 2, cluster(g)
display "@ rc_nogen " _rc
capture expandcl 2, generate(ng)
display "@ rc_nocluster " _rc
capture expandcl 2
display "@ rc_neither " _rc
capture expandcl 2, cluster(g) generate(id)
display "@ rc_exists " _rc
capture expandcl s, cluster(g) generate(ng)
display "@ rc_string " _rc
capture expandcl, cluster(g) generate(ng)
display "@ rc_noexp " _rc
generate double v = id
replace v = 3 if id == 2
capture expandcl v, cluster(g) generate(ng)
display "@ rc_const " _rc
capture expandcl 2, cluster(nosuch) generate(ng)
display "@ rc_nosuch " _rc
display "@ after " _N
* by and bysort are not allowed (r(190)), after a name the data lack (r(111)) and rows not in order (r(5))
clear
input byte g double x
1 1
1 2
2 3
end
sort g
capture by g: expandcl 2, cluster(g) generate(ng)
display "@ rc_ecl_by " _rc " " _N
capture by nosuch: expandcl 2, cluster(g) generate(ng)
display "@ rc_ecl_nosuch " _rc
capture bysort x: expandcl 2, cluster(g) generate(ng)
display "@ rc_ecl_bysort " _rc " " _N
clear
input byte g double x
2 1
1 2
end
capture by g: expandcl 2, cluster(g) generate(ng)
display "@ rc_ecl_unsorted " _rc
capture bysort g: expandcl 2, cluster(g) generate(ng)
display "@ rc_ecl_sorted " _rc " " g[1] " " x[1]
