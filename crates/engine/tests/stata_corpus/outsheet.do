* RCraft's Stata corpus (P2C.12y, batch B): made-up data; Stata 19's results are in outsheet.recorded
* commands: outsheet
* (outsheet: tab or comma or delimiter(), names, value labels and strings in quotes, nonames nolabel noquote, numbers in their display formats, missing values, if and in, a variable list, the .out extension, and the errors; each file read back line by line and compared)
version 19
set linesize 255
local t = char(9)
local q = char(34)
clear
input str8 name double x int k float f byte b str20 s
"ann" 1.5 3 .1 1 "plain"
"bob" . 4 2.5 . "has"
"c d" 2.25 . 1e8 3 "x"
"" -3.125 3 -.5 .a "comma, here"
end
replace s = "has " + char(34) + "quote" + char(34) in 2
label define kl 3 "three" 4 "four"
label values k kl
format f %9.2f
format x %8.3f
outsheet using osh_a, replace
local n 0
file open h using osh_a.out, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ a " `n' " " (`"|`l1'|"' == `"|name`t'x`t'k`t'f`t'b`t's|"') " " (`"|`l2'|"' == `"|"ann"`t'1.500`t'"three"`t'0.10`t'1`t'"plain"|"') " " (`"|`l3'|"' == `"|"bob"`t'`t'"four"`t'2.50`t'`t'"has "quote""|"') " " (`"|`l4'|"' == `"|"c d"`t'2.250`t'`t'1.00e+08`t'3`t'"x"|"') " " (`"|`l5'|"' == `"|""`t'-3.125`t'"three"`t'-0.50`t'.a`t'"comma, here"|"')
outsheet using osh_b.csv, comma replace
local n 0
file open h using osh_b.csv, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ b " `n' " " (`"|`l1'|"' == `"|name,x,k,f,b,s|"') " " (`"|`l2'|"' == `"|"ann",1.500,"three",0.10,1,"plain"|"') " " (`"|`l3'|"' == `"|"bob",,"four",2.50,,"has "quote""|"') " " (`"|`l5'|"' == `"|"",-3.125,"three",-0.50,.a,"comma, here"|"')
outsheet name x using osh_c.txt, delimiter(";") replace
local n 0
file open h using osh_c.txt, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ c " `n' " " (`"|`l1'|"' == `"|name;x|"') " " (`"|`l3'|"' == `"|"bob";|"') " " (`"|`l5'|"' == `"|"";-3.125|"')
outsheet using osh_d.txt, nonames noquote nolabel replace
local n 0
file open h using osh_d.txt, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ d " `n' " " (`"|`l1'|"' == `"|ann`t'1.500`t'3`t'0.10`t'1`t'plain|"') " " (`"|`l2'|"' == `"|bob`t'`t'4`t'2.50`t'`t'has "quote"|"') " " (`"|`l3'|"' == `"|c d`t'2.250`t'`t'1.00e+08`t'3`t'x|"') " " (`"|`l4'|"' == `"|`t'-3.125`t'3`t'-0.50`t'.a`t'comma, here|"')
outsheet k f b using osh_e.txt if k == 3, replace
local n 0
file open h using osh_e.txt, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ e " `n' " " (`"|`l1'|"' == `"|k`t'f`t'b|"') " " (`"|`l2'|"' == `"|"three"`t'0.10`t'1|"') " " (`"|`l3'|"' == `"|"three"`t'-0.50`t'.a|"')
outsheet using osh_f.txt in 2/3, replace
local n 0
file open h using osh_f.txt, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ f " `n' " " (`"|`l2'|"' == `"|"bob"`t'`t'"four"`t'2.50`t'`t'"has "quote""|"')
outsheet name using osh_g, replace noquote
local n 0
file open h using osh_g.out, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ g " `n' " " (`"|`l1'|"' == `"|name|"') " " (`"|`l5'|"' == `"||"')
capture outsheet using osh_g
display "@ exists " _rc
capture outsheet nosuchvar using osh_h.txt, replace
display "@ novar " _rc
capture outsheet using osh_h.txt, delimiter(ab) replace
display "@ delim2 " _rc
capture outsheet using osh_h.txt, comma delimiter(";") replace
display "@ both " _rc
capture outsheet using osh_h.txt, bogus replace
display "@ bogus " _rc
capture outsheet name x
display "@ nousing " _rc
capture outsheet name x using osh_h.txt, replace tab
display "@ tab " _rc
outs name using osh_i.txt, replace delim("|")
local n 0
file open h using osh_i.txt, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ i " `n' " " (`"|`l1'|"' == `"|name|"') " " (`"|`l2'|"' == `"|"ann"|"')
outsheet x name using "osh j.txt", replace delim(" ")
local n 0
file open h using "osh j.txt", read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ j " `n' " " (`"|`l1'|"' == `"|x name|"') " " (`"|`l2'|"' == `"|1.500 "ann"|"') " " (`"|`l3'|"' == `"| "bob"|"')
outsheet x if x > 100 using osh_k.txt, replace
local n 0
file open h using osh_k.txt, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ k " `n' " " (`"|`l1'|"' == `"|x|"')
clear
input double d double big double tiny double third float ff
. 123456789012 1e-10 .3333333333333333 .1
end
replace d = 21000
replace third = 1/3
format d %td
generate int dd = 100
generate long ll = 123456
generate double dbl = 1.23456789012345
generate float fl = 1.2345678
outsheet using osh_l.txt, replace
local n 0
file open h using osh_l.txt, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ l " `n' " " (`"|`l2'|"' == `"|30jun2017`t'1.235e+11`t'1.000e-10`t'.33333333`t'.1`t'100`t'123456`t'1.2345679`t'1.234568|"')
generate str5 empty = ""
format dbl %10.3e
format fl %12.4g
format ll %12.0gc
format tiny %5.1f
format big %5.0f
outsheet dbl fl ll tiny big empty using osh_m.txt, replace
local n 0
file open h using osh_m.txt, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ m " `n' " " (`"|`l2'|"' == `"|1.235e+00`t'1.235`t'123456`t'0.0`t'1.2e+11`t'""|"')
erase osh_a.out
erase osh_b.csv
erase osh_c.txt
erase osh_d.txt
erase osh_e.txt
erase osh_f.txt
erase osh_g.out
erase osh_i.txt
erase "osh j.txt"
erase osh_k.txt
erase osh_l.txt
erase osh_m.txt
