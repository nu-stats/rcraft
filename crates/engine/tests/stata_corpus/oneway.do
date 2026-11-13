* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in oneway.recorded
* commands: oneway
* (oneway's analysis of variance and Bartlett's test: r() and r(ANOVA), with tabulate, noanova, nomeans and nostandard, a single-observation group left out of Bartlett's test, a string group, if; frequency and analytic weights with the multiple comparisons)
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 12
set obs 90
generate educ = 1 + floor(3*runiform())
generate str12 s = cond(educ == 1, "alpha", "b")
label define el 1 "Primary" 2 "Secondary education" 3 "Tertiary"
label values educ el
generate y = 10 + 2*educ + 3*rnormal()
replace y = . in 2/3
oneway y educ
display "@ a " r(N) " " %24.17g r(mss) " " r(df_m) " " %24.17g r(rss) " " r(df_r) " " %24.17g r(F)
display "@ bart " %24.17g r(chi2bart) " " r(df_bart)
display "@ anova " %24.17g el(r(ANOVA), 1, 3) " " %24.17g el(r(ANOVA), 2, 3) " " %24.17g el(r(ANOVA), 3, 3) " " %24.17g el(r(ANOVA), 1, 5)
oneway y educ, tabulate
display "@ tab " r(N) " " %24.17g r(F)
replace y = 4 in 1
replace educ = 9 in 1
oneway y educ, tabulate
display "@ single " r(N) " " %24.17g r(F) " " %24.17g r(chi2bart) " " r(df_bart)
oneway y s, tabulate
display "@ string " r(N) " " %24.17g r(F) " " %24.17g r(chi2bart) " " r(df_bart)
oneway y educ, noanova tabulate
display "@ noanova " r(N) " " r(F)
oneway y educ if educ < 3
display "@ if " r(N) " " %24.17g r(F) " " r(df_m)
quietly oneway y educ
display "@ quiet " %24.17g r(rss)
generate w = 1 + mod(_n, 3)
oneway y educ [fweight = w]
display "@ fw " r(N) " " %24.17g r(mss) " " %24.17g r(rss) " " r(df_r) " " %24.17g r(F) " " %24.17g r(chi2bart)
oneway y educ [aweight = w], bonferroni scheffe sidak
display "@ aw " r(N) " " %24.17g r(mss) " " %24.17g r(rss) " " r(df_r) " " %24.17g r(F) " " %24.17g r(chi2bart)
oneway y educ, tabulate nomeans nostandard
display "@ nomeans " r(N) " " %24.17g r(F)
