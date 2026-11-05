* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in arpanel.recorded
* commands: input newey prais xtregar
* (xtregar fe and re with AR(1) disturbances: each rhotype(), rhof(), twostep and lbi, on a balanced panel built row by row with random draws (an AR(1) series through replace with L. and with [_n-1] under by) and on a small panel typed in with input; prais (Prais-Winsten and Cochrane-Orcutt, twostep, rhotype(dw), vce(robust)) and newey with lag(0) to lag(3), on float variables replaced row by row)
* tolerance: 1e-7
* (re keeps its transformed data in single precision; StatCraft's decoding agrees to a few parts in 1e8 there, fe to 1e-13)
version 19
set linesize 255
clear
set seed 324
set obs 30
generate int id = _n
generate double u = rnormal()*.8
expand 6
bysort id: generate int t = _n
xtset id t
generate double x1 = round(rnormal(), .01)
generate double x2 = round(runiform()*2, .01)
generate double e = rnormal()
replace e = .5*L.e + rnormal() if t > 1
generate double y = round(1 + .5*x1 - .3*x2 + u + e, .01)
display "@ data " %21x e[2] " " %21x y[6] " " %21x e[180]
quietly xtregar y x1 x2, fe
display "@ fe " e(rho_ar) " " _b[x1] " " _se[x1] " " _b[x2] " " _b[_cons] " " _se[_cons] " " e(sigma_u) " " e(sigma_e) " " e(rho_fov)
display "@ fe stats " e(N) " " e(N_g) " " e(df_r) " " e(F) " " e(F_f) " " e(r2_w) " " e(r2_b) " " e(r2_o) " " e(corr) " " e(ll) " " e(rmse)
foreach r in regress freg tscorr theil nagar onestep {
    quietly xtregar y x1 x2, fe rhotype(`r')
    display "@ fe `r' " e(rho_ar) " " _b[x1]
}
quietly xtregar y x1 x2, fe twostep
display "@ fe twostep " e(rho_ar) " " _b[x1]
quietly xtregar y x1 x2, fe rhof(.4)
display "@ fe rhof " e(rho_ar) " " _b[x1] " " _se[x2]
quietly xtregar y x1 x2, fe lbi
display "@ fe lbi " e(d1) " " e(LBI) " " e(N_LBI)
quietly xtregar y x1 x2, re
display "@ re " e(rho_ar) " " _b[x1] " " _se[x1] " " _b[_cons] " " e(sigma_u) " " e(sigma_e) " " e(chi2) " " e(thta_min) " " e(r2_w) " " e(r2_o)
quietly xtregar y x1 x2, re rhotype(regress) lbi
display "@ re regress lbi " e(rho_ar) " " _b[x2] " " e(LBI)
generate double eps = rnormal()
generate double v = eps
bysort id (t): replace v = eps + .4*v[_n-1] if t > 1
display "@ subscript " %21x v[2] " " %21x v[12]
clear
input int id int t double y double x
1 1 1.0 0.5
1 2 2.5 1.5
1 3 2.0 3.0
1 4 4.5 2.0
1 5 3.0 4.5
2 1 0.5 1.0
2 2 1.0 0.2
2 3 2.2 2.5
2 4 1.8 1.1
2 5 3.9 3.3
3 1 2.0 2.0
3 2 2.4 1.0
3 3 3.1 2.2
3 4 2.9 3.0
3 5 4.4 4.1
end
xtset id t
quietly xtregar y x, fe
display "@ typed fe " e(rho_ar) " " _b[x] " " _b[_cons] " " e(N)
quietly xtregar y x, re
display "@ typed re " e(rho_ar) " " _b[x] " " e(N_g)
clear
set obs 40
generate t = _n
tsset t
generate x1 = sin(_n / 3) * 5 + mod(_n * 7, 11) / 3
generate x2 = mod(_n * 13, 17) / 4
generate e = mod(_n * 29, 23) / 7 - 1.5
replace e = e + 0.6 * e[_n - 1] if _n > 1
generate y = 1 + 0.8 * x1 - 0.5 * x2 + e
quietly prais y x1 x2
display "@ prais " e(rho) " " _b[x1] " " _se[x1] " " _b[_cons] " " e(F) " " e(r2) " " e(dw) " " e(dw_0) " " e(ic)
quietly prais y x1 x2, corc
display "@ corc " e(rho) " " _b[x1] " " e(N) " " e(rss)
quietly prais y x1 x2, twostep
display "@ twostep " e(rho) " " _b[x2]
quietly prais y x1 x2, rhotype(dw)
display "@ dw " e(rho) " " _b[x1]
quietly prais y x1 x2, vce(robust)
display "@ robust " _se[x1] " " _se[_cons] " " e(F)
forvalues l = 0/3 {
    quietly newey y x1 x2, lag(`l')
    display "@ newey `l' " _se[x1] " " _se[x2] " " _se[_cons] " " e(F)
}
quietly newey y x1 x2, lag(2) noconstant
display "@ newey nocons " _b[x1] " " _se[x1] " " e(df_m)
