* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in rreg.recorded
* commands: rreg
* (rreg: Huber then biweight iterations, the pseudovalues' variance, e(F) e(r2) e(r2_a) e(mss) e(beta), and the rows Cook's distance leaves out)
* tolerance: 1e-9
version 19
clear
set obs 200
generate double x1 = sin(_n * 1.3) * 3
generate double x2 = cos(_n * 0.7) + (mod(_n, 5) == 0)
generate double e = sin(_n * 7.1) + 0.5 * sin(_n * 3.3)
generate double y1 = 1 + 0.5 * x1 + e
replace y1 = y1 + 8 in 5
replace y1 = y1 - 6 in 17
generate double y2 = 2 - 0.3 * x1 + 0.8 * x2 + e * (1 + abs(x2))
replace y2 = y2 + 12 in 40/44
rreg y1 x1 in 1/60
display "@ a " e(rmse) " " e(F) " " e(r2) " " e(mss) " " e(rss)
display "@ a2 " _b[x1] " " _se[x1] " " _b[_cons] " " _se[_cons]
quietly rreg y1 x1 in 1/25
display "@ e " e(N) " " _b[x1] " " _se[x1] " " e(rmse)
rreg y2 x1 x2 in 1/80
display "@ b " e(rmse) " " e(F) " " e(r2_a) " " e(N) " " e(df_r) " " _b[x2] " " _se[x2] " " _se[_cons]
matrix B = e(beta)
display "@ c " B[1,1] " " B[1,2]
clear
set obs 40
generate double x = sin(_n * 1.3) * 3
generate double y = 1 + 0.5 * x + sin(_n * 7.1)
replace x = 30 in 3
replace y = -40 in 3
replace y = . in 7
rreg y x
display "@ d " e(N) " " _b[x] " " _se[x] " " e(df_r) " " e(F)
