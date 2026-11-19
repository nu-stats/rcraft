* RCraft's Stata corpus (P2C.12d continued): made-up data; Stata 19's results are in substr.recorded
* functions: bsubstr substr udsubstr usubstr
* (substr() by bytes and usubstr() by characters with every start and length: from the end, 0, missing, beyond the string, non-whole, and on columns; bsubstr() and udsubstr() as in functions_f)
* tolerance: 1e-8
version 19
clear
set linesize 255
display "@ substr1 [" substr("abcdef", 2, 3) "] [" substr("abcdef", 0, 3) "] [" substr("abcdef", -2, 2) "] [" substr("abcdef", -2, .) "] [" substr("abcdef", 2, .) "] [" substr("abcdef", 2, 0) "] [" substr("abcdef", 2, -1) "] [" substr("abcdef", 7, 1) "] [" substr("abcdef", 6, 5) "] [" substr("abcdef", -10, 3) "] [" substr("abcdef", -10, 8) "] [" substr("", 1, 1) "]"
display "@ substr2 [" substr("abcdef", ., 2) "] [" substr("abcdef", 2, .a) "] [" substr("abcdef", 1.9, 2.9) "] [" substr("abcdef", -1.5, 1) "] [" substr("abcdef", -6, 1) "] [" substr("abcdef", -7, 2) "] [" substr("abcdef", 1, 100) "] [" substr("abcdef", 1e10, 2) "] [" substr("abcdef", 3, 1e10) "] [" substr("abcdef", -3, 1e10) "]"
display "@ substr3 " length(substr("éa", 1, 1)) " " length(substr("éa", 2, 2)) " " length(substr("éa", 1, 2)) " " (substr("éa", 1, 2) == "é") " " (substr("éa", 3, 1) == "a") " " length(substr("éaü", -2, 2)) " " length(substr("éaü", -1, .))
display "@ usubstr1 [" usubstr("abcdef", 2, 3) "] [" usubstr("abcdef", 0, 3) "] [" usubstr("abcdef", -2, 2) "] [" usubstr("abcdef", -2, .) "] [" usubstr("abcdef", 2, .) "] [" usubstr("abcdef", 2, 0) "] [" usubstr("abcdef", 2, -1) "] [" usubstr("abcdef", 7, 1) "] [" usubstr("abcdef", 6, 5) "] [" usubstr("abcdef", -10, 3) "] [" usubstr("abcdef", -10, 8) "] [" usubstr("", 1, 1) "] [" usubstr("abcdef", ., 2) "] [" usubstr("abcdef", 1.9, 2.9) "] [" usubstr("abcdef", -1.5, 1) "]"
display "@ usubstr2 [" usubstr("éaü", 2, 2) "] [" usubstr("éaü", -1, 1) "] [" usubstr("éaü", 0, 1) "] [" usubstr("éaü", 2, .) "] [" usubstr("éaü", 1, 5) "] [" usubstr("éaü", -3, 2) "] [" usubstr("éaü", -5, 2) "] [" usubstr("日本語", 2, 1) "] [" usubstr("日本語", -1, 1) "] " ustrlen(usubstr("éa", 1, 1))
set obs 6
generate str12 s = "abcdefghij" in 1
replace s = "an old rug" in 2
replace s = "" in 3
replace s = "x" in 4
replace s = "2024-05-17" in 5
replace s = "name.ext" in 6
generate n1 = _n - 3
generate n2 = 4 - _n
replace n2 = . in 5
generate str12 a = substr(s, n1, n2)
generate str12 b = substr(s, -3, .)
generate str12 c = substr(s, 6, 2)
generate str12 t = "é and ü" in 2
replace t = "日本語abc" in 1
generate str12 d = usubstr(t, n1, n2)
generate str12 e = usubstr(t, -3, .)
generate str12 f = substr(s, strpos(s, ".") + 1, .) if strpos(s, ".") > 0
forvalues i = 1/6 {
  display "@ substr_rows `i' [" subinstr(a[`i'], " ", "_", .) "] [" subinstr(b[`i'], " ", "_", .) "] [" subinstr(c[`i'], " ", "_", .) "] [" subinstr(d[`i'], " ", "_", .) "] [" subinstr(e[`i'], " ", "_", .) "] [" f[`i'] "]"
}
generate year = real(substr(s, 1, 4)) if substr(s, 5, 1) == "-"
generate month = real(substr(s, 6, 2)) if substr(s, 5, 1) == "-"
forvalues i = 1/6 {
  display "@ substr_nums `i' " year[`i'] " " month[`i']
}
