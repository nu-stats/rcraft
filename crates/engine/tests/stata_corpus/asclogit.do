* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in asclogit.recorded
* commands: _pctile asclogit drawnorm
* (asclogit with alternative-specific variables, noconstant and clustered errors, its e() and e(b); e(sample) to keep the rows; drawnorm from means(e(b)) and cov(e(V)), double and float, the draws bit for bit; the simulated probabilities and _pctile, as YFBPT6 runs them)
version 19
set linesize 255
matrix m = (1, -2, .5)
matrix V = (2, .3, -.4 \ .3, 1, .2 \ -.4, .2, .5)
clear
set seed 1234
drawnorm a b c, n(5) means(m) cov(V) double
display "@ double " %21x a[1] " " %21x b[3] " " %21x c[5] " " %21x a[5]
clear
set seed 99
drawnorm a b c, n(4) means(m) cov(V)
display "@ float " %21x a[1] " " %21x b[2] " " %21x c[4]
* YFBPT6's chain on made-up data
clear
set seed 77
set obs 600
generate int csid = ceil(_n/2)
bysort csid: generate byte alt = _n
generate int id_g = ceil(csid/5)
generate byte ac1 = runiform() < .3
generate byte ac2 = runiform() < .3
generate byte re1 = runiform() < .5
generate byte op2 = runiform() < .4
generate double u = .8*ac1 - .5*ac2 + .6*re1 - .4*op2 - ln(-ln(runiform()))
bysort csid (u): generate byte chosen = _n == _N
sort csid alt
quietly asclogit chosen ac1 ac2 re1 op2, case(csid) alternatives(alt) vce(clu id_g) noconstant
display "@ asc " _b[ac1] " " _se[ac1] " " _b[op2] " " _se[op2] " " e(ll) " " e(chi2) " " e(N_case) " " e(N_clust) " " e(k_alt) " " e(alt_avg)
drop if !e(sample)
display "@ sample " _N
set seed 1234
drawnorm beta1-beta4, n(1000) means(e(b)) cov(e(V)) clear
generate Vcan0 = exp(0 + beta3*1 + beta4*1)
generate Vcan1 = exp(beta1*1 + beta3*1 + beta4*1)
gen prob10 = Vcan1 / (Vcan1 + Vcan0)
egen ev10 = mean(prob10)
_pctile prob10, p(2.5,97.5)
display "@ sim " %21x beta1[1] " " %21x beta4[1000] " " ev10[1] " " r(r1) " " r(r2)
