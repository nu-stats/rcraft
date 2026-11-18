* RCraft's Stata corpus (P2C.12y, batch B): made-up data; Stata 19's results are in outfile.recorded
* commands: outfile
* (outfile: the free layout (numbers right-aligned in their display format's width, strings quoted and left-aligned in at least 11, labelled values quoted, two blanks after each field, a new line before a field that would reach column 80 unless wide), comma, noquote, nolabel, runtogether for strings, if, in and a variable list, the errors; each file read back line by line and compared)
version 19
set linesize 255
clear
input str8 name double x int k float f byte b str20 s
"ann" 1.5 3 .1 1 "plain"
"bob" . 4 2.5 . "has"
"c d" 2.25 . 1e8 3 "x y"
"" -3.125 3 -.5 .a "comma, here"
end
replace s = "has " + char(34) + "quote" + char(34) in 2
label define kl 3 "three" 4 "four"
label values k kl
format f %9.2f
format x %8.3f

outfile using ofx_a1, replace
local n 0
file open h using ofx_a1.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o1 " `n' " " (`"|`l1'|"' == `"|"ann"           1.500  "three"        0.10         1  "plain"|"') " " (`"|`l2'|"' == `"|"bob"               .  "four"         2.50         .  "has "quote""|"') " " (`"|`l3'|"' == `"|"c d"           2.250         .   1.00e+08         3  "x y"|"') " " (`"|`l4'|"' == `"|""             -3.125  "three"       -0.50        .a  "comma, here"|"')
outfile using ofx_a2.raw, noquote replace
local n 0
file open h using ofx_a2.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o2 " `n' " " (`"|`l1'|"' == `"|ann           1.500  three          0.10         1  plain|"') " " (`"|`l2'|"' == `"|bob               .  four           2.50         .  has "quote"|"') " " (`"|`l3'|"' == `"|c d           2.250         .   1.00e+08         3  x y|"') " " (`"|`l4'|"' == `"|             -3.125  three         -0.50        .a  comma, here|"')
outfile using ofx_a3.raw, comma replace
local n 0
file open h using ofx_a3.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o3 " `n' " " (`"|`l1'|"' == `"|"ann",1.500,"three",0.10,1,"plain"|"') " " (`"|`l2'|"' == `"|"bob",,"four",2.50,,"has "quote""|"') " " (`"|`l3'|"' == `"|"c d",2.250,,1.00e+08,3,"x y"|"') " " (`"|`l4'|"' == `"|"",-3.125,"three",-0.50,,"comma, here"|"')
outfile using ofx_a4.raw, comma nolabel replace
local n 0
file open h using ofx_a4.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o4 " `n' " " (`"|`l1'|"' == `"|"ann",1.500,3,0.10,1,"plain"|"') " " (`"|`l2'|"' == `"|"bob",,4,2.50,,"has "quote""|"') " " (`"|`l3'|"' == `"|"c d",2.250,,1.00e+08,3,"x y"|"') " " (`"|`l4'|"' == `"|"",-3.125,3,-0.50,,"comma, here"|"')
outfile using ofx_a5.raw, nolabel replace
local n 0
file open h using ofx_a5.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o5 " `n' " " (`"|`l1'|"' == `"|"ann"           1.500         3       0.10         1  "plain"|"') " " (`"|`l2'|"' == `"|"bob"               .         4       2.50         .  "has "quote""|"') " " (`"|`l3'|"' == `"|"c d"           2.250         .   1.00e+08         3  "x y"|"') " " (`"|`l4'|"' == `"|""             -3.125         3      -0.50        .a  "comma, here"|"')
outfile k f using ofx_a6.raw if k == 3, replace
local n 0
file open h using ofx_a6.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o6 " `n' " " (`"|`l1'|"' == `"|"three"        0.10|"') " " (`"|`l2'|"' == `"|"three"       -0.50|"')
outfile using ofx_a7.raw in 2/3, replace wide
local n 0
file open h using ofx_a7.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o7 " `n' " " (`"|`l1'|"' == `"|"bob"               .  "four"         2.50         .  "has "quote""           |"') " " (`"|`l2'|"' == `"|"c d"           2.250         .   1.00e+08         3  "x y"                   |"')
outfile using ofx_a8.raw, nolabel replace wide noquote
local n 0
file open h using ofx_a8.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o8 " `n' " " (`"|`l1'|"' == `"|ann           1.500         3       0.10         1  plain                 |"') " " (`"|`l2'|"' == `"|bob               .         4       2.50         .  has "quote"           |"') " " (`"|`l3'|"' == `"|c d           2.250         .   1.00e+08         3  x y                   |"') " " (`"|`l4'|"' == `"|             -3.125         3      -0.50        .a  comma, here           |"')
outfile name s using ofx_a9.raw, runtogether replace
local n 0
file open h using ofx_a9.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o9 " `n' " " (`"|`l1'|"' == `"|annplain|"') " " (`"|`l2'|"' == `"|bobhas "quote"|"') " " (`"|`l3'|"' == `"|c dx y|"') " " (`"|`l4'|"' == `"|comma, here|"')
outf name using ofx_a10.raw, replace
local n 0
file open h using ofx_a10.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o10 " `n' " " (`"|`l1'|"' == `"|"ann"|"') " " (`"|`l2'|"' == `"|"bob"|"') " " (`"|`l3'|"' == `"|"c d"|"') " " (`"|`l4'|"' == `"|""|"')
outfile using ofx_a11.raw, noquote comma replace
local n 0
file open h using ofx_a11.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o11 " `n' " " (`"|`l1'|"' == `"|ann,1.500,three,0.10,1,plain|"') " " (`"|`l2'|"' == `"|bob,,four,2.50,,has "quote"|"') " " (`"|`l3'|"' == `"|c d,2.250,,1.00e+08,3,x y|"') " " (`"|`l4'|"' == `"|,-3.125,three,-0.50,,comma, here|"')
outfile s name x using ofx_a12.raw, comma wide replace
local n 0
file open h using ofx_a12.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o12 " `n' " " (`"|`l1'|"' == `"|"plain","ann",1.500|"') " " (`"|`l2'|"' == `"|"has "quote"","bob",|"') " " (`"|`l3'|"' == `"|"x y","c d",2.250|"') " " (`"|`l4'|"' == `"|"comma, here","",-3.125|"')
clear
input byte b int i long l float f double d str1 s1 str5 s5 str20 s20
1 2 3 4.5 6.25 "a" "bcd" "efghijklmnop"
-1 -200 -30000 -.125 123456.789 "" "x" ""
. . . . . "z" "yyyyy" "abcdefghijklmnopqrst"
end
format f %6.2f

outfile using ofx_b1.raw, replace
local n 0
file open h using ofx_b1.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o13 " `n' " " (`"|`l1'|"' == `"|       1         2             3    4.50        6.25  "a"|"') " " (`"|`l2'|"' == `"|"bcd"        "efghijklmnop"|"') " " (`"|`l3'|"' == `"|      -1      -200        -30000   -0.12   123456.79  ""|"') " " (`"|`l4'|"' == `"|"x"          ""|"') " " (`"|`l5'|"' == `"|       .         .             .       .           .  "z"|"') " " (`"|`l6'|"' == `"|"yyyyy"      "abcdefghijklmnopqrst"|"')
outfile using ofx_b2.raw, wide replace
local n 0
file open h using ofx_b2.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o14 " `n' " " (`"|`l1'|"' == `"|       1         2             3    4.50        6.25  "a"          "bcd"        "efghijklmnop"          |"') " " (`"|`l2'|"' == `"|      -1      -200        -30000   -0.12   123456.79  ""           "x"          ""                      |"') " " (`"|`l3'|"' == `"|       .         .             .       .           .  "z"          "yyyyy"      "abcdefghijklmnopqrst"  |"')
outfile b i using ofx_b3.raw, wide replace
local n 0
file open h using ofx_b3.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o15 " `n' " " (`"|`l1'|"' == `"|       1         2  |"') " " (`"|`l2'|"' == `"|      -1      -200  |"') " " (`"|`l3'|"' == `"|       .         .  |"')
outfile l f using ofx_b4.raw, wide replace
local n 0
file open h using ofx_b4.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o16 " `n' " " (`"|`l1'|"' == `"|           3    4.50  |"') " " (`"|`l2'|"' == `"|      -30000   -0.12  |"') " " (`"|`l3'|"' == `"|           .       .  |"')
outfile d s1 using ofx_b5.raw, wide replace
local n 0
file open h using ofx_b5.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o17 " `n' " " (`"|`l1'|"' == `"|      6.25  "a"          |"') " " (`"|`l2'|"' == `"| 123456.79  ""           |"') " " (`"|`l3'|"' == `"|         .  "z"          |"')
outfile s5 s20 using ofx_b6.raw, wide replace
local n 0
file open h using ofx_b6.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o18 " `n' " " (`"|`l1'|"' == `"|"bcd"        "efghijklmnop"          |"') " " (`"|`l2'|"' == `"|"x"          ""                      |"') " " (`"|`l3'|"' == `"|"yyyyy"      "abcdefghijklmnopqrst"  |"')
outfile s20 s20 using ofx_b7.raw, wide replace
local n 0
file open h using ofx_b7.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o19 " `n' " " (`"|`l1'|"' == `"|"efghijklmnop"          "efghijklmnop"          |"') " " (`"|`l2'|"' == `"|""                      ""                      |"') " " (`"|`l3'|"' == `"|"abcdefghijklmnopqrst"  "abcdefghijklmnopqrst"  |"')
outfile using ofx_b8.raw, nolabel noquote replace
local n 0
file open h using ofx_b8.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o20 " `n' " " (`"|`l1'|"' == `"|       1         2             3    4.50        6.25  a          bcd|"') " " (`"|`l2'|"' == `"|efghijklmnop|"') " " (`"|`l3'|"' == `"|      -1      -200        -30000   -0.12   123456.79             x|"') " " (`"|`l4'|"' == `"||"') " " (`"|`l5'|"' == `"|       .         .             .       .           .  z          yyyyy|"') " " (`"|`l6'|"' == `"|abcdefghijklmnopqrst|"')
clear
set obs 2
forvalues j = 1/12 {
    generate double v`j' = `j' + _n/10
}

outfile using ofx_c1.raw, replace
local n 0
file open h using ofx_c1.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o21 " `n' " " (`"|`l1'|"' == `"|       1.1         2.1         3.1         4.1         5.1         6.1|"') " " (`"|`l2'|"' == `"|       7.1         8.1         9.1        10.1        11.1        12.1|"') " " (`"|`l3'|"' == `"|       1.2         2.2         3.2         4.2         5.2         6.2|"') " " (`"|`l4'|"' == `"|       7.2         8.2         9.2        10.2        11.2        12.2|"')
outfile using ofx_c2.raw, wide replace
local n 0
file open h using ofx_c2.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o22 " `n' " " (`"|`l1'|"' == `"|       1.1         2.1         3.1         4.1         5.1         6.1         7.1         8.1         9.1        10.1        11.1        12.1  |"') " " (`"|`l2'|"' == `"|       1.2         2.2         3.2         4.2         5.2         6.2         7.2         8.2         9.2        10.2        11.2        12.2  |"')
clear
input double g byte lab
1.5 1
2.5 2
3.5 3
end
label define ll 1 "one" 2 "a much longer label" 3 "x"
label values lab ll

outfile lab lab using ofx_d1.raw, wide replace
local n 0
file open h using ofx_d1.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o23 " `n' " " (`"|`l1'|"' == `"|"one"                "one"                |"') " " (`"|`l2'|"' == `"|"a much longer label"  "a much longer label"  |"') " " (`"|`l3'|"' == `"|"x"                  "x"                  |"')
format g %9.0g
outfile g g using ofx_d2.raw, wide replace
local n 0
file open h using ofx_d2.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o24 " `n' " " (`"|`l1'|"' == `"|      1.5        1.5  |"') " " (`"|`l2'|"' == `"|      2.5        2.5  |"') " " (`"|`l3'|"' == `"|      3.5        3.5  |"')
format g %5.1f
outfile g g using ofx_d3.raw, wide replace
local n 0
file open h using ofx_d3.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o25 " `n' " " (`"|`l1'|"' == `"|  1.5    1.5  |"') " " (`"|`l2'|"' == `"|  2.5    2.5  |"') " " (`"|`l3'|"' == `"|  3.5    3.5  |"')
format g %10.3f
outfile g g using ofx_d4.raw, wide replace
local n 0
file open h using ofx_d4.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o26 " `n' " " (`"|`l1'|"' == `"|     1.500       1.500  |"') " " (`"|`l2'|"' == `"|     2.500       2.500  |"') " " (`"|`l3'|"' == `"|     3.500       3.500  |"')
format g %-9.0g
outfile g g using ofx_d5.raw, wide replace
local n 0
file open h using ofx_d5.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o27 " `n' " " (`"|`l1'|"' == `"|      1.5        1.5  |"') " " (`"|`l2'|"' == `"|      2.5        2.5  |"') " " (`"|`l3'|"' == `"|      3.5        3.5  |"')
format g %20.0g
outfile g g using ofx_d6.raw, wide replace
local n 0
file open h using ofx_d6.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o28 " `n' " " (`"|`l1'|"' == `"|                 1.5                   1.5  |"') " " (`"|`l2'|"' == `"|                 2.5                   2.5  |"') " " (`"|`l3'|"' == `"|                 3.5                   3.5  |"')
format g %8.0gc
outfile g g using ofx_d7.raw, wide replace
local n 0
file open h using ofx_d7.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o29 " `n' " " (`"|`l1'|"' == `"|     1.5       1.5  |"') " " (`"|`l2'|"' == `"|     2.5       2.5  |"') " " (`"|`l3'|"' == `"|     3.5       3.5  |"')
format g %12.4e
outfile g g using ofx_d8.raw, wide replace
local n 0
file open h using ofx_d8.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o30 " `n' " " (`"|`l1'|"' == `"|  1.5000e+00    1.5000e+00  |"') " " (`"|`l2'|"' == `"|  2.5000e+00    2.5000e+00  |"') " " (`"|`l3'|"' == `"|  3.5000e+00    3.5000e+00  |"')
format g %8.0f
outfile g lab using ofx_d9.raw, replace
local n 0
file open h using ofx_d9.raw, read
file read h line
while r(eof) == 0 {
    local n = `n' + 1
    local l`n' `"`line'"'
    file read h line
}
file close h
display "@ o31 " `n' " " (`"|`l1'|"' == `"|       2  "one"|"') " " (`"|`l2'|"' == `"|       2  "a much longer label"|"') " " (`"|`l3'|"' == `"|       4  "x"|"')
capture noisily outfile using ofx_a1
display "@ exists " _rc
capture noisily outfile nosuchvar using ofx_e1.raw, replace
display "@ novar " _rc
capture noisily outfile using ofx_e2.raw, bogus replace
display "@ bogus " _rc
capture noisily outfile g lab using ofx_e3.raw, runtogether replace
display "@ rt_num " _rc
capture noisily outfile g
display "@ nousing " _rc
capture confirm file ofx_e1.raw
display "@ e1_file " _rc
erase ofx_a1.raw
erase ofx_a2.raw
erase ofx_a3.raw
erase ofx_a4.raw
erase ofx_a5.raw
erase ofx_a6.raw
erase ofx_a7.raw
erase ofx_a8.raw
erase ofx_a9.raw
erase ofx_a10.raw
erase ofx_a11.raw
erase ofx_a12.raw
erase ofx_b1.raw
erase ofx_b2.raw
erase ofx_b3.raw
erase ofx_b4.raw
erase ofx_b5.raw
erase ofx_b6.raw
erase ofx_b7.raw
erase ofx_b8.raw
erase ofx_c1.raw
erase ofx_c2.raw
erase ofx_d1.raw
erase ofx_d2.raw
erase ofx_d3.raw
erase ofx_d4.raw
erase ofx_d5.raw
erase ofx_d6.raw
erase ofx_d7.raw
erase ofx_d8.raw
erase ofx_d9.raw
