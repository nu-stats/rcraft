* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in center.recorded
* commands: center
* (center from SSC, as a replication package standardizes its moderators in place: new c_ variables and prefix(), inplace on a double, a float and a byte, standardize, casewise, if (with inplace too), aweights and fweights, double, replace, nolabel, and the add-on's errors)
* tolerance: 1e-13
version 19
set linesize 255
clear
set seed 31
set obs 40
generate double x = rnormal()
generate y = runiform()
generate byte b = mod(_n, 4)
generate w = 1 + mod(_n, 3)
replace x = . in 2
replace y = . in 5
label variable x "the x"
preserve
center x y
display "@ plain " %20.0g c_x[1] " " %20.0g c_x[6] " " %20.0g c_y[3] " " missing(c_x[2]) " " missing(c_x[5])
display "@ plain_lab " ("`: variable label c_x'" == "the x (centered)") " " ("`: variable label c_y'" == "y (centered)")
restore
preserve
center x y, casewise
display "@ case " %20.0g c_x[1] " " missing(c_x[5]) " " %20.0g c_y[1]
restore
preserve
center x y b, inplace
display "@ inplace " %20.0g x[1] " " %20.0g x[6] " " %20.0g y[3] " " %20.0g b[1] " " %20.0g b[3]
restore
preserve
center x, standardize
display "@ std " %20.0g c_x[1] " " %20.0g c_x[9] " " ("`: variable label c_x'" == "the x (standardized)")
restore
preserve
center x y, inplace standardize
display "@ inpstd " %20.0g x[1] " " %20.0g y[1]
restore
preserve
center x y, prefix(z_) double
display "@ prefix " %20.0g z_x[1] " " %20.0g z_y[4]
restore
preserve
center x if b > 0, s
display "@ if " %20.0g c_x[1] " " missing(c_x[4]) " " %20.0g c_x[7]
restore
preserve
center x y if b > 0, inplace
display "@ ifinplace " %20.0g x[1] " " %20.0g x[4] " " %20.0g y[4] " " %20.0g y[1]
restore
preserve
center x y, casewise inplace
display "@ caseinplace " %20.0g x[5] " " %20.0g x[1] " " missing(y[5])
restore
preserve
center x [aw = w]
display "@ aw " %20.0g c_x[1] " " %20.0g c_x[7]
restore
preserve
center x [fw = w], standardize
display "@ fw " %20.0g c_x[1] " " %20.0g c_x[7]
restore
preserve
generate c_x = 1
capture center x
display "@ exists " _rc
center x, replace nolabel
display "@ replace " %20.0g c_x[1] " " ("`: variable label c_x'" == "")
restore
generate str3 s = "a"
capture center s
display "@ string " _rc
capture center x, inplace prefix(k_)
display "@ inpre " _rc
