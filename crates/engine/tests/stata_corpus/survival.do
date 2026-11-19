* RCraft's Stata corpus (P2C.12, batch K): made-up data; Stata 19's results are in survival.recorded
* commands: stset stdescribe stsum sts stptime stir ltable
* (stdescribe: one record a subject, several records with gaps; stsum: by(), r(); sts list/generate/test: by(), at(), failure, na, level(), wilcoxon, tware, peto, fh(), strata(); stptime: by(), at(), trim, per(), level(); stir: strata(), level(); ltable: by(), intervals(), failure, hazard, noadjust, test, fweight)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 60
generate int id = _n
generate byte group = 1 + (_n > 30)
generate double x = round(sin(_n * 12.9898) * 2, .01)
generate byte s = mod(_n, 2)
generate byte exposed = _n > 30
generate int t = 1 + floor(abs(sin(_n * 3.7)) * 24 * (1 + (group == 1)))
generate byte died = abs(cos(_n * 5.1)) < 0.8
generate byte w = 1 + mod(_n, 3)
stset t, failure(died)
stdescribe
display "@ sd1 " r(N_sub) " " r(N_total) " " r(N_fail) " " r(N_gap) " " r(gap) " " r(gap_mean) " " %24.17g r(tr) " " %24.17g r(tr_mean) " " r(tr_min) " " r(tr_med) " " r(tr_max)
display "@ sd2 " %24.17g r(t1_mean) " " r(t1_min) " " r(t1_med) " " r(t1_max) " " r(t0_mean) " " r(t0_max) " " %24.17g r(f_mean) " " r(f_med) " " r(N_mean) " " r(N_max)
stsum
display "@ ss1 " r(N_sub) " " %24.17g r(ir) " " %24.17g r(risk) " " r(p25) " " r(p50) " " r(p75)
stsum, by(group)
display "@ ss2 " r(N_sub) " " %24.17g r(ir) " " r(p25) " " r(p50) " " r(p75)
sts list
sts list, by(group)
sts list, at(5 10 15)
sts list, failure
sts list, na level(90)
sts generate s1 = s
sts generate lb1 = lb(s) ub1 = ub(s) se1 = se(s)
sts generate na1 = na
sts generate nlb1 = lb(na) nub1 = ub(na) nse1 = se(na)
sts generate h1 = h
sts generate n1 = n d1 = d
sts generate s2 = s, by(group)
sts generate lb2 = lb(s), by(group) level(90)
foreach v in s1 lb1 ub1 se1 na1 nlb1 nub1 nse1 h1 n1 d1 s2 lb2 {
    quietly summarize `v'
    display "@ sg_`v' " r(N) " " %24.17g r(mean) " " %24.17g r(min) " " %24.17g r(max)
    display "@ sgv_`v' " %24.17g `v'[1] " " %24.17g `v'[17] " " %24.17g `v'[44] " " %24.17g `v'[60]
}
sts test group
display "@ st1 " %24.17g r(chi2) " " r(df)
sts test group, wilcoxon
display "@ st2 " %24.17g r(chi2) " " r(df)
sts test group, tware
display "@ st3 " %24.17g r(chi2) " " r(df)
sts test group, peto
display "@ st4 " %24.17g r(chi2) " " r(df)
sts test group, fh(1 0)
display "@ st5 " %24.17g r(chi2) " " r(df)
sts test group, fh(0 1)
display "@ st6 " %24.17g r(chi2) " " r(df)
sts test group, logrank strata(s)
display "@ st7 " %24.17g r(chi2) " " r(df)
sts test s
display "@ st8 " %24.17g r(chi2) " " r(df)
generate byte g3 = 1 + mod(_n, 3)
sts test g3
display "@ st9 " %24.17g r(chi2) " " r(df)
sts test g3, wilcoxon
display "@ st10 " %24.17g r(chi2) " " r(df)
* ltable: only r() after test is stored; the tables are not
ltable t died
ltable t died, by(group) test
display "@ lt1 " %24.17g r(chi2) " " r(df)
ltable t died, failure
ltable t died, hazard
ltable t died, intervals(5)
ltable t died, intervals(0 2 5 10 20)
ltable t died, noadjust
ltable t died, level(90)
ltable t died [fweight=w]
ltable t died [fweight=w], by(group) test
display "@ lt2 " %24.17g r(chi2) " " r(df)
ltable t died if group == 1, failure intervals(3)
ltable t, by(group) test
display "@ lt3 " %24.17g r(chi2) " " r(df)
* id(): stptime and stir
stset t, failure(died) id(id)
stptime
display "@ pt1 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate) " " %24.17g r(lb) " " %24.17g r(ub)
stptime, by(group)
display "@ pt2 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate) " " %24.17g r(lb) " " %24.17g r(ub)
stptime, per(1000)
display "@ pt3 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate) " " %24.17g r(lb) " " %24.17g r(ub)
stptime, at(0 2 5 10 20)
display "@ pt4 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate) " " %24.17g r(lb) " " %24.17g r(ub)
stptime, at(0 2 5 10 20) by(group) trim
display "@ pt5 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate) " " %24.17g r(lb) " " %24.17g r(ub)
stptime, dd(2) level(90)
display "@ pt6 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate) " " %24.17g r(lb) " " %24.17g r(ub)
stptime if x > 0
display "@ pt7 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate) " " %24.17g r(lb) " " %24.17g r(ub)
stir exposed
display "@ ir1 " %24.17g r(ird) " " %24.17g r(lb_ird) " " %24.17g r(ub_ird) " " %24.17g r(irr) " " %24.17g r(lb_irr) " " %24.17g r(ub_irr)
display "@ ir2 " %24.17g r(afe) " " %24.17g r(lb_afe) " " %24.17g r(ub_afe) " " %24.17g r(afp) " " %24.17g r(p_lower_midp) " " %24.17g r(p_upper_midp) " " %24.17g r(p_twosided_midp)
display "@ ir3 " %24.17g r(p_lower_exact) " " %24.17g r(p_upper_exact) " " %24.17g r(p_twosided_exact)
stir exposed, level(90)
display "@ ir4 " %24.17g r(ird) " " %24.17g r(lb_ird) " " %24.17g r(ub_ird) " " %24.17g r(irr) " " %24.17g r(lb_irr) " " %24.17g r(ub_irr)
stir exposed, strata(s)
display "@ ir5 " %24.17g r(irr) " " %24.17g r(lb_irr) " " %24.17g r(ub_irr) " " %24.17g r(crude) " " %24.17g r(pooled) " " %24.17g r(chi2_mh) " " r(df)
stir group
display "@ ir6 " %24.17g r(irr) " " %24.17g r(lb_irr) " " %24.17g r(ub_irr)
stset t, failure(died) id(id) scale(10)
stptime
display "@ pt8 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate) " " %24.17g r(lb) " " %24.17g r(ub)
stir exposed
display "@ ir7 " %24.17g r(irr) " " %24.17g r(lb_irr) " " %24.17g r(ub_irr)
* several records a subject, with gaps
clear
set obs 40
generate int id = ceil(_n / 2)
generate byte rec = 2 - mod(_n, 2)
generate double a = 1 + floor(abs(sin(id * 2.3)) * 9)
generate double t0 = cond(rec == 1, 0, a + floor(abs(sin(id * 5.7)) * 3))
generate double t1 = cond(rec == 1, a, t0 + 1 + floor(abs(cos(id * 4.1)) * 8))
generate byte fail = rec == 2 & abs(sin(id * 1.9)) < 0.6
generate byte g = 1 + mod(id, 2)
stset t1, failure(fail) id(id) time0(t0)
stdescribe
display "@ sm1 " r(N_sub) " " r(N_total) " " r(N_fail) " " r(N_gap) " " %24.17g r(gap) " " %24.17g r(gap_mean) " " r(gap_min) " " r(gap_med) " " r(gap_max)
display "@ sm2 " %24.17g r(tr) " " %24.17g r(tr_mean) " " r(tr_min) " " r(tr_med) " " r(tr_max) " " r(N_mean) " " r(N_max) " " r(t0_mean) " " r(t0_max) " " r(t1_mean) " " r(t1_max)
stptime
display "@ sm3 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate) " " %24.17g r(lb) " " %24.17g r(ub)
stptime, by(g)
display "@ sm4 " %24.17g r(ptime) " " r(failures) " " %24.17g r(rate)
stsum
display "@ sm5 " r(N_sub) " " %24.17g r(ir) " " %24.17g r(risk)
