* RCraft's Stata corpus (P2C.12y, batch A2): made-up data; Stata 19's results are in compare.recorded
* commands: compare
* (compare: r(N), numbers and strings, missing values of both kinds, if and in, and the errors with their return codes; the table itself was read against Stata 19's by hand, since only numbers are recorded: the counts below are the ones its rows hold)
version 19
set linesize 255
clear
set obs 40
generate double x = mod(_n * 37, 101) / 7
generate double y = 2 + .5 * x + mod(_n * 13, 17) / 3
generate double z = mod(_n * 11, 23) / 4 - .2 * x
generate int g = 1 + mod(_n, 3)
generate int k = 1 + mod(_n * 7, 5)
generate str5 s = "a" + string(mod(_n, 4))
generate str3 s2 = "b"
replace z = . in 3
replace k = . in 5/6
replace g = .a in 9
replace y = x in 1/10
compare x y
display "@ N_xy " r(N)
quietly count if x < y
display "@ lt " r(N)
quietly count if x == y
display "@ eq " r(N)
quietly count if x > y
display "@ gt " r(N)
compare x z
display "@ N_xz " r(N)
compare g k
display "@ N_gk " r(N)
compare s s2
display "@ N_ss2 " r(N)
compare x y if g == 1
display "@ N_if " r(N)
compare x y in 5/20
display "@ N_in " r(N)
compare x y if x > 1000
display "@ N_none " r(N)
compare x x
display "@ N_xx " r(N)
quietly compare x y
display "@ N_quiet " r(N)
capture compare x
display "@ rc_one " _rc
capture compare x y z
display "@ rc_three " _rc
capture compare x y [fw = k]
display "@ rc_weight " _rc
capture compare x nosuch
display "@ rc_nosuch " _rc
