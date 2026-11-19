* RCraft's Stata corpus (P2C.12y, batch A1): made-up data; Stata 19's results are in fillin.recorded
* commands: fillin
* (fillin: every combination of the values, the _fillin marker, the order afterwards, missing and string levels, other variables missing in the added rows, repeated combinations kept, labels kept, and the errors with their return codes)
version 19
set linesize 255
clear
input str2 g t y x
"a" 1 10 1.5
"a" 3 30 .
"b" 2 20 2.5
"c" 1 15 3
"c" 2 . 4
end
fillin g t
display "@ n1 " _N
forvalues i = 1/9 {
    display "@ r1_`i' " strpos("abc", g[`i']) " " t[`i'] " " y[`i'] " " x[`i'] " " _fillin[`i']
}
quietly summarize _fillin
display "@ sum1 " r(sum) " " r(mean)
capture fillin g t
display "@ rc_again " _rc
drop _fillin
clear
input a b c v
2 1 . 7
1 2 1 8
2 . 1 9
1 1 2 6
end
fillin a b c
display "@ n2 " _N
local N = _N
forvalues i = 1/`N' {
    display "@ r2_`i' " a[`i'] " " b[`i'] " " c[`i'] " " v[`i'] " " _fillin[`i']
}
clear
input k j
1 1
1 2
2 1
2 2
end
fillin k j
display "@ n3 " _N " " _fillin[1] _fillin[2] _fillin[3] _fillin[4]
clear
input k j m
1 1 5
1 1 6
2 2 7
end
fillin k j
display "@ n4 " _N
local N = _N
forvalues i = 1/`N' {
    display "@ r4_`i' " k[`i'] " " j[`i'] " " m[`i'] " " _fillin[`i']
}
clear
input byte sex age
0 20
1 30
end
label define sexlab 0 "male" 1 "female"
label values sex sexlab
label variable age "the age"
fillin sex age
display "@ n5 " _N " " sex[1] " " age[1] " " sex[4] " " age[4] " " _fillin[1] _fillin[2] _fillin[3] _fillin[4]
display "@ lab5 " ("`: label (sex) 1'" == "female") " " ("`: variable label age'" == "the age")
clear
input k j
1 1
end
capture fillin k
display "@ rc_one " _rc
capture fillin k nosuch
display "@ rc_nosuch " _rc
capture fillin k k
display "@ rc_twice " _rc
capture fillin
display "@ rc_none " _rc
generate _fillin = 1
capture fillin k j
display "@ rc_exists " _rc
drop _fillin
clear
input k j v
1 1 3
2 .a 4
3 . 5
. 1 6
.b 2 7
end
fillin k j
display "@ n6 " _N
local N = _N
forvalues i = 1/`N' {
    local kk = cond(k[`i'] < ., k[`i'], 100*(k[`i']==.) + 101*(k[`i']==.a) + 102*(k[`i']==.b))
    local jj = cond(j[`i'] < ., j[`i'], 100*(j[`i']==.) + 101*(j[`i']==.a))
    display "@ r6_`i' " `kk' " " `jj' " " v[`i'] " " _fillin[`i']
}

* by and bysort are not allowed (r(190)), after a name the data lack (r(111)) and rows not in order (r(5))
clear
input byte g double x
1 1
1 2
2 3
end
sort g
capture by g: fillin g x
display "@ rc_fil_by " _rc " " _N
capture by nosuch: fillin g x
display "@ rc_fil_nosuch " _rc
capture bysort x: fillin g x
display "@ rc_fil_bysort " _rc " " _N
clear
input byte g double x
2 1
1 2
end
capture by g: fillin g x
display "@ rc_fil_unsorted " _rc
capture bysort g: fillin g x
display "@ rc_fil_sorted " _rc " " g[1] " " x[1]
