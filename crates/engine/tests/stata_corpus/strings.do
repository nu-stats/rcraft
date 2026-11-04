* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in strings.recorded
* functions: char float ltrim rtrim strltrim strmatch strrtrim strtrim trim ustrltrim ustrrtrim ustrtrim
* (the trims, which blanks each takes (spaces only, or tabs and new lines too for the ustr ones), strmatch()'s * and ? with every other character as itself, char() of ASCII codes, missing and past 255, and float())
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
