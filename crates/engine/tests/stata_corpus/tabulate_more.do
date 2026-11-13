* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in tabulate_more.recorded
* commands: tabulate summarize
* (tabulate's matcell(), matrow() and matcol(), one way and two, with frequency and analytic weights and if; lrchi2 and its r(); summarize, mean (meanonly))
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 82
set obs 120
generate g = 1 + floor(3*runiform())
generate b = runiform() < .4
generate y = 1 + .3*g + rnormal()
replace y = . in 1/7
generate wt = 1 + mod(_n, 4)
tabulate g b, matcell(F) matrow(R) matcol(C)
display "@ cell " F[1,1] " " F[2,2] " " F[3,1] " " rowsof(F) " " colsof(F) " " R[3,1] " " C[1,2]
tabulate g, matcell(F1) matrow(R1)
display "@ one " F1[1,1] " " F1[3,1] " " R1[2,1] " " rowsof(F1)
tabulate g b if y < ., matcell(F2)
display "@ if " F2[1,1] " " F2[3,2]
tabulate g b [aweight = wt], matcell(A)
display "@ aw " %24.17g A[1,1] " " %24.17g A[2,2]
tabulate g b [fweight = wt], matcell(W)
display "@ fw " W[1,1] " " W[3,2]
tabulate g b, lrchi2
display "@ lr " %24.17g r(chi2_lr) " " %24.17g r(p_lr) " " r(N)
tabulate g b, chi2 lrchi2
display "@ lr2 " %24.17g r(chi2) " " %24.17g r(chi2_lr)
tabulate g b [fweight = wt], lrchi2
display "@ lrfw " %24.17g r(chi2_lr) " " %24.17g r(p_lr)
summarize y, mean
display "@ mean " %24.17g r(mean) " " r(N) " " %24.17g r(sum)
