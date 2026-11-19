* RCraft's Stata corpus (P2C.12c, batch C): made-up data; Stata 19's results are in pwmean_tukey.recorded
* commands: pwmean
* (pwmean with mcompare(tukey): the critical value and p-values of the studentized range, with equal and unequal group sizes, level(), several over() variables, the number of letters and one comparison)
* tolerance: 1e-7
version 19
set linesize 255
clear
set obs 40
generate double x1 = mod(_n * 37, 101) / 7
generate double y = 2 + .5 * x1 + mod(_n * 7, 11) / 5
generate int g = 1 + mod(_n, 4)
generate int w = 1 + mod(_n, 3)
generate double z = y + 4 * (g == 3) + 2.2 * (g == 2) - 3 * (g == 4)
pwmean z, over(g) mcompare(tukey) effects
display "@ t_tukey " r(mctitle_vs) " " r(mcmethod_vs) " " el(r(k_groups), 1, 1)
forvalues j = 1/6 {
    display "@ tukey`j' " %24.17g el(r(table_vs), 3, `j') " " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
}
display "@ tukeyp " %24.17g el(r(table_vs), 4, 1) " " %24.17g el(r(table_vs), 4, 4)
pwmean z, over(g) mcompare(tukey) level(90) groups
display "@ tl90 " r(level) " " el(r(k_groups), 1, 1) " " %24.17g el(r(table_vs), 5, 3) " " %24.17g el(r(table_vs), 8, 3) " " %24.17g el(r(table_vs), 4, 4)
pwmean z, over(g) mcompare(tukey) level(90) effects
display "@ tl90e " %24.17g el(r(table_vs), 5, 3) " " %24.17g el(r(table_vs), 8, 3) " " %24.17g el(r(table), 8, 3)
pwmean z, over(g) mcompare(tukey) cimeans
display "@ tc " r(mcmethod_vs) " " %24.17g el(r(table), 5, 1) " " %24.17g el(r(table), 6, 4) " " %24.17g el(r(table_vs), 8, 1)
pwmean z, over(g) mcompare(tukey) groups
display "@ tg " el(r(k_groups), 1, 1)
pwmean z if _n > 5, over(g) mcompare(tukey) effects
display "@ ub " e(balanced) " " e(df_r) " " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 6, 6) " " %24.17g el(r(table_vs), 4, 1)
display "@ ub2 " %24.17g el(r(table_vs), 8, 1) " " %24.17g el(r(table_vs), 3, 2) " " %24.17g el(r(table_vs), 4, 3)
pwmean y in 1/30, over(g w) mcompare(tukey) effects
display "@ ot " %24.17g el(r(table_vs), 4, 1) " " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 8, 1) " " %24.17g el(r(table_vs), 4, 66)
pwmean z if g <= 2, over(g) mcompare(tukey) effects
display "@ two " r(mcmethod_vs) " " colsof(r(table_vs)) " " %24.17g el(r(table_vs), 5, 1)
pwmean z, over(g) mcompare(tuk) pveffects
display "@ tp " %24.17g el(r(table_vs), 4, 1) " " %24.17g el(r(table_vs), 4, 4) " " %24.17g el(r(table_vs), 8, 1)
