* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in perfect_predict.recorded
* commands: probit
* (probit and logit with a variable that predicts the outcome perfectly on one side, as a replication package's attrition model has: the variable omitted and its rows not used, then predict, which is missing on those rows by default, in the sample or out of it, and the weights built from it)
version 19
set linesize 255
clear
set seed 77
set obs 400
generate x1 = rnormal()
generate x2 = runiform()
generate byte grid = runiform() < .04
generate byte y = (0.5 * x1 - x2 + rnormal()) > -0.3
replace y = 0 if grid == 1
generate byte out = _n > 380
probit y x1 grid x2 if !out, r
display "@ probit " e(N) " " %20.0g _b[x1] " " %20.0g _b[x2] " " %20.0g _se[x1]
display "@ eb " colsof(e(b)) " " el(e(b), 1, 2) " " %20.0g el(e(b), 1, 3) " " el(e(V), 2, 2)
predict double p
count if missing(p)
display "@ p_missing " r(N)
count if missing(p) & out
display "@ p_missing_out " r(N)
summarize p
display "@ p_sum " %20.0g r(sum) " " r(N)
predict double xb, xb
count if missing(xb)
display "@ xb_missing " r(N)
generate w = 1 / p
regress x1 x2 [pw = w]
display "@ ipw " e(N) " " %20.0g _b[x2]
logit y x1 grid x2
predict double pl
count if missing(pl)
display "@ logit_missing " r(N) " " e(N)
