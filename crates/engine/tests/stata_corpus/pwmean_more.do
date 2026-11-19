* RCraft's Stata corpus (P2C.12c, batch C, second round): made-up data; Stata 19's results are in pwmean_more.recorded
* commands: pwmean
* (pwmean, the parts the first round refused: a variable twice in over() and the i., ib#. and c. prefixes, an empty mcompare(), an option given twice and the level() rule for it, a bare pwmean that shows the last results again with other options, an adjusted method with cimeans and other tables (which level each matrix holds), and zero or one residual degrees of freedom)
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
* over(): a variable twice, the i., ib#. and c. prefixes, an empty mcompare()
pwmean z, over(g g)
display "@ og1 " e(over) " " colsof(e(b)) " " colsof(e(b_vs)) " " %24.17g el(r(table_vs), 3, 2) " " %24.17g el(r(table_vs), 5, 2)
pwmean z, over(i.g)
display "@ og2 " e(over) " " colsof(e(b)) " " colsof(e(b_vs)) " " %24.17g el(r(table_vs), 3, 2) " " %24.17g el(r(table_vs), 5, 2)
pwmean z, over(ib2.g)
display "@ og3 " e(over) " " colsof(e(b)) " " colsof(e(b_vs)) " " %24.17g el(r(table_vs), 3, 2) " " %24.17g el(r(table_vs), 5, 2)
pwmean z, over(c.g)
display "@ og4 " e(over) " " colsof(e(b)) " " colsof(e(b_vs)) " " %24.17g el(r(table_vs), 3, 2) " " %24.17g el(r(table_vs), 5, 2)
pwmean z, over(g) mcompare()
display "@ og5 " r(mcmethod_vs) " " r(mctitle_vs) " " %24.17g el(r(table_vs), 3, 2) " " %24.17g el(r(table_vs), 5, 2)
pwmean z, over(i.g i.w)
display "@ og6 " e(over) " " colsof(e(b)) " " colsof(e(b_vs)) " " %24.17g el(r(table_vs), 3, 5) " " %24.17g el(r(table_vs), 5, 5)
capture pwmean z, over(g#w)
display "@ rc_hash " _rc
capture pwmean z i.g, over(g)
display "@ rc_fv " _rc
* an option given twice is r(198); a second level() only when it says 95
capture pwmean z, over(g) over(w)
display "@ rc_t1 " _rc
capture pwmean z, over(g) mcompare(bonferroni) mcompare(tukey)
display "@ rc_t2 " _rc
capture pwmean z, over(g) effects eff
display "@ rc_t3 " _rc
capture pwmean z, over(g) cieffects ci
display "@ rc_t4 " _rc
capture pwmean z, over(g) sort sort
display "@ rc_t5 " _rc
capture pwmean z, over(g) cimeans cimeans
display "@ rc_t6 " _rc
capture pwmean z, over(g) groups groups
display "@ rc_t7 " _rc
capture pwmean z, over(g) pveffects pveffects
display "@ rc_t8 " _rc
capture pwmean z, over(g) level(90) level(80)
display "@ rc_t9 " _rc
capture pwmean z, over(g) level(95) level(90)
display "@ rc_t10 " _rc
capture pwmean z, over(g) level(90) level(90)
display "@ rc_t11 " _rc
capture pwmean z, over(g) level(90) level(95)
display "@ rc_t12 " _rc " " r(level)
capture pwmean z, over(g) level(95) level(95)
display "@ rc_t13 " _rc " " r(level)
capture pwmean z, over(g) level(95.5) level(95)
display "@ rc_t14 " _rc " " r(level)
* a bare pwmean shows the last results again, with new options
quietly pwmean z, over(g) level(90) mcompare(bonferroni) cimeans
pwmean
display "@ rp1 " r(level) " " r(mcmethod_vs) " " e(cmdline)
display "@ rp2 " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table), 5, 1) " " e(df_r) " " colsof(r(table))
pwmean, mcompare(sidak) effects level(90)
display "@ rp3 " r(mcmethod_vs) " " %24.17g el(r(table_vs), 4, 2) " " %24.17g el(r(table_vs), 5, 2) " " r(level)
pwmean, groups level(80)
display "@ rp4 " el(r(k_groups), 1, 1) " " r(level)
pwmean, cimeans sort
display "@ rp5 " %24.17g el(r(table), 6, 3) " " r(mcmethod_vs)
capture pwmean, over(g)
display "@ rc_rp1 " _rc
capture pwmean if g < 3
display "@ rc_rp2 " _rc
capture pwmean in 1/5
display "@ rc_rp3 " _rc
capture pwmean [aw = x1]
display "@ rc_rp4 " _rc
capture pwmean z
display "@ rc_rp5 " _rc
quietly pwmean z, over(g)
preserve
clear
pwmean
display "@ rp6 " r(level) " " %24.17g el(r(table_vs), 3, 1)
restore
quietly regress z g
capture pwmean
display "@ rc_rp6 " _rc
capture pwmean, over(g)
display "@ rc_rp7 " _rc
* an adjusted method with cimeans and other tables: the level each matrix holds
pwmean z, over(g) mcompare(bonferroni) cimeans cieffects level(90)
display "@ ac1 " %24.17g el(r(table), 5, 1) " " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 8, 1) " " el(r(k_groups), 1, 1) " " r(level)
pwmean z, over(g) mcompare(scheffe) cimeans groups level(90)
display "@ ac2 " %24.17g el(r(table), 5, 1) " " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 8, 1) " " el(r(k_groups), 1, 1)
pwmean z, over(g) mcompare(sidak) cimeans pveffects level(80)
display "@ ac3 " %24.17g el(r(table), 5, 1) " " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 8, 1) " " %24.17g el(r(table_vs), 4, 2)
pwmean z, over(g) mcompare(bonferroni) cimeans groups pveffects cieffects effects level(80)
display "@ ac4 " %24.17g el(r(table), 5, 1) " " %24.17g el(r(table_vs), 5, 1) " " %24.17g el(r(table_vs), 8, 1) " " el(r(k_groups), 1, 1)
* zero residual degrees of freedom: missing standard errors, intervals and p-values
pwmean z in 1/4, over(g)
display "@ df0a " e(df_r) " " e(balanced) " " el(r(table), 1, 1) " " el(r(table), 2, 1) " " el(r(table), 7, 1) " " el(r(table_vs), 2, 1) " " el(r(table_vs), 4, 3) " " el(r(table_vs), 8, 2) " " el(e(V), 1, 1) " " el(e(V_vs), 2, 2)
pwmean z in 1/4, over(g) mcompare(tukey) groups
display "@ df0b " el(r(k_groups), 1, 1) " " el(r(table_vs), 8, 1) " " r(mcmethod_vs)
pwmean z in 1/4, over(g) mcompare(bonferroni) cimeans effects
display "@ df0c " el(r(table), 3, 2) " " el(r(table_vs), 5, 4) " " el(r(table_vs), 6, 4)
* one residual degree of freedom: Tukey has no value, the other methods do
pwmean z if inlist(_n, 1, 2, 4, 8), over(g) mcompare(tukey)
display "@ df1a " e(df_r) " " %24.17g el(r(table_vs), 3, 1) " " el(r(table_vs), 4, 1) " " el(r(table_vs), 5, 1) " " el(r(table_vs), 8, 1)
pwmean z if inlist(_n, 1, 2, 4, 8), over(g) mcompare(bonferroni)
display "@ df1b " %24.17g el(r(table_vs), 4, 2) " " %24.17g el(r(table_vs), 5, 2) " " %24.17g el(r(table_vs), 8, 2)
pwmean z if inlist(_n, 1, 2, 4, 8), over(g) mcompare(sidak) cimeans
display "@ df1c " %24.17g el(r(table), 5, 1) " " %24.17g el(r(table), 8, 1) " " %24.17g el(r(table_vs), 4, 1)
pwmean z if inlist(_n, 1, 2, 4, 8), over(g) mcompare(tukey) cimeans groups
display "@ df1d " %24.17g el(r(table), 5, 1) " " el(r(k_groups), 1, 1)
pwmean z if inlist(_n, 1, 2, 4, 8), over(g) mcompare(scheffe) effects
display "@ df1e " %24.17g el(r(table_vs), 4, 3) " " %24.17g el(r(table_vs), 6, 3) " " %24.17g el(r(table_vs), 8, 3)
