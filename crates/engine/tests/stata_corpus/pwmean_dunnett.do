* RCraft's Stata corpus (P2C.12c, batch C, second round): made-up data; Stata 19's results are in pwmean_dunnett.recorded
* commands: pwmean
* (pwmean with mcompare(dunnett): each group against the first, with Dunnett's critical value and p-values for the equicorrelated case whatever the group sizes, level(), sort, cimeans, several over() variables, the error with groups, one comparison and one or no residual degrees of freedom; Stata's own dunnettprob is approximate, so these are compared to 1e-3 and only p-values that are not tiny are recorded)
* tolerance: 1e-3
version 19
set linesize 255
clear
set obs 40
generate double x1 = mod(_n * 37, 101) / 7
generate double y = 2 + .5 * x1 + mod(_n * 7, 11) / 5
generate int g = 1 + mod(_n, 4)
generate int w = 1 + mod(_n, 3)
generate double z = y + 4 * (g == 3) + 2.2 * (g == 2) - 3 * (g == 4)
generate int u = ceil(sqrt(_n))
generate double zu = y + u * .7 + mod(_n * 13, 7) / 3
* equal group sizes: three comparisons with the first group
pwmean z, over(g) mcompare(dunnett) effects
display "@ db_t " r(mctitle_vs) " " r(mcmethod_vs) " " colsof(r(table_vs)) " " colsof(e(b_vs)) " " colsof(e(b)) " " e(df_r)
forvalues j = 1/3 {
    display "@ db`j' " %24.17g el(r(table_vs), 3, `j') " " %24.17g el(r(table_vs), 4, `j') " " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
}
display "@ dbv " %24.17g el(e(b_vs), 1, 2) " " %24.17g el(e(V_vs), 1, 1) " " %24.17g el(e(V_vs), 2, 1) " " %24.17g el(e(V_vs), 3, 3) " " rowsof(e(V_vs))
display "@ dbs " r(level) " " rowsof(r(table_vs)) " " colsof(r(table))
* unequal group sizes: the correlation is still one half
pwmean zu, over(u) mcompare(dunnett) effects
forvalues j = 1/6 {
    display "@ du`j' " %24.17g el(r(table_vs), 3, `j') " " %24.17g el(r(table_vs), 4, `j') " " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
}
display "@ duv " %24.17g el(e(V_vs), 1, 1) " " %24.17g el(e(V_vs), 2, 1) " " %24.17g el(e(V_vs), 6, 6) " " e(balanced)
* level(), sort and cimeans
pwmean zu, over(u) mcompare(dunnett) pveffects level(90)
display "@ l90 " %24.17g el(r(table_vs), 4, 4) " " %24.17g el(r(table_vs), 5, 4) " " %24.17g el(r(table_vs), 8, 4)
pwmean zu, over(u) mcompare(dunnett) level(80) sort
display "@ l80 " %24.17g el(r(table_vs), 5, 5) " " %24.17g el(r(table_vs), 6, 5) " " %24.17g el(r(table_vs), 8, 5)
pwmean z, over(g) mcompare(dunnett) cimeans level(80)
display "@ cm80 " %24.17g el(r(table), 5, 2) " " %24.17g el(r(table_vs), 5, 2) " " %24.17g el(r(table_vs), 8, 2)
* eleven comparisons with two over() variables
pwmean z, over(g w) mcompare(dunnett)
display "@ tw " colsof(r(table_vs)) " " colsof(e(b)) " " %24.17g el(r(table_vs), 8, 1) " " %24.17g el(r(table_vs), 5, 4) " " %24.17g el(r(table_vs), 6, 9)
* dunnett with groups is r(198), even with two groups
capture pwmean z, over(g) mcompare(dunnett) groups
display "@ rc_dg " _rc
capture pwmean z if g < 3, over(g) mcompare(dunnett) groups
display "@ rc_dg2 " _rc
* one comparison: the method is ignored
pwmean z if g < 3, over(g) mcompare(dunnett)
display "@ one " r(mcmethod_vs) " " r(mctitle_vs) " " %24.17g el(r(table_vs), 8, 1)
* no or one residual degree of freedom
pwmean z in 1/4, over(g) mcompare(dunnett) effects
display "@ d0 " e(df_r) " " colsof(r(table_vs)) " " el(r(table_vs), 2, 1) " " el(r(table_vs), 4, 1) " " el(r(table_vs), 8, 1)
pwmean z if inlist(_n, 1, 2, 4, 8), over(g) mcompare(dunnett) effects
display "@ d1 " e(df_r) " " colsof(r(table_vs)) " " %24.17g el(r(table_vs), 3, 2) " " el(r(table_vs), 4, 2) " " el(r(table_vs), 5, 2) " " el(r(table_vs), 8, 2)
* a bare pwmean with another method
quietly pwmean z, over(g)
pwmean, mcompare(dunnett) pveffects
display "@ rpd " r(mcmethod_vs) " " colsof(r(table_vs)) " " %24.17g el(r(table_vs), 8, 1)
capture pwmean, mcompare(dunnett) groups
display "@ rc_rpd " _rc
