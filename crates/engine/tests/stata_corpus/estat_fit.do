* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in estat_fit.recorded
* commands: estat
* (estat imtest with white after regress; classification after logit and probit with cutoff(); gof by covariate patterns and Hosmer-Lemeshow with group(); to 5 digits: imtest's statistics are Stata's to 3e-6, its skewness and kurtosis parts not further)
* tolerance: 1e-5
version 19
clear
set linesize 255
set seed 91
set obs 150
generate x = rnormal()
generate w = .6*x + rnormal()
generate g = 1 + floor(3*runiform())
generate y = 1 + .5*x + .2*w + rnormal()
generate yb = y > 1.2
generate z = round(x)
regress y x w i.g
estat imtest, white
display "@ im " %24.17g r(chi2_h) " " r(df_h) " " %24.17g r(chi2_s) " " r(df_s) " " %24.17g r(chi2_k) " " r(df_t) " " %24.17g r(chi2)
logit yb x w
estat classification
display "@ cl " %24.17g r(P_corr) " " %24.17g r(P_p1) " " %24.17g r(P_0n) " " el(r(ctable),1,1) " " el(r(ctable),3,3)
estat classification, cutoff(.3)
display "@ cl3 " %24.17g r(P_corr) " " %24.17g r(P_1p)
estat gof
display "@ gof " %24.17g r(chi2) " " r(df) " " r(m) " " %24.17g r(p)
estat gof, group(10)
display "@ hl " %24.17g r(chi2) " " r(df) " " r(m) " " %24.17g r(p)
logit yb z
estat gof
display "@ gofp " %24.17g r(chi2) " " r(df) " " r(m)
estat gof, group(4) table
display "@ hl4 " %24.17g r(chi2) " " r(df) " " r(m)
probit yb x w
estat classification
display "@ pcl " %24.17g r(P_corr) " " %24.17g r(P_n0)
