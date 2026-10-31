* RCraft's Stata corpus (P2B.10): made-up data; Stata 19's results are in descriptives.recorded
* commands: import summarize tabulate correlate pwcorr ttest count generate egen
* tolerance: 1e-8
version 19
import delimited corpus.csv, clear
summarize y, detail
display "@ sum_N " %24.17g r(N)
display "@ sum_mean " %24.17g r(mean)
display "@ sum_sd " %24.17g r(sd)
display "@ sum_p25 " %24.17g r(p25)
display "@ sum_p50 " %24.17g r(p50)
display "@ sum_skew " %24.17g r(skewness)
display "@ sum_kurt " %24.17g r(kurtosis)
summarize x [aw=a]
display "@ sumaw_mean " %24.17g r(mean)
display "@ sumaw_sd " %24.17g r(sd)
summarize x [fw=f]
display "@ sumfw_N " %24.17g r(N)
display "@ sumfw_mean " %24.17g r(mean)
display "@ sumfw_sd " %24.17g r(sd)
tabulate ord g, chi2
display "@ tab_N " %24.17g r(N)
display "@ tab_chi2 " %24.17g r(chi2)
display "@ tab_p " %24.17g r(p)
correlate y x w
display "@ cor_rho " %24.17g r(rho)
correlate y x w, covariance
display "@ cov_12 " %24.17g r(cov_12)
display "@ cov_var1 " %24.17g r(Var_1)
pwcorr y x w
display "@ pw_rho " %24.17g r(rho)
display "@ pw_N " %24.17g r(N)
ttest y == 1
display "@ t1_t " %24.17g r(t)
display "@ t1_p " %24.17g r(p)
display "@ t1_se " %24.17g r(se)
ttest y, by(g)
display "@ t2_t " %24.17g r(t)
display "@ t2_p " %24.17g r(p)
display "@ t2_df " %24.17g r(df_t)
ttest y, by(g) unequal
display "@ t3_t " %24.17g r(t)
display "@ t3_df " %24.17g r(df_t)
ttest y == x
display "@ tp_t " %24.17g r(t)
display "@ tp_mu " %24.17g r(mu_diff)
count if x > 0
display "@ count " %24.17g r(N)
generate double xw = x * w
summarize xw
display "@ xw_mean " %24.17g r(mean)
display "@ xw_sd " %24.17g r(sd)
egen mx = mean(x), by(panel)
summarize mx
display "@ mx_sd " %24.17g r(sd)
display "@ mx_max " %24.17g r(max)
