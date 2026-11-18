* RCraft's Stata corpus (P2C.12, batch C): made-up data; Stata 19's results are in pcorr.recorded
* commands: pcorr
* (pcorr: partial and semipartial correlations, their squares and p values in r(), r(N) and r(df), frequency and analytic weights, if, and the errors with their return codes)
version 19
set linesize 255
clear
set obs 40
generate double x = mod(_n * 37, 101) / 7
generate double y = 2 + .5 * x + mod(_n * 13, 17) / 3
generate double z = mod(_n * 11, 23) / 4 - .2 * x
generate int g = 1 + mod(_n, 3)
generate int k = 1 + mod(_n * 7, 5)
generate double w2 = 0.5 + mod(_n * 3, 7) / 4
replace z = . in 3
replace k = . in 5/6
pcorr y x z
display "@ n1 " r(N) " " r(df)
display "@ p1 " %24.17g el(r(p_corr),1,1) " " %24.17g el(r(p_corr),2,1)
display "@ s1 " %24.17g el(r(sp_corr),1,1) " " %24.17g el(r(sp_corr),2,1)
display "@ dim1 " rowsof(r(p_corr)) " " colsof(r(sp_corr))
pcorr y x z g k
display "@ n2 " r(N) " " r(df)
forvalues i = 1/4 {
    display "@ pp2_`i' " %24.17g el(r(p_corr),`i',1) " " %24.17g el(r(sp_corr),`i',1)
}
pcorr y x z g if g < 3
display "@ n3 " r(N) " " r(df) " " %24.17g el(r(p_corr),3,1) " " %24.17g el(r(sp_corr),3,1)
pcorr y x z [fweight = k]
display "@ n4 " r(N) " " r(df)
display "@ pp4 " %24.17g el(r(p_corr),1,1) " " %24.17g el(r(sp_corr),1,1) " " %24.17g el(r(p_corr),2,1) " " %24.17g el(r(sp_corr),2,1)
pcorr y x z [aweight = k]
display "@ n5 " r(N) " " r(df)
display "@ pp5 " %24.17g el(r(p_corr),1,1) " " %24.17g el(r(sp_corr),1,1) " " %24.17g el(r(p_corr),2,1) " " %24.17g el(r(sp_corr),2,1)
pcorr y x z [aweight = w2]
display "@ n6 " r(N) " " r(df)
display "@ pp6 " %24.17g el(r(p_corr),1,1) " " %24.17g el(r(sp_corr),1,1) " " %24.17g el(r(p_corr),2,1) " " %24.17g el(r(sp_corr),2,1)
capture pcorr y x z [pweight = w2]
display "@ rc_pw " _rc
capture pcorr y x z [iweight = w2]
display "@ rc_iw " _rc
capture pcorr y x z [fweight = w2]
display "@ rc_fw " _rc
capture pcorr y x
display "@ rc_two " _rc
capture pcorr y
display "@ rc_one " _rc
capture pcorr
display "@ rc_none " _rc
capture pcorr y x z if 0
display "@ rc_empty " _rc
generate str3 s = "a"
capture pcorr y x s
display "@ rc_str " _rc
capture pcorr y x nosuch
display "@ rc_nosuch " _rc
capture pcorr y x x
display "@ rc_dup " _rc
pcorr x y z in 5/30
display "@ n7 " r(N) " " r(df) " " %24.17g el(r(p_corr),1,1)
pcorr x y z if g == 1
display "@ n8 " r(N) " " r(df)
