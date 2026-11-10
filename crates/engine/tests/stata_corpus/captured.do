* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in captured.recorded
* commands: drop
* functions: string strofreal
* (capture drop of names and patterns the data does not have: _rc 111, r() kept; string() with and without a format, of a number and of a variable)
* tolerance: 0
version 19
clear
set obs 10
generate x = _n
generate y = 1
generate Zxa = 1
summarize x, detail
capture drop nosuch
display "@ a " _rc " " r(p50) " " r(N)
capture drop Z*b
display "@ b " _rc " " r(p50)
capture keep nosuch2
display "@ c " _rc " " r(N) " " _N
capture drop Z*a
display "@ d " _rc
capture confirm variable Zxa
display "@ e " _rc
display "@ f [" string(1.646309596749012, "%4.1f") "][" string(5, "%9.2f") "][" string(1234567) "][" string(.) "]"
display "@ g [" string(1/3) "][" string(-3.2, "%8.3f") "][" string(1234567.891, "%12.2fc") "][" string(0.000012345) "]"
display "@ h [" string(123456789012) "][" string(3, "%-8.2f") "][" strofreal(2.5, "%6.2f") "][" string(1e20) "]"
replace x = . in 3
generate s = string(x / 3)
generate t = string(x, "%6.2f")
display "@ i [" s[1] "][" s[3] "][" t[2] "][" t[3] "]"
