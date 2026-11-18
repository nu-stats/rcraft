* RCraft's Stata corpus (P2C.12y, batch B): made-up data; Stata 19's results are in fdasave.recorded
* commands: fdasave fdause
* (fdasave and fdause: a SAS transport file written and read back, numbers, extended missing values, strings, dates, variable labels and value labels (formats.xpf, vallabfile()), the names cut to 8 characters (rename, a name that is taken), if, replace, and the errors with their return codes)
version 19
set linesize 255
clear
input double x float y byte z str6 s double d
1.5 2 3 "ab" 0
-2 . 1 "longer" 1
1e10 .a 0 "" 21915
.1 .z 3 "x y" -365
end
replace x = 1/3 in 4
format d %td
format x %10.2f
label variable x "The x"
label variable s "A string"
label define zl 0 "zero" 1 "one" 3 "three"
label values z zl
fdasave fdz_f1, replace
capture noisily fdasave fdz_f1
display "@ exists " _rc
capture confirm file formats.xpf
display "@ xpf1 " _rc
fdasave fdz_f3 if x > 0, replace vallabfile(none)
capture confirm file formats.xpf
display "@ xpf_none " _rc
fdasave fdz_f4, replace vallabfile(sascode)
capture confirm file fdz_f4.sas
display "@ sas4 " _rc
capture confirm file formats.xpf
display "@ xpf_sas " _rc
fdasave fdz_f5, replace vallabfile(both)
capture confirm file fdz_f5.sas
display "@ sas5 " _rc
capture confirm file formats.xpf
display "@ xpf_both " _rc
fdause fdz_f1, clear
display "@ n " _N " " x[1] " " x[2] " " x[3] " " x[4]
display "@ y " y[1] " " (y[2] == .) " " (y[3] == .a) " " (y[4] == .z)
display "@ z " z[1] " " z[2] " " z[3] " " z[4]
display "@ s " (s[1] == "ab") " " (s[2] == "longer") " " (s[3] == "") " " (s[4] == "x y")
display "@ d " d[1] " " d[2] " " d[3] " " d[4]
decode z, generate(zt)
display "@ lab " (zt[1] == "three") " " (zt[2] == "one") " " (zt[3] == "zero")
fdause fdz_f3, clear
display "@ if " _N " " x[1] " " x[2]
fdause fdz_f1, clear novallabels
capture decode z, generate(zt)
display "@ novallabels " _rc
erase formats.xpf
fdause fdz_f1, clear
capture decode z, generate(zt)
display "@ nofile " _rc
capture noisily fdause fdz_nosuch, clear
display "@ missing " _rc
capture noisily fdause fdz_f1, clear bogus
display "@ bogus " _rc
clear
capture noisily fdasave fdz_f9, replace
display "@ novars " _rc
input double averylongname double abcdefghik double Lower double abcdefghij
1 2 3 4
2 3 4 5
end
capture noisily fdasave fdz_f6, replace
display "@ rename_needed " _rc
fdasave fdz_f6, replace rename
fdause fdz_f6, clear
display "@ renamed " averylon[1] " " abcdefgh[1] " " abcdefg2[1] " " lower[1] " " _N
capture confirm variable averylon
display "@ n1 " _rc
capture confirm variable abcdefgh
display "@ n2 " _rc
capture confirm variable abcdefg2
display "@ n3 " _rc
capture confirm variable lower
display "@ n4 " _rc
capture noisily fdasave fdz_f6, replace
display "@ renamed_again " _rc
capture noisily fdasave fdz_f6 if averylon > 100, replace rename
display "@ noobs " _rc
clear
input double v
0
1
-1
.1
1e-80
-123456.789
2.5e-75
.b
end
fdasave fdz_f7, replace
fdause fdz_f7, clear
display "@ v " v[1] " " v[2] " " v[3] " " v[4] " " v[5] " " v[6] " " v[7] " " (v[8] == .b)
replace v = 1e80 in 6
capture noisily fdasave fdz_f8, replace
display "@ toolarge " _rc
generate str250 big = "a"
replace v = 1 in 6
capture noisily fdasave fdz_f8, replace
display "@ toolong " _rc
* with replace the old file goes first: before the checks that stop (but not "no variables")
clear
input double a double b
1 2
2 3
end
fdasave fdz_f9, replace
capture confirm file fdz_f9.xpt
display "@ made " _rc
capture noisily fdasave fdz_f9 if a > 100, replace
display "@ noobs2 " _rc
capture confirm file fdz_f9.xpt
display "@ after_noobs " _rc
fdasave fdz_f9, replace
clear
capture noisily fdasave fdz_f9, replace
capture confirm file fdz_f9.xpt
display "@ after_novars " _rc
input double a double b
1 2
end
fdasave fdz_f9, replace
label define zz 1 "a"
label values a zz
label drop zz
capture noisily fdasave fdz_f9, replace
display "@ droplabel " _rc
capture confirm file fdz_f9.xpt
display "@ after_droplabel " _rc
capture noisily fdasave fdz_f9, replace vallabfile(zzz)
display "@ badvl " _rc
capture noisily fdasave fdz_f9, replace badopt
display "@ badopt " _rc
capture noisily fdasave, replace
display "@ nofile " _rc
capture erase fdz_f1.xpt
capture erase fdz_f3.xpt
capture erase fdz_f4.xpt
capture erase fdz_f4.sas
capture erase fdz_f5.xpt
capture erase fdz_f5.sas
capture erase fdz_f6.xpt
capture erase fdz_f7.xpt
capture erase fdz_f8.xpt
capture erase formats.xpf
capture erase fdz_f9.xpt
