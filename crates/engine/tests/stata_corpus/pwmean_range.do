* RCraft's Stata corpus (P2C.12c, batch C, second round): made-up data; Stata 19's results are in pwmean_range.recorded
* commands: pwmean
* (pwmean with mcompare(snk) and mcompare(duncan), compared to 1e-6 because R's studentized range is the more exact: Stata's critical value is about 3e-8 off for spans of five means and more, which a limit near zero shows as 1e-6: the studentized-range critical value for the number of means a pair spans in the sorted means, with unequal and equal group sizes, level(), cimeans, sort, the number of letters, one comparison and one or no residual degrees of freedom)
* tolerance: 1e-6
version 19
set linesize 255
clear
set obs 40
generate double x1 = mod(_n * 37, 101) / 7
generate double y = 2 + .5 * x1 + mod(_n * 7, 11) / 5
generate int g = 1 + mod(_n, 4)
generate double z = y + 4 * (g == 3) + 2.2 * (g == 2) - 3 * (g == 4)
generate int u = ceil(sqrt(_n))
generate double zu = y + u * .7 + mod(_n * 13, 7) / 3
* unequal group sizes: every pair has its own standard error and its own number of means spanned
pwmean zu, over(u) mcompare(snk) effects
display "@ snk_t " r(mctitle_vs) " " r(mcmethod_vs) " " el(r(k_groups), 1, 1) " " colsof(r(table_vs)) " " e(df_r)
forvalues j = 1/21 {
    display "@ snk`j' " %24.17g el(r(table_vs), 3, `j') " " %24.17g el(r(table_vs), 4, `j') " " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
}
pwmean zu, over(u) mcompare(duncan) effects
display "@ dun_t " r(mctitle_vs) " " r(mcmethod_vs) " " el(r(k_groups), 1, 1) " " colsof(r(table_vs)) " " e(df_r)
forvalues j = 1/21 {
    display "@ dun`j' " %24.17g el(r(table_vs), 3, `j') " " %24.17g el(r(table_vs), 4, `j') " " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
}
* the letters: Duncan separates the highest mean from three of the others, and the sets that cannot be told apart are not runs of neighbours
pwmean zu, over(u) mcompare(snk) groups
display "@ snk_k " el(r(k_groups), 1, 1) " " r(level)
pwmean zu, over(u) mcompare(duncan) groups
display "@ dun_k " el(r(k_groups), 1, 1) " " r(level)
pwmean zu, over(u) mcompare(duncan) groups level(90)
display "@ dun_k90 " el(r(k_groups), 1, 1) " " r(level)
* equal group sizes, means not in the order of the groups
pwmean z, over(g) mcompare(snk) effects
display "@ bsnk " r(mcmethod_vs) " " el(r(k_groups), 1, 1)
forvalues j = 1/6 {
    display "@ bsnk`j' " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
}
pwmean z, over(g) mcompare(duncan) effects
forvalues j = 1/6 {
    display "@ bdun`j' " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
}
pwmean z, over(g) mcompare(snk) groups
display "@ bsnk_k " el(r(k_groups), 1, 1)
pwmean z, over(g) mcompare(duncan) groups
display "@ bdun_k " el(r(k_groups), 1, 1)
* three groups, level(), sort, cimeans
pwmean z if g < 4, over(g) mcompare(snk) effects
forvalues j = 1/3 {
    display "@ tsnk`j' " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
}
pwmean z if g < 4, over(g) mcompare(duncan) effects
forvalues j = 1/3 {
    display "@ tdun`j' " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
}
pwmean zu, over(u) mcompare(snk) pveffects level(90) sort
display "@ l90 " %24.17g el(r(table_vs), 4, 7) " " %24.17g el(r(table_vs), 5, 7) " " %24.17g el(r(table_vs), 8, 7)
pwmean zu, over(u) mcompare(duncan) level(80)
display "@ l80 " %24.17g el(r(table_vs), 5, 8) " " %24.17g el(r(table_vs), 6, 8) " " %24.17g el(r(table_vs), 8, 8)
pwmean zu, over(u) mcompare(snk) cimeans level(80)
display "@ cm80 " %24.17g el(r(table), 5, 3) " " %24.17g el(r(table_vs), 5, 8)
* one comparison: the method is ignored
pwmean z if g < 3, over(g) mcompare(snk)
display "@ one1 " r(mcmethod_vs) " " r(mctitle_vs) " " %24.17g el(r(table_vs), 8, 1)
pwmean z if g < 3, over(g) mcompare(duncan)
display "@ one2 " r(mcmethod_vs) " " r(mctitle_vs) " " %24.17g el(r(table_vs), 8, 1)
* no or one residual degree of freedom: missing p-values, critical values and limits
pwmean z in 1/4, over(g) mcompare(snk) effects
display "@ d0a " e(df_r) " " el(r(table_vs), 2, 1) " " el(r(table_vs), 4, 1) " " el(r(table_vs), 8, 1)
pwmean z if inlist(_n, 1, 2, 4, 8), over(g) mcompare(snk) effects
display "@ d1a " e(df_r) " " %24.17g el(r(table_vs), 3, 2) " " el(r(table_vs), 4, 2) " " el(r(table_vs), 5, 2) " " el(r(table_vs), 8, 2)
pwmean z if inlist(_n, 1, 2, 4, 8), over(g) mcompare(duncan) effects
display "@ d1b " e(df_r) " " %24.17g el(r(table_vs), 3, 2) " " el(r(table_vs), 4, 2) " " el(r(table_vs), 5, 2) " " el(r(table_vs), 8, 2)
pwmean z if inlist(_n, 1, 2, 4, 8), over(g) mcompare(duncan) groups
display "@ d1c " el(r(k_groups), 1, 1)
