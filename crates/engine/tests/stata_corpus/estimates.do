* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in estimates.recorded
* commands: estimates
* (estimates store, restore, drop and clear: _b, _se, e() and predict after a restore, a model stored under quietly, after a later model and after new variables; a name dropped or cleared is r(111))
* tolerance: 1e-8
version 19
import delimited corpus.csv, clear
regress y x w
est sto a
logit yb x
estimates store b
regress y w
predict double p1
estimates restore a
display "@ a_b " %24.17g _b[x]
display "@ a_se " %24.17g _se[w]
display "@ a_n " e(N)
display "@ a_dfm " e(df_m)
predict double p2
summarize p2
display "@ a_p " %24.17g r(sum)
est res b
display "@ b_b " %24.17g _b[x]
display "@ b_ll " %24.17g e(ll)
predict double p3
summarize p3
display "@ b_p " %24.17g r(sum)
quietly regress y x if g == 1
estimates store c
estimates restore a
estimates restore c
display "@ c_b " %24.17g _b[x]
display "@ c_n " e(N)
generate double z2 = z^2
estimates restore a
predict double p4
summarize p4
display "@ a_p4 " %24.17g r(sum)
estimates drop b
capture estimates restore b
display "@ drop_rc " _rc
display "@ still_a " %24.17g _b[x]
estimates clear
capture estimates restore a
display "@ clear_rc " _rc
