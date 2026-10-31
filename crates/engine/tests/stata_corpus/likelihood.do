* RCraft's Stata corpus (P2B.10): made-up data; Stata 19's results are in likelihood.recorded
* Stata stops iterating at its tolerance, so these agree to about 7 digits
* commands: import logit probit poisson ologit oprobit mlogit xtset xtreg stset stcox
* tolerance: 1e-6
version 19
import delimited corpus.csv, clear
logit yb x w
display "@ lg_b_x " %24.17g _b[x]
display "@ lg_se_x " %24.17g _se[x]
display "@ lg_ll " %24.17g e(ll)
display "@ lg_chi2 " %24.17g e(chi2)
logit yb x w, robust
display "@ lgr_se_x " %24.17g _se[x]
probit yb x w
display "@ pb_b_x " %24.17g _b[x]
display "@ pb_se_x " %24.17g _se[x]
display "@ pb_ll " %24.17g e(ll)
poisson cnt x w
display "@ ps_b_x " %24.17g _b[x]
display "@ ps_se_x " %24.17g _se[x]
display "@ ps_ll " %24.17g e(ll)
ologit ord x w
display "@ ol_b_x " %24.17g _b[x]
display "@ ol_cut1 " %24.17g _b[/cut1]
display "@ ol_ll " %24.17g e(ll)
oprobit ord x w
display "@ op_b_x " %24.17g _b[x]
display "@ op_ll " %24.17g e(ll)
mlogit cat3 x, baseoutcome(1)
display "@ ml_b3 " %24.17g _b[3:x]
display "@ ml_se3 " %24.17g _se[3:x]
display "@ ml_ll " %24.17g e(ll)
xtset panel t
xtreg y x w, mle
display "@ xm_b_x " %24.17g _b[x]
display "@ xm_ll " %24.17g e(ll)
display "@ xm_sigma_u " %24.17g e(sigma_u)
stset time, failure(died)
stcox x w
display "@ cx_b_x " %24.17g _b[x]
display "@ cx_se_x " %24.17g _se[x]
display "@ cx_ll " %24.17g e(ll)
