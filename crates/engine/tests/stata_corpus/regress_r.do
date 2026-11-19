* RCraft's Stata corpus (P2C.12x): made-up data; Stata 19's results are in regress_r.recorded
* commands: regress summarize
* (regress clears r() and leaves only r(table), quietly or not: r(N), r(mean) and any other name are missing after it)
version 19
set linesize 255
clear
input double y double x
1 2
3 1
2 4
5 3
4 6
6 5
end
summarize y
display "@ sum_before " r(N) " " r(mean)
regress y x
display "@ after_shown " missing(r(N)) " " missing(r(mean)) " " missing(r(r2))
summarize x
display "@ sum_again " r(N) " " r(mean)
quietly regress y x
display "@ after_quiet " missing(r(N)) " " missing(r(mean)) " " e(N)
local n = r(N)
display "@ local_n " missing(`n')
