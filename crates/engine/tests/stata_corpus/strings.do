* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in strings.recorded
* commands: confirm file split
* functions: char float invlogit length logit ltrim rtrim strltrim strmatch strrtrim strtrim subinstr trim ustrltrim ustrrtrim ustrtrim
* (the trims, which blanks each takes (spaces only, or tabs and new lines too for the ustr ones), strmatch()'s * and ? with every other character as itself, char() of ASCII codes, missing and past 255, and float(); split: words, parse() strings with the empty pieces they leave, notrim, limit(), if, and its r(); file open, write and close, read back with insheet; length() in bytes and subinstr() on bytes, with char(128) to char(255) searched for and put in; confirm and the return codes it leaves)
* tolerance: 0
version 19
set linesize 255
clear
set obs 4
generate s = char(9) + " x " + char(9)
replace s = char(10) + " x " + char(13) in 2
replace s = "  x  " in 3
replace s = "[Statement - - see Appendix.]" in 4
generate a = length(strtrim(s))
generate b = length(strltrim(s))
generate c = length(strrtrim(s))
generate d = length(trim(s)) + 100 * length(ltrim(s)) + 10000 * length(rtrim(s))
generate e = length(ustrtrim(s)) + 100 * length(ustrltrim(s)) + 10000 * length(ustrrtrim(s))
forvalues i = 1/4 {
  display "@ trims `i' " a[`i'] " " b[`i'] " " c[`i'] " " d[`i'] " " e[`i']
}
generate m1 = strmatch(s, "[Statement - - see Appendix.]")
generate m2 = strmatch(s, "*x*")
generate m3 = strmatch(s, "?*")
generate m4 = strmatch(s, "*[*")
forvalues i = 1/4 {
  display "@ strmatch `i' " m1[`i'] " " m2[`i'] " " m3[`i'] " " m4[`i']
}
display "@ patterns " strmatch("abc", "a*") " " strmatch("abc", "A*") " " strmatch("abc", "a?c") " " strmatch("abc", "a?") " " strmatch("", "*") " " strmatch("", "") " " strmatch("abc", "a.c") " " strmatch("a*c", "a\*c") " " strmatch("a\c", "a\c") " " strmatch("abc", "[a]bc") " " strmatch("a(b)c", "a(b)c") " " strmatch("ab" + char(10) + "c", "ab*c") " " strmatch("x", "**")
display "@ char " length(char(65)) " " (char(65.7) == "A") " " length(char(256)) " " length(char(.)) " " length(char(-3)) " " (char(97) + char(98) == "ab")
display "@ float " (float(0.1) == float(0.1)) " " (float(0.1) == 0.1) " " float(0.5) " " (float(16777217) == 16777216)
clear
set obs 6
generate full = "MR  JOHN SMITH" in 1
replace full = " lead and trail " in 2
replace full = "a . b" in 3
replace full = ".a" in 4
replace full = "a.b..c." in 5
replace full = "x inflation y inflation z" in 6
preserve
split full
display "@ split words " r(k_new) " " ("`r(varlist)'" == "full1 full2 full3 full4 full5 ")
forvalues i = 1/6 {
  display "@ split words `i' " length(full1[`i']) " " length(full2[`i']) " " length(full3[`i']) " " length(full4[`i']) " " (full2[`i'] == "JOHN")
}
restore, preserve
split full, p(" ") generate(part) limit(2)
display "@ split space " r(k_new) " " (part1[1] == "MR") " " (part2[1] == "") " " (part2[2] == "and")
restore, preserve
split full, parse(".") generate(s)
display "@ split dot " r(k_new) " " (s1[3] == "a ") " " (s2[3] == " b") " " (s1[4] == "") " " (s2[4] == "a") " " (s3[5] == "") " " (s4[5] == "c") " " (s1[2] == "lead and trail")
restore, preserve
split full, parse(".") notrim generate(t)
display "@ split notrim " (t1[2] == " lead and trail ") " " (t2[3] == " b")
restore, preserve
split full, parse(" inflation ") generate(inf)
display "@ split string " r(k_new) " " (inf1[6] == "x") " " (inf2[6] == "y") " " (inf3[6] == "z")
restore, preserve
split full, parse(" " ".") generate(m)
display "@ split two " r(k_new) " " (m2[1] == "") " " (m3[1] == "JOHN") " " (m4[3] == "b") " " (m2[5] == "b")
restore, preserve
split full if _n > 3, generate(c)
display "@ split if " r(k_new) " " (c1[1] == "") " " (c1[6] == "x")
restore
generate full1 = "taken"
capture split full
display "@ split taken " _rc
tempname fh
file open `fh' using written_by_file.txt, write replace
file write `fh' "a" _tab "b" _tab "c" _n
file write `fh' (1/3) _tab %5.2f (2.5) _tab "x y" _n
file write `fh' (12345678) _tab (123456789) _tab `"q"' _newline
file close `fh'
capture file open `fh' using written_by_file.txt, write
display "@ file exists " _rc
capture file write `fh' "x"
display "@ file closed " _rc
file open `fh' using written_by_file.txt, write append
file write `fh' (-0.5) _tab (1e-10) _tab "z" _n
file close `fh'
insheet using written_by_file.txt, clear double
display "@ file read back " _N " " %21x a[1] " " b[1] " " (c[1] == "x y") " " a[2] " " b[2] " " (c[2] == "q") " " a[3] " " b[3]
erase written_by_file.txt
clear
set obs 2
generate str20 s = "café" in 1
replace s = "plain text" in 2
generate t = subinstr(s, char(169), "X", .)
generate u = subinstr(s, char(195), char(200), 1)
generate v = subinstr(s, "a", char(65) + char(66), .)
display "@ bytes " length(s[1]) " " length(t[1]) " " length(u[1]) " " length(v[1]) " " (t[2] == "plain text") " " length(v[2]) " " (substr(t[1], 1, 3) == "caf")
capture confirm variable s
local r1 = _rc
capture confirm variable nosuch
local r2 = _rc
capture confirm numeric variable s
local r3 = _rc
capture confirm string variable s
local r4 = _rc
capture confirm new variable s
local r5 = _rc
capture confirm new variable brandnew
local r6 = _rc
capture confirm number abc
local r7 = _rc
capture confirm integer number 3.5
local r8 = _rc
capture confirm number 3.5
local r9 = _rc
capture confirm file nosuchfile_590.txt
local r10 = _rc
capture confirm variable pl
local r11 = _rc
display "@ confirm `r1' `r2' `r3' `r4' `r5' `r6' `r7' `r8' `r9' `r10' `r11'"
display "@ invlogit " %21x invlogit(0.3) " " %21x invlogit(-2) " " %21x logit(0.25) " " missing(logit(1)) " " missing(logit(0))
display "@ repeat " (3*"ab" == "ababab") " " length(char(65)*4) " " (0*"x" == "")
