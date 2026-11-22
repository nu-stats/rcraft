* RCraft's Stata corpus (P2C.12, batch E follow-on): made-up data; Stata 19's results are in ml_iterate.recorded
* commands: logit probit poisson
* (iterate(#) on logit, probit and poisson: the Newton path stopped after # steps with convergence not achieved and the warning, converged inside the cap, nolog, robust errors, odds ratios and IRR, e(converged), the estimates and variance where the path stopped, predict after it)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 120
generate double x1 = mod(_n * 37, 101) / 20 - 2
generate double x2 = mod(_n * 13, 17) / 5
generate byte b = mod(_n * 7, 5) < 2
generate int k = mod(_n * 3, 7)
generate int w = 1 + mod(_n, 3)
program define shwit
    display "@ s " e(N) " " %24.17g e(ll) " " e(rank) " " e(df_m) " " e(converged) " " e(rc)
    display "@ t " %24.17g round(e(chi2), 1e-9) " " %24.17g e(p) " " %24.17g round(e(r2_p), 1e-9) " " %24.17g e(ll_0)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(e(V), 1, `j')
    }
end
logit b x1 x2, iterate(0)
shwit
logit b x1 x2, iterate(1)
shwit
logit b x1 x2, iterate(2)
shwit
logit b x1 x2, iterate(1) nolog
shwit
logit b x1 x2, iterate(2) vce(robust)
shwit
logit b x1 x2, iterate(1) or
shwit
logit b x1 x2, iterate(20)
shwit
display "@ e1 " (e(cmd) == "logit")
predict double p1
summarize p1
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
logit b x1 x2 [fweight = w], iterate(2)
shwit
probit b x1 x2, iterate(0)
shwit
probit b x1 x2, iterate(1)
shwit
probit b x1 x2, iterate(2) nolog
shwit
probit b x1 x2, iterate(15)
shwit
poisson k x1 x2, iterate(0)
shwit
poisson k x1 x2, iterate(1)
shwit
poisson k x1 x2, iterate(2)
shwit
poisson k x1 x2, iterate(1) irr
shwit
poisson k x1 x2, iterate(1) vce(robust)
shwit
poisson k x1 x2, iterate(0) nolog
shwit
predict double m1
summarize m1
display "@ p2 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
