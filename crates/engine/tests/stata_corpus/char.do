* RCraft's Stata corpus (P2C.12y, batch A2): made-up data; Stata 19's results are in char.recorded
* commands: char
* (char: setting, replacing and removing characteristics of a variable and of _dta, reading them with the macro function and the macro `var[name]', the text trimmed, commas and if/in words in the text, names that differ in case, an abbreviated variable name, the errors with their return codes, and the characteristics kept in a .dta file)
version 19
set linesize 255
clear
set obs 3
generate x = _n
generate y = 2 * _n
generate abcdef = 3
char x[a] hello
char x[b] "two words"
char _dta[d1] dd
char y[note1] zz
display "@ get " ("`: char x[a]'" == "hello") " " ("`: char x[b]'" == "two words") " " ("`: char _dta[d1]'" == "dd") " " ("`: char y[note1]'" == "zz") " " ("`: char y[nope]'" == "")
display "@ macro " ("`x[a]'" == "hello") " " ("`_dta[d1]'" == "dd") " " ("`y[nope]'" == "")
char x[a]
display "@ removed " ("`: char x[a]'" == "") " " ("`x[a]'" == "") " " ("`: char x[b]'" == "two words")
char x[b]  leading   spaces   inside
display "@ trim " length("`: char x[b]'") " " ("`: char x[b]'" == "leading   spaces   inside")
char x[c] a, b
char x[d] a if x
char x[e] a in 3
display "@ words " ("`x[c]'" == "a, b") " " ("`x[d]'" == "a if x") " " ("`x[e]'" == "a in 3")
char x[Abc] up
char x[abc] low
display "@ case " ("`x[Abc]'" == "up") " " ("`x[abc]'" == "low")
char x[a] 7
display "@ number " `x[a]' + 1
char define x[f] defined
display "@ define " ("`x[f]'" == "defined")
char abc[k] viaabbrev
display "@ abbrev " ("`: char abcdef[k]'" == "viaabbrev")
char _dta[d1] replaced
display "@ replaced " ("`_dta[d1]'" == "replaced")
quietly char list
rename x z
display "@ rename " ("`z[b]'" == "leading   spaces   inside") " " ("`: char z[Abc]'" == "up")
capture char nosuch[a] 1
display "@ rc_nosuch " _rc
capture char x[a b] 1
display "@ rc_badname " _rc
capture char z[] 1
display "@ rc_empty " _rc
capture char list z
display "@ rc_list_plain " _rc
capture char list nosuch[]
display "@ rc_list_nosuch " _rc
generate xyz = 1
generate xyw = 2
capture char xy[a] 1
display "@ rc_ambiguous " _rc
drop z
display "@ dropped " ("`z[b]'" == "") " " ("`_dta[d1]'" == "replaced")
keep y abcdef xyz xyw
display "@ kept " ("`_dta[d1]'" == "replaced") " " ("`y[note1]'" == "zz")
sort y
display "@ sorted " ("`y[note1]'" == "zz")
preserve
char y[temp] only here
restore
display "@ restored " ("`y[temp]'" == "") " " ("`y[note1]'" == "zz")
save char_rt, replace
clear
use char_rt, clear
display "@ reread " ("`_dta[d1]'" == "replaced") " " ("`y[note1]'" == "zz") " " ("`abcdef[k]'" == "viaabbrev")
char y[note1]
save char_rt, replace
use char_rt, clear
display "@ reread2 " ("`y[note1]'" == "") " " ("`_dta[d1]'" == "replaced")
erase char_rt.dta
