* RCraft's Stata corpus (P2C.12y, batch A1): made-up data; Stata 19's results are in mvcode.recorded
* commands: mvdecode mvencode
* (mvdecode and mvencode: one value, a list and a range, value = .a to .z, several clauses, if and in, string variables ignored, float values, a value already there, a missing code that is not one, and the errors with their return codes)
version 19
set linesize 255
clear
input byte b int i long l float f double d str3 s
1 -99 1 1.1 1 "a"
-99 2 -99 -99 2 "-99"
3 -99 3 . -99 "c"
2 5 -98 2.5 7 "d"
end
generate double d2 = d
mvdecode b i l f d, mv(-99)
display "@ dec1 " b[1] " " b[2] " " b[3] " " i[1] " " i[3] " " l[2] " " f[2] " " d[3] " " (s[2] == "-99")
mvdecode d2, mv(1 = .a \ 2 = .b)
display "@ dec2 " (d2[1] == .a) " " (d2[2] == .b) " " d2[3] " " d2[4]
mvdecode l, mv(-98 1/3 = .c)
display "@ dec3 " (l[1] == .c) " " (l[2] == .) " " (l[3] == .c) " " (l[4] == .c)
mvdecode f, mv(1.1)
display "@ float " f[1] " " f[4]
mvdecode s, mv(-99)
display "@ str " s[2]
mvdecode i in 1/2, mv(2 5)
display "@ in " (i[1] == .) " " (i[2] == .) " " i[4]
mvdecode i if i == 5, mv(5)
display "@ if " (i[4] == .)
mvdecode d, mv(2)
display "@ none " d[1] " " d[2] " " d[4]
mvdecode d, mv(7 = .z)
display "@ z " (d[4] == .z)
display "@ kinds " (b[2] == .) " " (b[2] == .a) " " (d2[1] == .a) " " (d2[1] > .) " " (d2[2] == .b)
capture mvdecode b
display "@ rc_nomv " _rc
capture mvdecode b, mv(.)
display "@ rc_dot " _rc
capture mvdecode b, mv(abc)
display "@ rc_abc " _rc
capture mvdecode nosuch, mv(1)
display "@ rc_nosuch " _rc
capture mvdecode b, mv(1) foo
display "@ rc_option " _rc
capture mvdecode b, mv(1 = 5)
display "@ rc_code " _rc
capture mvdecode b, mv(1/2 \ 3 = .c)
display "@ rc_mixed " _rc
capture mvdecode b, mv(.a)
display "@ rc_mva " _rc
* mvencode
clear
input byte b int i long l float f double d str3 s
. . . . . "a"
1 .a 2 .b 3 "b"
3 .c .z 4 . "c"
end
mvencode b, mv(100)
display "@ enc1 " b[1] " " b[2] " " b[3]
mvencode i f, mv(7)
display "@ enc2 " i[1] " " i[2] " " i[3] " " f[1] " " f[2] " " f[3]
mvencode l, mv(.z = 9)
display "@ enc3 " l[1] " " l[2] " " l[3]
mvencode l, mv(. = 8)
display "@ enc4 " l[1] " " l[2] " " l[3]
mvencode d s, mv(.=5)
display "@ enc5 " d[1] " " d[2] " " d[3] " " (s[1] == "a")
capture mvencode b, mv(100)
display "@ rc_already " _rc
mvencode b, mv(100) override
display "@ enc6 " b[1]
capture mvencode b, mv(3)
display "@ rc_clash " _rc " " b[1]
capture mvencode i l, mv(2)
display "@ rc_clash2 " _rc " " i[2] " " l[2]
clear
input byte b int i long l float f double d
. . . . .
1 1 1 1 1
. . . . .
end
mvencode b, mv(500)
mvencode i, mv(40000)
mvencode l, mv(3000000000)
mvencode f, mv(0.1)
mvencode d, mv(1e300)
display "@ recast " b[1] " " i[1] " " %20.0f l[1]
display "@ recast_b " %20.0f l[3] " " %20.15g f[1] " " (d[1] == 1e300)
generate byte b2 = 1 in 2
mvencode b2 in 1/2, mv(0.5)
display "@ recast2 " b2[1] " " b2[2]
capture mvencode b, mv(.=)
display "@ rc_empty " _rc
capture mvencode b, mv(1/2)
display "@ rc_range " _rc
capture mvencode b
display "@ rc_nomv2 " _rc
capture mvencode b, mv(5) foo
display "@ rc_option2 " _rc
capture mvencode nosuch, mv(5)
display "@ rc_nosuch2 " _rc
capture mvencode b, mv(.a = 1 \ .b = 2 \ . = 3)
display "@ rc_multi " _rc
* mvdecode numlists with a step, to and : forms, commas, reversed ends, and the lists Stata calls invalid
clear
set obs 12
generate double x = _n
replace x = 0.5 in 11
replace x = -2 in 12
save mvcode_nl, replace
local n = 0
foreach form in "1(2)9" "9(-2)1" "1 3(2)9" "1(1)3 7/8" "1 2 to 5" "1 3 to 9" "1 3 : 9" "5(2)1" "3/1" "0(.5)2" "1(1.5)6" "-2(2)4" "1(2)9 4" "1 / 3" "1 (2) 9" "1,3" "1, 3" "1e0(2)9" "0.5(1)2.5" "5 to 8" "1(0)3" "1 3 5 to" "1(2)3(4)5" "1,,3" ",1" "1(2)9 abc" "2 to 2" {
    use mvcode_nl, clear
    capture mvdecode x, mv(`form')
    local rc = _rc
    local mask ""
    forvalues i = 1/12 {
        local mask "`mask'`=(x[`i'] == .)'"
    }
    local ++n
    display "@ nl`n' `rc' `mask'"
}
clear
input double x float f
.1 .1
.2 .2
.3 .3
.7 .7
1.1 1.1
end
save mvcode_nl2, replace
foreach form in "0(.1).3" ".1(.1).3" "0.1(0.2)1.1" "0(.3)1.2" ".6(.5)1.1" {
    foreach v in x f {
        use mvcode_nl2, clear
        mvdecode `v', mv(`form')
        local mask ""
        forvalues i = 1/5 {
            local mask "`mask'`=(`v'[`i'] == .)'"
        }
        local ++n
        display "@ nl`n' `mask'"
    }
}
use mvcode_nl, clear
mvdecode x, mv(1(2)5 = .a \ 7(2)9 = .b)
display "@ nlclause " (x[1] == .a) (x[3] == .a) (x[5] == .a) (x[7] == .b) (x[9] == .b) (x[2] == .)
use mvcode_nl, clear
mvdecode x, mv(1(2)9 = .a \ 3 = .b)
display "@ nlfirst " (x[3] == .a) (x[3] == .b)
erase mvcode_nl.dta
erase mvcode_nl2.dta
* mvencode with more than one clause for a code: the first takes it; only a clause that took a value can recast
clear
set obs 5
generate double x = .
replace x = .a in 2
replace x = .b in 3
replace x = 5 in 4
replace x = .a in 5
save mvcode_me, replace
foreach form in ".a=1 \ .a=2" ".=1 \ .=2" ".a=1 \ .a=1" ".=1 \ .a=2 \ .b=3" ".b=3 \ .a=2 \ .=1" ".a=1 \ .=2 \ .a=3" ".a=1 \" ".a = 1 \ .z = 9" ".a=1 \ 7" "7 \ .a=1" ".a=1 .b=2" ".a .b=2" "\ .a=1" ".a=1 \ \ .b=2" ".A=1" ".c=1" {
    use mvcode_me, clear
    capture mvencode x, mv(`form')
    local rc = _rc
    local ++n
    generate double chk = cond(x == ., -1, cond(x == .a, -2, cond(x == .b, -3, x)))
    display "@ me`n' `rc' " chk[1] " " chk[2] " " chk[3] " " chk[4] " " chk[5]
}
foreach alt in 0 1 {
    foreach form in ".a=1 \ .a=2" ".a=1 \ .a=1" ".a=2 \ .a=1" ".b=1 \ .a=2" {
        clear
        set obs 3
        generate byte b = .
        replace b = .a in 1
        replace b = 2 + `alt' in 2
        replace b = 1 + 3 * `alt' in 3
        capture mvencode b, mv(`form')
        local ++n
        generate double chk = cond(b == ., -1, cond(b == .a, -2, b))
        display "@ me`n' " _rc " " chk[1] " " chk[2] " " chk[3]
    }
}
clear
set obs 2
generate byte b = .
generate int i = .
replace b = .a in 1
replace i = 1 in 1
replace b = 2 in 2
replace i = .a in 2
capture mvencode b i, mv(.a=1 \ .a=2)
generate double cb = cond(b == .a, -2, b)
generate double ci = cond(i == .a, -2, i)
display "@ me_two " _rc " " cb[1] " " ci[1] " " cb[2] " " ci[2]
clear
set obs 2
generate byte b = .
replace b = .a in 2
generate byte b2 = b
generate byte b3 = b
mvencode b, mv(.=1 \ .a=500)
display "@ rcast1 " b[1] " " b[2]
mvencode b2, mv(.=1 \ .=500 \ .a=2)
display "@ rcast2 " b2[1] " " b2[2]
clear
input byte b
.
end
mvencode b, mv(.=1 \ .a=500)
display "@ rcast3 " b[1]
clear
input double x
1
2
end
mvdecode x, mv(1 = .a \)
display "@ dec_trail " (x[1] == .a) " " x[2]
foreach form in "\ 1=.a" "1=.a \ \ 2=.b" "1 \" "1 \ 2 = .a" "\" {
    capture mvdecode x, mv(`form')
    local ++n
    display "@ dec_odd`n' " _rc
}
capture mvencode x, mv(\)
display "@ enc_odd " _rc
capture mvencode x, mv(5 \)
display "@ enc_odd2 " _rc
erase mvcode_me.dta
