* RCraft's Stata corpus (P2C.12y, batch A2): made-up data; Stata 19's results are in labelbook.recorded
* commands: labelbook
* (labelbook: r(names) and the lists of label sets with a problem (gaps, not used, numeric labels, blanks, duplicate labels in full and shortened), the options problems and length(), sets given by name, and the error for a set that is not there; the printed pages were read against Stata 19's by hand)
version 19
set linesize 255
clear
set obs 40
generate double x = mod(_n * 37, 101) / 7
generate int g = 1 + mod(_n, 3)
generate int k = 1 + mod(_n * 7, 5)
label define gl 1 "low" 2 "mid" 3 "high"
label values g gl
label define kl 1 "one" 2 "two" 5 "five", modify
label values k kl
label define unused 0 "zero" 9 "nine"
labelbook
display "@ all_names " ("`r(names)'" == "gl kl unused") " " ("`r(gaps)'" == "kl unused") " " ("`r(notused)'" == "unused") " " ("`r(numeric)'" == "") " " ("`r(blanks)'" == "") " " ("`r(nuniq)'" == "") " " ("`r(nuniq_sh)'" == "")
labelbook gl
display "@ one " ("`r(names)'" == "gl") " " ("`r(gaps)'" == "") " " ("`r(notused)'" == "")
labelbook kl unused
display "@ two " ("`r(names)'" == "kl unused") " " ("`r(gaps)'" == "kl unused") " " ("`r(notused)'" == "unused")
labelbook, problems
display "@ problems " ("`r(names)'" == "gl kl unused") " " ("`r(gaps)'" == "kl unused") " " ("`r(notused)'" == "unused")
label define odd 1 "a" 2 " b" 4 "123" 7 "a" -3 "neg"
label define dup 1 "a very long label string that runs" 2 "a very long label string that runs past"
labelbook odd dup
display "@ odd " ("`r(names)'" == "dup odd") " " ("`r(numeric)'" == "odd") " " ("`r(blanks)'" == "odd") " " ("`r(nuniq)'" == "odd") " " ("`r(nuniq_sh)'" == "dup odd") " " ("`r(gaps)'" == "odd") " " ("`r(notused)'" == "dup odd")
labelbook, problems
display "@ odd_problems " ("`r(names)'" == "dup gl kl odd unused") " " ("`r(numeric)'" == "odd") " " ("`r(nuniq_sh)'" == "dup odd") " " ("`r(gaps)'" == "kl odd unused") " " ("`r(notused)'" == "dup odd unused")
labelbook odd, length(5)
display "@ len5 " ("`r(nuniq_sh)'" == "odd") " " ("`r(nuniq)'" == "odd")
labelbook odd dup, length(40)
display "@ len40 " ("`r(nuniq_sh)'" == "odd") " " ("`r(nuniq)'" == "odd")
label define shared 1 "x" 2 "y"
generate byte h = 1
generate byte h2 = 2
label values h shared
label values h2 shared
labelbook shared
display "@ shared " ("`r(notused)'" == "") " " ("`r(names)'" == "shared")
labelbook gl shared, problems
display "@ gl_shared " ("`r(names)'" == "gl shared") " " ("`r(gaps)'" == "") " " ("`r(notused)'" == "")
capture labelbook nosuch
display "@ rc_nosuch " _rc
capture labelbook gl nosuch
display "@ rc_second " _rc
quietly labelbook odd
display "@ quiet " ("`r(names)'" == "odd")
