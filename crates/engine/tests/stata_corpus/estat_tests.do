* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in estat_tests.recorded
* commands: estat
* (estat vif, centered and uncentered, with a factor variable; on tsset data bgodfrey and durbinalt with lags() and nomiss0, dwatson, a gap in the sample; r() with its matrices)
* tolerance: 1e-9
version 19
clear
set linesize 255
set seed 92
set obs 120
generate t = _n
generate x = rnormal()
generate w = rnormal()
generate v = x + .3*rnormal()
generate g = 1 + floor(3*runiform())
generate e = rnormal()
replace e = e + .5*e[_n-1] in 2/l
generate y = 1 + .5*x + .2*w + .1*v + e
regress y w x v
estat vif
display "@ vif " %24.17g r(vif_1) " " r(name_1) " " %24.17g r(vif_3) " " r(name_3)
estat vif, uncentered
display "@ vifu " %24.17g r(vif_1) " " r(name_1) " " %24.17g r(vif_4) " " r(name_4)
regress y x w i.g
estat vif
display "@ viffv " %24.17g r(vif_1) " " r(name_3) " " %24.17g r(vif_4)
tsset t
regress y x w
estat bgodfrey, lags(1 2)
display "@ bg " %24.17g el(r(chi2),1,1) " " %24.17g el(r(chi2),1,2) " " %24.17g el(r(p),1,2) " " r(k) " " r(N) " " r(lags)
estat bgodfrey, nomiss0
display "@ bgnm " %24.17g el(r(chi2),1,1)
estat durbinalt, lags(1 2)
display "@ da " %24.17g el(r(chi2),1,1) " " %24.17g el(r(chi2),1,2)
estat durbinalt, nomiss0
display "@ danm " %24.17g el(r(chi2),1,1)
estat dwatson
display "@ dw " %24.17g r(dw) " " r(k) " " r(N) " " r(N_gaps)
drop in 50/52
regress y x w
estat bgodfrey
display "@ bggap " %24.17g el(r(chi2),1,1) " " r(N_gaps)
estat durbinalt
display "@ dagap " %24.17g el(r(chi2),1,1)
estat dwatson
display "@ dwgap " %24.17g r(dw) " " r(N_gaps)
