* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in mvtest.recorded
* commands: mvtest
* (Hotelling's T2 that means are equal or zero, the four MANOVA statistics by(), the LR test allowing for heterogeneity, Box's M, the adjusted LR tests of a diagonal, spherical or compound-symmetric covariance matrix, Lawley's and Jennrich's tests of correlations, Doornik-Hansen, Henze-Zirkler and Mardia's tests of normality, missing by() values as a group)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 72
generate double x1 = mod(_n * 37, 101) / 7
generate double x2 = mod(_n * 13, 17) / 3 + x1 / 4
generate double x3 = mod(_n * 11, 23) / 4 - x2 / 5
generate double x4 = mod(_n * 7, 19) / 2 + x3
generate g = 1 + mod(_n, 3)
generate k = 1 + mod(_n, 4)
replace g = . if mod(_n, 11) == 0
replace x4 = . in 3
quietly mvtest means x1 x2 x3
display "@ meq " %24.17g r(T2) " " %24.17g r(F) " " %24.17g r(p_F) " " r(df_m) " " r(df_r)
quietly mvtest means x1 x2 x3 x4, zero
display "@ mzero " %24.17g r(T2) " " %24.17g r(F) " " %24.17g r(p_F) " " r(df_r)
quietly mvtest means x1 x2 x3, by(k)
display "@ manova " %24.17g r(F) " " %24.17g r(df_r) " " %24.17g r(p_F)
display "@ stat_m " %24.17g el(r(stat_m), 2, 1) " " %24.17g el(r(stat_m), 3, 2) " " %24.17g el(r(stat_m), 4, 5) " " el(r(stat_m), 1, 6)
quietly mvtest means x1 x2 x3 x4, by(g) missing
display "@ manova_miss " %24.17g r(F) " " %24.17g r(df_r) " " %24.17g r(p_F)
quietly mvtest means x1 x2 x3, by(g) lr
display "@ lr " %24.17g r(chi2) " " %24.17g r(p_chi2) " " r(df)
quietly mvtest covariances x1 x2 x3, by(k)
display "@ box " %24.17g r(chi2) " " %24.17g r(p_chi2) " " %24.17g r(F_Box) " " %24.17g r(df_r_Box) " " %24.17g r(p_F_Box)
quietly mvtest covariances x1 x2 x3 x4, diagonal
display "@ diag " %24.17g r(chi2) " " %24.17g r(p_chi2)
quietly mvtest covariances x1 x2 x3, spherical
display "@ sph " %24.17g r(chi2) " " %24.17g r(p_chi2)
quietly mvtest covariances x1 x2 x3 x4, compound
display "@ comp " %24.17g r(chi2) " " %24.17g r(p_chi2)
quietly mvtest correlations x1 x2 x3 x4
display "@ lawley " %24.17g r(chi2) " " %24.17g r(p_chi2) " " r(df)
quietly mvtest correlations x1 x2 x3, by(g)
display "@ jennrich " %24.17g r(chi2) " " %24.17g r(p_chi2) " " r(df)
quietly mvtest normality x1 x2 x3 x4, stats(all)
display "@ dh " %24.17g r(chi2_dh) " " %24.17g r(p_dh)
display "@ hz " %24.17g r(hz) " " %24.17g r(z_hz) " " %24.17g r(p_hz) " " %24.17g r(E_hz) " " %24.17g r(V_hz)
display "@ mardia " %24.17g r(mskew) " " %24.17g r(chi2_mskew) " " %24.17g r(p_mskew) " " %24.17g r(mkurt) " " %24.17g r(z_mkurt) " " %24.17g r(p_mkurt)
display "@ u_dh " %24.17g el(r(U_dh), 1, 1) " " %24.17g el(r(U_dh), 2, 2) " " %24.17g el(r(U_dh), 3, 3) " " %24.17g el(r(U_dh), 4, 4)
quietly mvtest normality x1 x2 x3, bivariate univariate
display "@ biv " %24.17g el(r(Btest), 1, 1) " " %24.17g el(r(Btest), 3, 3)
display "@ univ " %24.17g el(r(Utest), 1, 1) " " %24.17g el(r(Utest), 2, 3) " " %24.17g el(r(Utest), 3, 4)
capture mvtest bogus x1 x2
display "@ rc_sub " _rc
capture mvtest means x1 x2 x3, by(g) zero
display "@ rc_opt " _rc
generate double x5 = 2 * x1 - x2
capture mvtest covariances x1 x2 x5
display "@ rc_sing " _rc
