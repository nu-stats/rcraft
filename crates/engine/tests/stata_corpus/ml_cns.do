* RCraft's Stata corpus (P2C.12, batch E leftovers): made-up data; Stata 19's results are in ml_cns.recorded
* commands: logit probit poisson logistic
* (constraints() in logit, probit, poisson and logistic: one and several constraints, a slope or the constant fixed, an equation-named constraint, odds ratios and IRR, robust and clustered errors, frequency and sampling weights, if, noconstant (with and without constraints), then predict and test after it; e(b), e(V), e(Cns), the Wald test and the rank)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 40
generate double x1 = mod(_n * 37, 101) / 7
generate double x2 = mod(_n * 13, 17) / 3
generate double x3 = mod(_n * 11, 23) / 4
generate int g = 1 + mod(_n, 4)
generate int w = 1 + mod(_n, 3)
generate double w2 = .5 + mod(_n * 7, 13) / 10
generate byte b = mod(_n * 7, 5) < 2
generate int k = mod(_n * 3, 7)
program define shwml
    display "@ s " e(N) " " %24.17g e(ll) " " e(rank) " " e(df_m) " " %24.17g e(chi2) " " %24.17g e(p) " " e(converged)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(e(V), 1, `j')
    }
    forvalues i = 1/`=rowsof(e(Cns))' {
        forvalues j = 1/`=colsof(e(Cns))' {
            display "@ c`i'_`j' " %24.17g el(e(Cns), `i', `j')
        }
    }
end
constraint define 1 x1 = x2
constraint 2 x3 = -0.2
constraint 4 [b]x2 = 0.1
constraint 5 _cons = 2
logit b x1 x2 x3, constraints(1)
shwml
display "@ e1 " (e(cmd) == "logit") " " (e(chi2type) == "Wald") " " (e(vce) == "oim") " " missing(e(ll_0)) " " missing(e(r2_p))
logit b x1 x2 x3, constraints(1 2)
shwml
logit b x1 x2 x3, constraints(1) or
shwml
logit b x1 x2 x3, constraints(1) vce(robust)
shwml
logit b x1 x2 x3, constraints(1) vce(cluster g)
shwml
display "@ e2 " e(N_clust)
logit b x1 x2 x3 [fweight = w], constraints(1)
shwml
logit b x1 x2 x3 [pweight = w2], constraints(1)
shwml
logit b x1 x2 x3, constraints(5)
shwml
logit b x1 x2 x3, constraints(1 5)
shwml
predict double p1
summarize p1
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
test x1
display "@ t1 " %24.17g r(chi2) " " r(df) " " %24.17g r(p)
logit b x1 x2 x3, constraints(4)
shwml
logit b x1 x2 x3 if g < 4, constraints(1)
shwml
logistic b x1 x2 x3, constraints(1)
shwml
display "@ e3 " (e(cmd) == "logistic")
probit b x1 x2 x3, constraints(1)
shwml
probit b x1 x2 x3, constraints(1 2)
shwml
probit b x1 x2 x3, constraints(1) vce(robust)
shwml
poisson k x1 x2 x3, constraints(1)
shwml
poisson k x1 x2 x3, constraints(1 2)
shwml
display "@ e4 " (e(cmd) == "poisson") " " (e(chi2type) == "Wald") " " missing(e(r2_p))
poisson k x1 x2 x3, constraints(1) irr
shwml
poisson k x1 x2 x3, constraints(1) vce(robust)
shwml
poisson k x1 x2 x3 [fweight = w], constraints(1 2)
shwml
predict double m1
summarize m1
display "@ p2 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
* without the constant: the same Wald header, the log from 0, no e(ll_0)
program define shwnc
    display "@ n " e(N) " " %24.17g e(ll) " " e(rank) " " e(df_m) " " %24.17g e(chi2) " " %24.17g e(p) " " missing(e(ll_0)) " " missing(e(r2_p))
    forvalues j = 1/`=colsof(e(b))' {
        display "@ nb`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(e(V), 1, `j')
    }
end
logit b x1 x2, noconstant
shwnc
display "@ e5 " (e(chi2type) == "Wald")
logit b x1 x2, noconstant or
shwnc
logit b x1 x2, noconstant vce(robust)
shwnc
logit b x1 x2 [fweight = w], noconstant
shwnc
probit b x1 x2, noconstant
shwnc
logistic b x1 x2, noconstant
shwnc
display "@ e6 " (e(cmd) == "logistic")
logistic b x1 x2, noconstant coef
shwnc
logit b x1 x2 x3, noconstant constraints(1)
shwml
