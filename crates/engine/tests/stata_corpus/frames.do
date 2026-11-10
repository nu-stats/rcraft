* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in frames.recorded
* commands: frame
* (frame create, change and drop: each frame keeps its own data, e() stays; _rc 110, 111 and 119 under capture; frame pwf; preserve is each frame's own)
* tolerance: 1e-8
version 19
clear
set obs 6
generate x = _n * 2
regress x
capture frame drop nosuch
display "@ a " _rc
frame create fa
capture frame create fa
display "@ b " _rc
capture frame change nosuch
display "@ c " _rc
capture frame drop default
display "@ d " _rc
frame change fa
clear
set obs 3
generate y = _n + 10
summarize y
display "@ e " r(mean) " " r(N) " " e(N)
frame change default
summarize x
display "@ f " r(mean) " " r(N) " " e(N)
frame change fa
summarize y
display "@ g " r(sum)
capture frame drop fa
display "@ h " _rc
frame change default
capture frame drop fa
display "@ i " _rc
capture frame change fa
display "@ j " _rc
capture set obs 2
display "@ k " _rc
frame pwf
preserve
drop in 1/2
frame create pb
frame change pb
set obs 2
generate z = 1
preserve
drop in 1
frame change default
display "@ l " _N
restore
display "@ m " _N
frame change pb
display "@ n " _N
restore
display "@ o " _N
