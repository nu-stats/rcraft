* RCraft's Stata corpus (P2C.12, batch E): made-up data; Stata 19's results are in logistic.recorded
* commands: logistic
* (logistic: logit with odds ratios and no iteration log; coef, level(), vce(robust), if, pweights, e() as logit's with e(cmd) logistic, predict and estat after it)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 120
generate double x = mod(_n * 37, 101) / 20
generate double z = mod(_n * 13, 17) / 9
generate byte b = mod(_n * 7, 5) < 2 | x > 4
program define shwlg
    display "@ s " %24.17g e(N) " " %24.17g e(ll) " " %24.17g e(ll_0) " " %24.17g e(chi2) " " %24.17g e(p) " " %24.17g e(r2_p) " " e(df_m) " " e(rank)
    forvalues j = 1/`=colsof(e(b))' {
        display "@ b`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j')
    }
end
logistic b x z
shwlg
display "@ m1 " (e(cmd) == "logistic") " " (e(predict) == "logistic_p") " " (e(title) == "Logistic regression") " " (e(depvar) == "b") " " (e(vce) == "oim")
logistic b x z, coef
shwlg
logistic b x z, vce(robust)
shwlg
display "@ m2 " (e(vcetype) == "Robust") " " (e(chi2type) == "Wald")
logistic b x z if x > 1, level(90)
shwlg
logistic b x z [pweight = x]
shwlg
logistic b x z
predict double pr
predict double xb, xb
summarize pr xb
display "@ p1 " %24.17g r(mean) " " %24.17g r(sd) " " %24.17g r(min) " " %24.17g r(max)
test x
display "@ t1 " %24.17g r(chi2) " " %24.17g r(p) " " r(df)
