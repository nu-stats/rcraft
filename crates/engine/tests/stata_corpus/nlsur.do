* RCraft's Stata corpus (P2C.12ao, batch E on-hold item): made-up data; Stata 19's results are in nlsur.recorded
* commands: nlsur
* (nlsur: NLS, FGNLS and iterated FGNLS on two and three equations, shared parameters, a start where parameters stay constrained, noconstant, if and in, initial(), eps(), level(), one equation, e() and predict with equation(#k); the iteration log is compared by eye, not here)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 150
generate double x1 = 1 + mod(_n * 37, 101) / 20
generate double x2 = 1 + mod(_n * 13, 17) / 5
generate double x3 = 1 + mod(_n * 7, 23) / 6
generate double e1 = (mod(_n * 29, 19) - 9) / 40
generate double e2 = 0.5 * e1 + (mod(_n * 7, 11) - 5) / 50
generate double e3 = -0.3 * e1 + (mod(_n * 3, 13) - 6) / 60
generate double y1 = 1 + 0.8 * x1 + e1
generate double y2 = 2 + 1.5 * exp(0.3 * x2) + e2
generate double y3 = 5 / (1 + exp(-0.8 * (x3 - 3))) + e3
program define shwnl
    display "@ s " %24.17g e(N) " " %24.17g e(k) " " %24.17g e(rank) " " %24.17g e(ll) " " e(converged) " " e(n_eq) " " e(k_eq) " " e(k_eq_model)
    forvalues i = 1/`=e(n_eq)' {
        display "@ q`i' " %24.17g e(rss_`i') " " %24.17g e(mss_`i') " " %24.17g e(r2_`i') " " %24.17g e(rmse_`i') " " e(k_`i')
    }
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
    forvalues i = 1/`=rowsof(e(Sigma))' {
        forvalues j = 1/`=colsof(e(Sigma))' {
            display "@ g`i'_`j' " %24.17g el(e(Sigma), `i', `j')
        }
    }
    forvalues j = 1/`=colsof(e(init))' {
        display "@ i`j' " %24.17g el(e(init), 1, `j')
    }
end
nlsur (y1 = {a1=1} + {b1=1}*x1) (y2 = {a2=1} + {b2=1}*exp({c=.5}*x2)), nolog
shwnl
display "@ m1 " (e(cmd) == "nlsur") " " (e(method) == "fgnls") " " (e(vce) == "gnr") " " (e(params) == "a1 b1 a2 b2 c") " " (e(constants) == "a1 a2") " " (e(depvar) == "y1 y2") " " (e(params_2) == "a2 b2 c") " " (e(depvar_1) == "y1") " " (e(sexp_2) == "{a2} + {b2} *exp( {c} *x2)") " " (e(sexp_1) == "{a1} + {b1} *x1") " " (e(type) == "1")
display "@ v1 " %24.17g el(e(V), 1, 2) " " %24.17g el(e(V), 3, 5) " " %24.17g el(e(V), 4, 5)
nlsur (y1 = {a1=1} + {b1=1}*x1) (y2 = {a2=1} + {b2=1}*exp({c=.5}*x2)), nls
shwnl
display "@ m2 " (e(method) == "nls") " " (e(vce) == "gnr")
display "@ v2 " %24.17g el(e(V), 1, 2) " " %24.17g el(e(V), 3, 5) " " %24.17g el(e(V), 4, 5)
nlsur (y1 = {a1=1} + {b1=1}*x1) (y2 = {a2=1} + {b2=1}*exp({c=.5}*x2)), ifgnls
shwnl
display "@ m3 " (e(method) == "ifgnls")
display "@ v3 " %24.17g el(e(V), 1, 2) " " %24.17g el(e(V), 3, 5) " " %24.17g el(e(V), 4, 5)
nlsur (y1 = {a1=1} + {b1=1}*x1) (y2 = {a2=1} + {b2=1}*exp({c=.5}*x2)), fgnls eps(1e-3)
shwnl
nlsur (y1 = {a1=1} + {b1=1}*x1) (y2 = {a2=1} + {b2=1}*exp({c=.5}*x2)), nls eps(1e-3) level(90)
shwnl
display "@ v4 " %24.17g el(e(V), 1, 2) " " %24.17g el(e(V), 3, 5) " " %24.17g el(e(V), 4, 5)
nlsur (y1 = {a1} + {b1}*x1) (y2 = {a2} + {b2}*exp({c}*x2)), initial(a1 1 b1 1 a2 1 b2 1 c .5) nolog
shwnl
nlsur (y1 = {a1} + {b1}*x1) (y2 = {a2} + {b2}*exp({c}*x2)), nolog
shwnl
display "@ m5 " (e(constants) == "a1 a2") " " (e(rank) == 3)
nlsur (y1 = {a1=1} + {b1=1}*x1) (y2 = {a2=1} + {b2=1}*exp({c=.5}*x2)) if x1 > 3 in 1/120, nolog
shwnl
nlsur (y1 = {a1=1} + {b1=1}*x1) (y2 = {b2=1}*exp({c=.5}*x2)), nolog
shwnl
display "@ m6 " (e(constants) == "a1 .")
nlsur (y1 = {a1=1} + {b1=1}*x1) (y2 = {a2=1} + {b2=1}*exp({c=.5}*x2)), noconstant nolog nls
shwnl
nlsur (y1 = {b1=1}*x1) (y2 = {b2=1}*exp({c=.5}*x2)), nolog
shwnl
display "@ m7 " (e(constants) == ". .")
nlsur (y1 = {a1=1} + {b1=1}*x1), nolog
shwnl
nlsur (y1 = {a=1} + {b=1}*x1) (y2 = {a} + {b}*x2), nolog
shwnl
nlsur (y1 = {A=1} * x1^{p=.5}) (y3 = {K=4} / (1 + exp(-{r=1} * (x3 - {m=2})))) (y2 = {c0=1} + {c1=1} * ln(x1) + {c2=0} * x2), nolog
shwnl
display "@ m8 " (e(params) == "A p K r m c0 c1 c2") " " (e(constants) == ". . c0")
nlsur (y1 = {A=1} * x1^{p=.5}) (y3 = {K=4} / (1 + exp(-{r=1} * (x3 - {m=2})))), nls
shwnl
nlsur (y1 = {A=1}*x1^{p=.5}) (y2 = {c0=1} + {c1=1} * ln(x1) + {c2=0} * x2) if x3 > 2, ifgnls nolog
shwnl
nlsur (y1 = {a1=1} + {b1=1}*x1) (y2 = {a2=1} + {b2=1}*exp({c=.5}*x2)), nolog
predict yh
predict yh1, equation(#1)
predict yh2, equation(#2) yhat
predict rs2, equation(#2) residuals
predict rs1, residuals
generate double q2 = rs2^2
generate double q1 = rs1^2
summarize yh1
display "@ p1 " %24.17g r(sum) " " r(N)
summarize yh2
display "@ p2 " %24.17g r(sum) " " r(N)
summarize q1
display "@ p3 " %24.17g r(sum)
summarize q2
display "@ p4 " %24.17g r(sum)
display "@ p5 " %24.17g _b[a2:_cons] " " %24.17g _se[c:_cons]
count if yh == yh1
display "@ p6 " r(N)
