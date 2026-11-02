* RCraft's Stata corpus (P2C.11 continued): made-up data; Stata 19's results are in variates.recorded
* functions: rbinomial rcauchy rchi2 rexponential rgeometric rhypergeometric rigaussian rlaplace rlogistic rnbinomial rpareto runiformint rweibull rweibullph
* (the other random variates with Stata's own draws: each draws a column, then its sum, one value and the uniform after it are checked; arguments that give missing. macOS's log, sin and sqrt differ from Windows' and Linux's in the last bit for some arguments, so 1e-12 and not 0)
* tolerance: 1e-12
version 19
clear
set obs 400
set seed 20261102
generate double v1 = rexponential(2.5)
generate double v2 = rlogistic(3)
generate double v3 = rlogistic(1, 2)
generate double v4 = rweibull(1.5, 2)
generate double v5 = rweibullph(2, 0.5, -1)
generate double v6 = rcauchy(1, 2)
generate double v7 = rlaplace(-1, 3)
generate double v8 = rpareto(2, 3)
generate double v9 = rgeometric(0.3)
generate double v10 = rigaussian(2, 3)
generate double v11 = rchi2(3)
generate double v12 = runiformint(-5, 1000)
generate double v13 = runiformint(7, 7)
generate double v14 = rbinomial(50, 0.7)
generate double v15 = rnbinomial(3, 0.5)
generate double v16 = rhypergeometric(50, 10, 20)
generate double v17 = rexponential(-1)
generate double v18 = rbinomial(10, 0.3) if mod(_n, 3) == 0
forvalues j = 1/18 {
    quietly summarize v`j'
    display "@ v`j' " %24.17g r(sum) " " %24.17g v`j'[7]
}
display "@ next " %24.17g runiform()
