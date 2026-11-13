* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in sdtest.recorded
* commands: sdtest sdtesti estimates
* (sdtest of one standard deviation, of two variables (each over its own rows) and of two groups with by(), level(); sdtesti of one and two groups; estimates stats of the active results and of stored ones, r(S))
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 95
set obs 100
generate g = runiform() < .5
label define gl 0 "control" 1 "treated"
label values g gl
generate y = rnormal() + g
generate x = rnormal()
replace x = . in 1/5
replace y = . in 90/100
sdtest y == 1
display "@ one " %24.17g r(chi2) " " r(df) " " %24.17g r(p) " " %24.17g r(p_l) " " %24.17g r(sd) " " r(N)
sdtest y == 2.25, level(90)
display "@ one2 " %24.17g r(chi2) " " %24.17g r(p_u)
sdtest y == x
display "@ vars " %24.17g r(F) " " r(df_1) " " r(df_2) " " %24.17g r(p) " " %24.17g r(sd) " " %24.17g r(sd_2) " " r(N)
sdtest y, by(g)
display "@ by " %24.17g r(F) " " r(df_1) " " r(df_2) " " %24.17g r(p_l) " " %24.17g r(sd_1) " " r(N)
sdtesti 50 . .9 1.5
display "@ i1 " %24.17g r(chi2) " " %24.17g r(p)
sdtesti 30 2 1.3 25 1 .7
display "@ i2 " %24.17g r(F) " " %24.17g r(p) " " %24.17g r(sd)
regress y x
estimates store m1
regress y x g
estimates store m2
estimates stats m1 m2
matrix S = r(S)
display "@ stats " %24.17g S[1,2] " " %24.17g S[1,5] " " %24.17g S[2,6] " " S[2,4] " " S[1,1]
logit g y
estimates stats
matrix S = r(S)
display "@ lstats " %24.17g S[1,3] " " %24.17g S[1,6]
