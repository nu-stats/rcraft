* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in clogit_margins.recorded
* commands: parmest
* (margins after clogit as a conjoint replication package runs it: fvset bases, clogit with clusters, margins ar. of three terms with predict(pu0), parmest of the model, margins dydx() of two levels, margins dydx(*) post predict(pu0) vce(unconditional) mcomp(bonferroni), and parmest of the margins; both parmest files read back)
* tolerance: 1e-6
version 19
set linesize 255
clear
set seed 411
set obs 1200
generate id_g = ceil(_n / 12)
generate csid = ceil(_n / 2)
generate actor = ceil(runiform() * 5)
generate reform = ceil(runiform() * 4)
generate opinion = ceil(runiform() * 3)
generate u = 0.3 * (actor == 2) - 0.4 * (reform == 1) + 0.5 * (opinion == 3) - 0.2 * (actor == 4) + rlogistic()
bysort csid: egen m = max(u)
generate chosen = u == m
drop m u
fvset base 1 opinion
fvset base 4 reform
fvset base 5 actor
clogit chosen i.actor i.reform i.opinion, group(csid) vce(clu id_g) baselevels
display "@ clogit " e(N) " " e(N_clust) " " _b[1.actor] " " _se[1.actor] " " _b[3.opinion] " " _se[1.reform]
margins ar.actor ar.reform ar.opinion, predict(pu0) asobserved cformat(%5.3f)
display "@ ar " el(r(b), 1, 1) " " el(r(b), 1, 4) " " el(r(b), 1, 5) " " el(r(b), 1, 9) " " sqrt(el(r(V), 1, 1)) " " sqrt(el(r(V), 5, 5)) " " sqrt(el(r(V), 9, 9)) " " el(r(V), 1, 2)
parmest, idnum(7) saving(cm_contrast, replace)
margins, dydx(1.reform 3.opinion) predict(pu0)
display "@ levels " colsof(r(b)) " " el(r(b), 1, 1) " " el(r(b), 1, colsof(r(b))) " " sqrt(el(r(V), 1, 1)) " " sqrt(el(r(V), colsof(r(b)), colsof(r(b)))) " " el(r(V), 1, colsof(r(b)))
margins, dydx(*) post predict(pu0) asobserved cformat(%5.3f) vce(unconditional) mcomp(bonferroni)
display "@ dydx " _b[1.actor] " " _b[4.actor] " " _b[1.reform] " " _b[3.opinion] " " _se[1.actor] " " _se[2.reform] " " _se[3.opinion] " " e(N)
display "@ eb " colsof(e(b)) " " el(e(b), 1, 1) " " el(e(b), 1, 5) " " el(e(b), 1, 9) " " el(e(b), 1, 12) " " el(e(V), 1, 1) " " el(e(V), 5, 5)
matrix list e(b)
parmest, idnum(7) saving(cm_dydx, replace)
use cm_contrast, clear
quietly count if eq == "chosen"
local eqs = r(N)
quietly count if parm == "5b.actor" & _n == 5
local base = r(N)
display "@ p_contrast " _N " " idnum[1] " " `eqs' " " `base' " " estimate[1] " " stderr[1] " " z[1] " " p[1] " " min95[1] " " max95[12] " " missing(z[5])
use cm_dydx, clear
quietly count if (parm == "1.actor" & _n == 1) | (parm == "1b.opinion" & _n == 10)
local names = r(N)
display "@ p_dydx " _N " " `names' " " estimate[2] " " stderr[2] " " p[12] " " min95[6] " " max95[7] " " estimate[9] " " missing(p[9])
erase cm_contrast.dta
erase cm_dydx.dta
