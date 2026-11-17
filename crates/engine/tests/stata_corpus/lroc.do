* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in lroc.recorded
* commands: lroc
* (the area under the ROC curve after logit and probit: e(sample), if and in, all, beta(), factor variables, frequency weights, a sample of one outcome; Stata's errors)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 60
generate double x = mod(_n * 7, 13) / 4 - 1.5
generate byte g = mod(_n, 3)
generate byte w = 1 + mod(_n, 4)
generate byte y = mod(_n * 11 + 3 * g, 7) < 2 + (x > 0) + g
replace x = . in 5
generate float half = 1.5
matrix b = (1, 0.5, -0.2)
matrix colnames b = x g _cons
matrix c = (0.8, -0.1)
matrix colnames c = x _cons
quietly logit y x g
quietly lroc, nograph
display "@ logit " %24.17g r(area) " " r(N)
quietly lroc y, nograph
display "@ depvar " %24.17g r(area) " " r(N)
quietly lroc if g == 1, nograph
display "@ if " %24.17g r(area) " " r(N)
quietly lroc in 1/30, nograph
display "@ in " %24.17g r(area) " " r(N)
quietly lroc if y == 1, nograph
display "@ one outcome " %24.17g r(area) " " r(N)
quietly lroc y, nograph beta(b)
display "@ beta " %24.17g r(area) " " r(N)
quietly lroc y, nograph beta(c)
display "@ beta two " %24.17g r(area) " " r(N)
quietly lroc [fw=w], nograph
display "@ lroc weights " %24.17g r(area) " " r(N)
quietly logit y x g if _n > 12
quietly lroc, nograph
display "@ sample " %24.17g r(area) " " r(N)
quietly lroc, nograph all
display "@ all " %24.17g r(area) " " r(N)
quietly lroc in 1/15, nograph
display "@ in past the sample " %24.17g r(area) " " r(N)
quietly probit y x g
quietly lroc, nograph
display "@ probit " %24.17g r(area) " " r(N)
quietly logit y i.g x
quietly lroc
display "@ factor " %24.17g r(area) " " r(N)
quietly logit y g
quietly lroc, nograph
display "@ ties " %24.17g r(area) " " r(N)
quietly logit y x g [fw=w]
quietly lroc, nograph
display "@ fit weights " %24.17g r(area) " " r(N)
quietly lroc if g == 2, nograph
display "@ fit weights if " %24.17g r(area) " " r(N)
quietly logit y x g [pw=w]
capture lroc, nograph
display "@ rc fit pweights " _rc
quietly logit y x g
capture lroc [pw=w], nograph
display "@ rc pweights " _rc
capture lroc [aw=w], nograph
display "@ rc aweights " _rc
capture lroc [fw=half], nograph
display "@ rc noninteger " _rc
capture lroc g, nograph
display "@ rc varlist " _rc
capture lroc y x, nograph
display "@ rc too many " _rc
capture lroc, nograph beta(b)
display "@ rc beta depvar " _rc
capture lroc y, nograph beta(nosuch)
display "@ rc beta matrix " _rc
capture lroc if g == 1, nograph all
display "@ rc all if " _rc
capture lroc if g == 7, nograph
display "@ rc no observations " _rc
quietly regress y x g
capture lroc, nograph
display "@ rc regress " _rc
quietly lroc y, nograph beta(b)
display "@ beta after regress " %24.17g r(area) " " r(N)
