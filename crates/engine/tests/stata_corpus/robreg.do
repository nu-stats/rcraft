* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in robreg.recorded
* commands: robreg
* (robreg m, the SSC package's M regression, as the replication packages run it: its coefficients, robust standard errors, scale and R-squareds; with if and efficiency(); the empty model starting at the median; predict's xb, residuals and weights; indicators, where the LAD start is not unique)
* tolerance: 1e-6
version 19
clear
set obs 40
generate x = mod(_n * 7, 13) / 2
generate z = mod(_n * 5, 9)
generate g = mod(_n, 3) + 1
generate y = 1 + 0.8 * x - 0.3 * z + mod(_n * 11, 7) / 3 + (g == 2)
replace y = y + 15 in 3
replace y = y - 12 in 17
generate s = cond(mod(_n, 4) == 0, "1", "0")
robreg m y x z
display "@ b " _b[x] " " _b[z] " " _b[_cons]
display "@ se " _se[x] " " _se[z] " " _se[_cons]
display "@ e " e(N) " " e(df_m) " " e(df_r) " " e(scale) " " e(scale0) " " e(k) " " e(efficiency)
display "@ fit " e(chi2) " " e(p) " " e(r2_w) " " e(r2_p) " " e(rank)
predict p
quietly summarize p
display "@ xb " r(N) " " r(mean) " " r(min) " " r(max)
predict r, residuals
quietly summarize r
display "@ residuals " r(N) " " r(mean) " " r(min) " " r(max)
predict w, weights
quietly summarize w
display "@ weights " r(N) " " r(mean) " " r(min) " " r(max)
robreg m y x z if s == "0"
display "@ if " e(N) " " _b[x] " " _se[x] " " e(scale)
predict pif if s == "0"
quietly summarize pif
display "@ predict_if " r(N) " " r(mean)
robreg m y x z, efficiency(90)
display "@ efficiency " _b[x] " " _se[x] " " e(k) " " e(scale)
robreg m y x
display "@ one " _b[x] " " _b[_cons] " " _se[x] " " e(r2_p)
* indicators, where the LAD starting fit is not unique: robreg starts inside
* the LAD solutions, as an interior-point solver does
robreg m y x i.g
display "@ ig " _b[x] " " _b[2.g] " " _b[3.g] " " _b[_cons] " " e(scale)
generate byte d = mod(_n, 3) == 0
robreg m y d
display "@ dummy " _b[d] " " _b[_cons] " " _se[d] " " e(scale)
