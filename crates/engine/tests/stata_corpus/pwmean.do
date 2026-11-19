* RCraft's Stata corpus (P2C.12c, batch C): made-up data; Stata 19's results are in pwmean.recorded
* commands: pwmean
* (pwmean: pairwise comparisons of means with equal variances, unadjusted and with Bonferroni, Sidak and Scheffe adjustments (Tukey is in pwmean_tukey.do), level(), cimeans, sort, groups (the number of letters), one and several over() variables, unbalanced groups, if and in, missing values, one comparison (the method is ignored), the matrices e(b), e(V), e(b_vs), e(V_vs) and r(table), r(table_vs), r(k_groups), and the errors with their return codes)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 40
generate double x1 = mod(_n * 37, 101) / 7
generate double y = 2 + .5 * x1 + mod(_n * 7, 11) / 5
generate int g = 1 + mod(_n, 4)
generate int w = 1 + mod(_n, 3)
generate double z = y + 4 * (g == 3) + 2.2 * (g == 2) - 3 * (g == 4)
replace y = . in 5
pwmean y, over(g)
display "@ e1 " e(balanced) " " e(df_r) " " colsof(e(b)) " " colsof(e(b_vs)) " " rowsof(e(V_vs))
display "@ r1 " r(level) " " colsof(r(table)) " " colsof(r(table_vs)) " " rowsof(r(table_vs)) " " el(r(k_groups), 1, 1)
display "@ t1 " r(mctitle_vs) " " r(mcmethod_vs)
forvalues j = 1/4 {
    display "@ m`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(r(table), 2, `j') " " %24.17g el(r(table), 3, `j') " " %24.17g el(r(table), 4, `j')
    display "@ n`j' " %24.17g el(r(table), 5, `j') " " %24.17g el(r(table), 6, `j') " " el(r(table), 7, `j') " " %24.17g el(r(table), 8, `j') " " el(r(table), 9, `j')
}
forvalues j = 1/6 {
    display "@ u`j' " %24.17g el(e(b_vs), 1, `j') " " %24.17g el(r(table_vs), 2, `j') " " %24.17g el(r(table_vs), 3, `j') " " %24.17g el(r(table_vs), 4, `j')
    display "@ v`j' " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " el(r(table_vs), 7, `j') " " %24.17g el(r(table_vs), 8, `j') " " el(r(table_vs), 9, `j')
    display "@ x`j' " %24.17g el(e(V_vs), `j', 1) " " %24.17g el(e(V_vs), `j', 4) " " %24.17g el(e(V_vs), `j', `j') " " el(e(error_vs), 1, `j')
}
display "@ e2 " el(e(error), 1, 1) " " el(e(error), 1, 4) " " el(e(V), 1, 2) " " el(e(V), 4, 3)
display "@ s1 " e(cmd) " " e(depvar) " " e(over) " " e(properties)
* the unadjusted columns again with effects and sort, which change the printing only
pwmean y, over(g) effects sort
display "@ a1 " %24.17g el(r(table_vs), 4, 1) " " %24.17g el(r(table_vs), 4, 6) " " %24.17g el(e(b_vs), 1, 2)
* adjusted: each method's critical value, p-values and intervals
foreach m in bonferroni sidak scheffe {
    pwmean z, over(g) mcompare(`m') effects
    display "@ t_`m' " r(mctitle_vs) " " r(mcmethod_vs) " " el(r(k_groups), 1, 1)
    forvalues j = 1/6 {
        display "@ `m'`j' " %24.17g el(r(table_vs), 3, `j') " " %24.17g el(r(table_vs), 4, `j') " " %24.17g el(r(table_vs), 5, `j') " " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(r(table_vs), 8, `j')
    }
}
pwmean z, over(g) mcompare(bon) level(99)
display "@ bon99 " %24.17g el(r(table_vs), 5, 2) " " %24.17g el(r(table_vs), 8, 2) " " %24.17g el(r(table), 8, 2)
pwmean z, over(g) mcompare(sch) pveffects
display "@ sch " %24.17g el(r(table_vs), 4, 2) " " %24.17g el(r(table_vs), 8, 2)
pwmean z, over(g) level(95.5) cieffects
display "@ l955 " r(level) " " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 6, 1)
pwmean z, over(g) level(10) ci
display "@ l10 " r(level) " " %24.17g el(r(table_vs), 5, 1)
* level() reaches r(table) only with cimeans and r(table_vs) only with a table of contrasts' intervals
pwmean z, over(g) level(90) cimeans
display "@ lc " r(level) " " %24.17g el(r(table), 5, 2) " " %24.17g el(r(table), 8, 2) " " %24.17g el(r(table_vs), 5, 2) " " %24.17g el(r(table_vs), 8, 2)
pwmean z, over(g) level(90) groups
display "@ lg " r(level) " " el(r(k_groups), 1, 1) " " %24.17g el(r(table), 8, 2) " " %24.17g el(r(table_vs), 8, 2)
pwmean z, over(g) level(90) cimeans groups
display "@ lcg " el(r(k_groups), 1, 1) " " %24.17g el(r(table), 8, 2) " " %24.17g el(r(table_vs), 8, 2)
pwmean z, over(g) level(90) pveffects
display "@ lp " %24.17g el(r(table), 8, 2) " " %24.17g el(r(table_vs), 8, 2)
* groups: how many letters
pwmean z, over(g) groups
display "@ k1 " el(r(k_groups), 1, 1)
generate double zz = z + 40 * (g == 3) - 30 * (g == 1) + 80 * (g == 4)
pwmean zz, over(g) groups
display "@ k2 " el(r(k_groups), 1, 1)
pwmean z, over(g) mcompare(scheffe) groups
display "@ k3 " el(r(k_groups), 1, 1)
pwmean z, over(g) mcompare(bonferroni) groups level(90)
display "@ k4 " el(r(k_groups), 1, 1)
* unbalanced groups, if and in
pwmean z if _n > 5, over(g) effects
display "@ ub " e(balanced) " " e(df_r) " " %24.17g el(r(table), 2, 1) " " %24.17g el(r(table), 2, 2)
display "@ ub2 " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 6, 6) " " %24.17g el(r(table_vs), 4, 1)
pwmean z in 3/38, over(g) effects
display "@ in " e(balanced) " " e(df_r) " " %24.17g el(e(V), 1, 1) " " %24.17g el(e(V), 2, 2) " " %24.17g el(e(V_vs), 3, 3) " " %24.17g el(e(V_vs), 3, 6)
pwmean z if x1 > 5 in 3/38, over(g) effects
display "@ ifin " e(df_r) " " %24.17g el(r(table_vs), 2, 1) " " %24.17g el(r(table_vs), 2, 6) " " %24.17g el(r(table_vs), 4, 3)
replace g = . in 1/3
pwmean z, over(g)
display "@ miss " e(df_r) " " %24.17g el(e(b), 1, 1) " " %24.17g el(e(b), 1, 4)
replace g = 1 + mod(_n, 4)
* several over() variables
pwmean y in 1/30, over(g w)
display "@ o1 " e(balanced) " " e(df_r) " " colsof(e(b)) " " colsof(e(b_vs)) " " el(r(k_groups), 1, 1) " " e(over)
forvalues j = 1/12 {
    display "@ om`j' " %24.17g el(e(b), 1, `j') " " %24.17g el(e(V), `j', `j') " " %24.17g el(r(table), 2, `j') " " %24.17g el(r(table), 5, `j') " " %24.17g el(r(table), 6, `j')
}
foreach j in 1 2 3 11 12 13 40 66 {
    display "@ ou`j' " %24.17g el(e(b_vs), 1, `j') " " %24.17g el(r(table_vs), 2, `j') " " %24.17g el(r(table_vs), 4, `j') " " %24.17g el(r(table_vs), 5, `j')
    display "@ ow`j' " %24.17g el(r(table_vs), 6, `j') " " %24.17g el(e(V_vs), `j', `j')
}
pwmean y in 1/30, over(g w) mcompare(bonferroni) effects
display "@ ob " %24.17g el(r(table_vs), 4, 1) " " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 8, 1) " " %24.17g el(r(table_vs), 4, 66)
pwmean y in 1/30, over(w g) mcompare(scheffe) effects
display "@ os " %24.17g el(r(table_vs), 4, 1) " " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 8, 1) " " %24.17g el(e(b), 1, 2)
* only one comparison: the method is ignored
pwmean z if g <= 2, over(g) mcompare(scheffe) effects
display "@ two " r(mcmethod_vs) " " r(mctitle_vs) " " colsof(r(table_vs)) " " e(df_r) " " %24.17g el(r(table_vs), 4, 1) " " %24.17g el(r(table_vs), 5, 1)
pwmean z if g <= 2, over(g) mcompare(sidak) groups
display "@ two2 " r(mcmethod_vs) " " el(r(k_groups), 1, 1)
* a group with no value of the variable drops out
replace z = . if g == 4
pwmean z, over(g) effects
display "@ drop " colsof(e(b)) " " colsof(e(b_vs)) " " e(df_r) " " %24.17g el(r(table_vs), 4, 3)
replace z = y + 4 * (g == 3) + 2.2 * (g == 2) - 3 * (g == 4)
* the results come after a model too
regress z x1
pwmean z, over(g)
display "@ after " e(cmd) " " e(df_r)
* errors
capture pwmean y
display "@ rc1 " _rc
capture pwmean y, over(g) mcompare(holm)
display "@ rc2 " _rc
capture pwmean y, over(nosuch)
display "@ rc3 " _rc
capture pwmean nosuch, over(g)
display "@ rc4 " _rc
generate str3 s = "a" + string(g)
capture pwmean y, over(s)
display "@ rc5 " _rc
capture pwmean s, over(g)
display "@ rc6 " _rc
generate int negg = g - 3
capture pwmean y, over(negg)
display "@ rc7 " _rc
capture pwmean y, over(x1)
display "@ rc8 " _rc
capture pwmean y x1, over(g)
display "@ rc9 " _rc
capture pwmean y if 0, over(g)
display "@ rc10 " _rc
capture pwmean y if g == 2, over(g)
display "@ rc11 " _rc
capture pwmean y [aweight = w], over(g)
display "@ rc12 " _rc
capture pwmean y, over(g) level(5)
display "@ rc13 " _rc
capture pwmean y, over(g) level(100)
display "@ rc14 " _rc
capture pwmean y, over(g) mcompare(tu)
display "@ rc15 " _rc
capture pwmean y, over(g) mcompare(tuk)
display "@ rc16 " _rc
capture pwmean y, over(g) mcompare(noadjust) level(10)
display "@ rc17 " _rc
