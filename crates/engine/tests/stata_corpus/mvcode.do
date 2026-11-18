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
