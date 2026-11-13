* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in centile.recorded
* commands: centile
* (centile with Stata's interpolated binomial interval: centile(), level(), bounds held at the sample's ends, small samples, missing values, if; r() of the last variable)
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 31
set obs 50
generate x = round(100*runiform(), .1)
generate double y = rnormal()
generate t = _n
replace y = . in 1/3
centile x
display "@ med " r(N) " " r(n_cent) " " %24.17g r(c_1) " " %24.17g r(lb_1) " " %24.17g r(ub_1)
centile x y, centile(25 50 75)
display "@ q1 " r(N) " " %24.17g r(c_1) " " %24.17g r(lb_1) " " %24.17g r(ub_1)
display "@ q3 " %24.17g r(c_3) " " %24.17g r(lb_3) " " %24.17g r(ub_3)
centile x, level(90)
display "@ l90 " %24.17g r(lb_1) " " %24.17g r(ub_1)
centile x, centile(1 99)
display "@ ends " %24.17g r(c_1) " " %24.17g r(lb_1) " " %24.17g r(ub_1) " " %24.17g r(c_2) " " %24.17g r(lb_2) " " %24.17g r(ub_2)
centile t if t <= 10, centile(10 50 90)
display "@ ten " %24.17g r(c_1) " " %24.17g r(ub_1) " " %24.17g r(lb_2) " " %24.17g r(ub_2) " " %24.17g r(lb_3) " " %24.17g r(c_3)
centile t if t <= 2, centile(10 50 90)
display "@ two " %24.17g r(c_1) " " %24.17g r(ub_1) " " %24.17g r(lb_3) " " %24.17g r(c_2)
centile t if t <= 4
display "@ four " r(N) " " %24.17g r(c_1) " " r(lb_1) " " r(ub_1)
quietly centile y, centile(33.3)
display "@ quiet " r(N) " " %24.17g r(c_1) " " %24.17g r(lb_1) " " %24.17g r(ub_1)
centile x, centile(5 50 95) cci
display "@ cci " %24.17g r(lb_1) " " %24.17g r(ub_1) " " %24.17g r(lb_2) " " %24.17g r(ub_2) " " %24.17g r(lb_3) " " %24.17g r(ub_3)
centile x y, centile(10 50) normal
display "@ normal " %24.17g r(c_1) " " %24.17g r(lb_1) " " %24.17g r(ub_1) " " %24.17g r(lb_2) " " %24.17g r(ub_2)
centile x, centile(25 50) meansd level(90)
display "@ meansd " %24.17g r(c_1) " " %24.17g r(lb_1) " " %24.17g r(ub_1) " " %24.17g r(c_2) " " %24.17g r(ub_2)
centile t if t <= 10, centile(10 50 90) cci
display "@ cci10 " %24.17g r(lb_1) " " %24.17g r(ub_1) " " %24.17g r(lb_2) " " %24.17g r(ub_2) " " %24.17g r(lb_3) " " %24.17g r(ub_3)
