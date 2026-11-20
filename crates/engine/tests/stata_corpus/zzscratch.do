version 19
set linesize 255
clear
set obs 120
set seed 297
generate g = 1 + floor(3 * runiform())
generate h = runiform() > 0.5
label define yesno 0 "no" 1 "yes"
label values h yesno
generate x = round(invnormal(runiform()), .01)
generate y = round(1 + 0.5 * x + 0.3 * (g == 2) + 0.2 * h + invnormal(runiform()), .01)
generate b = runiform() < .5
logistic b x
logistic b x, coef
xi: regress y i.g x
describe _I*
list g _Ig_2 _Ig_3 in 1/5
display "@ done 1"
