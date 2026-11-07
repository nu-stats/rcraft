* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in svy.recorded
* commands: svy svyset
* (svyset with a PSU and pweights, svy: clogit with factor variables and fvset bases, as YFBPT6 runs it: the weighted estimates, the linearized variance, the design df and the adjusted Wald F; predict pc1 and pu0; margins dydx(*) with predict(pu0) asobserved vce(unconditional) and Bonferroni, the weighted average marginal effects)
* tolerance: 1e-5
* (margins' standard errors rest on numerical derivatives, Stata's steps its own: one of them agrees to 6 digits, the estimates to 8)
version 19
set linesize 255
clear
set seed 88
set obs 1200
generate int csid = ceil(_n/2)
bysort csid: generate byte alt = _n
generate int id_g = ceil(csid/5)
generate double weight_rake = .5 + mod(id_g*37, 11)/7
generate byte actor = 1 + floor(runiform()*5)
generate byte reform = 1 + floor(runiform()*4)
generate byte opinion = 1 + floor(runiform()*3)
generate double u = .3*(actor==2) - .4*(actor==4) + .5*(reform==1) - .6*(opinion==3) - ln(-ln(runiform()))
bysort csid (u): generate byte chosen = _n == _N
sort csid alt
fvset base 1 opinion
fvset base 4 reform
fvset base 5 actor
svyset id_g [pweight=weight_rake]
quietly svy: clogit chosen i.actor i.reform i.opinion, group(csid) baselevels
display "@ fit " _b[1.actor] " " _se[1.actor] " " _b[4.actor] " " _se[4.actor] " " _b[3.opinion] " " _se[3.opinion] " " e(N) " " e(N_pop) " " e(N_psu) " " e(df_r) " " e(F) " " e(p) " " e(df_m) " " e(rank)
predict pc1
predict pu0, pu0
display "@ pred " pc1[1] " " pc1[2] " " pu0[1] " " pu0[7]
quietly margins, dydx(*) post predict(pu0) asobserved vce(unconditional) mcomp(bonferroni)
display "@ amce " _b[1.actor] " " _se[1.actor] " " _b[1.reform] " " _se[1.reform] " " _b[3.opinion] " " _se[3.opinion]
