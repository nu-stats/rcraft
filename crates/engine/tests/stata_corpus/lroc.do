* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in lroc.recorded
* commands: lroc
* (the area under the ROC curve after logit and probit: e(sample), if and in, all, beta() with factor levels and equations, frequency and importance weights, a sample of one outcome, graph options; Stata's errors)
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
generate byte yvar = y
generate byte nw = w
replace nw = -1 in 3
generate byte w0 = w
replace w0 = 0 in 2
generate str1 s = "a"
matrix k = (-0.2)
matrix colnames k = _cons
matrix nc = (1, 0.5)
matrix colnames nc = x g
matrix m = (1, 0.5, .)
matrix colnames m = x g _cons
matrix d1 = (1, 0.5)
matrix colnames d1 = x nosuch
matrix t = b'
quietly logit y i.g x
matrix fb = e(b)
generate double g2 = 2 * g
quietly logit y x g g2
matrix ob = e(b)
quietly logit y x g
quietly lroc y, nograph beta(fb)
display "@ beta factor " %24.17g r(area) " " r(N)
quietly lroc y, nograph beta(ob)
display "@ beta omitted " %24.17g r(area) " " r(N)
quietly lroc y, nograph beta(k)
display "@ beta constant " %24.17g r(area) " " r(N)
quietly lroc y, nograph beta(nc)
display "@ beta no constant " %24.17g r(area) " " r(N)
capture lroc y, nograph beta(m)
display "@ rc beta missing " _rc
capture lroc y, nograph beta(d1)
display "@ rc beta variable " _rc
capture lroc y, nograph beta(t)
display "@ rc beta column " _rc
capture lroc s, nograph beta(b)
display "@ rc beta string " _rc
capture lroc y x, nograph beta(b)
display "@ rc beta too many " _rc
quietly logit yvar x g
quietly lroc yv, nograph
display "@ abbreviated " %24.17g r(area) " " r(N)
matrix eb = e(b)
quietly lroc yvar, nograph beta(eb)
display "@ beta e(b) " %24.17g r(area) " " r(N)
quietly lroc [w=w], nograph
display "@ weight assumed " %24.17g r(area) " " r(N)
capture lroc [fw=nw], nograph
display "@ rc negative " _rc
capture by g: lroc, nograph
display "@ rc by unsorted " _rc
sort g
quietly logit yvar x g
capture by g: lroc, nograph
display "@ rc by " _rc
capture lroc, nograph title(hi)
display "@ rc nograph title " _rc
quietly lroc, title(hi) legend(off)
display "@ title " %24.17g r(area) " " r(N)
quietly logit y x g [iw=w]
quietly lroc, nograph
display "@ iweights " %24.17g r(area) " " r(N)
quietly lroc y, nograph beta(b)
display "@ beta after iweights " %24.17g r(area) " " r(N)
quietly logit y x g [iw=w0]
quietly lroc, nograph
display "@ iweights zero " %24.17g r(area) " " r(N)
quietly logit y x g [fw=w]
quietly lroc y, nograph beta(b)
display "@ beta after fweights " %24.17g r(area) " " r(N)
quietly logit y x g [pw=w]
quietly lroc [fw=w], nograph
display "@ own weights after pweights " %24.17g r(area) " " r(N)
quietly mlogit g x
capture lroc, nograph
display "@ rc mlogit " _rc
quietly poisson w x
capture lroc, nograph
display "@ rc poisson " _rc
