* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in suest.recorded
* commands: estimates suest
* (suest of two conditional logits on two groups of respondents, clustered by respondent, as a conjoint replication package compares them; margins after it of the sum of the two equations' predicted probabilities, dydx() of a level of each group's factor, posted, and the test that they are equal; the same without clusters)
* tolerance: 1e-6
version 19
set linesize 255
clear
set seed 411
set obs 2400
generate id_g = ceil(_n / 12)
generate csid = ceil(_n / 2)
generate know = mod(id_g, 2)
generate actor = ceil(runiform() * 5)
generate reform = ceil(runiform() * 4)
generate opinion = ceil(runiform() * 3)
generate u = 0.3 * (actor == 2) - 0.4 * (reform == 1) + 0.5 * (opinion == 3) - 0.2 * (actor == 4) + 0.3 * know * (reform == 1) + rlogistic()
bysort csid: egen m = max(u)
generate chosen = u == m
drop m u
generate reform2 = reform
generate opinion2 = opinion
fvset base 1 opinion
fvset base 4 reform
fvset base 5 actor
fvset base 1 opinion2
fvset base 4 reform2
clogit chosen i.actor i.reform i.opinion if know == 1, group(csid) baselevels
estimates store co
clogit chosen i.actor i.reform2 i.opinion2 if know == 0, group(csid) baselevels
estimates store fls
estimates table co fls, stats(N ll) star
display "@ et " ("`r(names)'" == "co fls")
suest co fls, vce(clu id_g) coefl
display "@ suest " e(N) " " e(N_clust) " " e(rank) " " colsof(e(b)) " " %20.0g el(e(b), 1, 1) " " %20.0g el(e(b), 1, 18) " " %20.0g el(e(b), 1, 5)
display "@ suest_V " %20.0g el(e(V), 1, 1) " " %20.0g el(e(V), 6, 6) " " %20.0g el(e(V), 1, 6) " " %20.0g el(e(V), 18, 18) " " %20.0g el(e(V), 13, 24) " " %20.0g el(e(V), 1, 13) " " %20.0g el(e(V), 5, 5)
margins, dydx(1.reform 1.reform2) expression(invlogit(predict(equation(co_chosen))) + invlogit(predict(equation(fls_chosen)))) post coefl asobserved cformat(%5.3f) mcomp(bonferroni)
display "@ m " e(N) " " colsof(e(b)) " " %20.0g _b[1.reform] " " %20.0g _b[1.reform2] " " %20.0g _se[1.reform] " " %20.0g _se[1.reform2] " " _b[4b.reform]
test 1.reform = 1.reform2
display "@ t " %20.0g r(chi2) " " %20.0g r(p) " " r(df)
suest co fls
display "@ plain " e(N) " " %20.0g el(e(V), 1, 1) " " %20.0g el(e(V), 1, 13) " " %20.0g el(e(V), 9, 9)
margins, dydx(3.opinion 3.opinion2) expression(invlogit(predict(equation(co_chosen))) + invlogit(predict(equation(fls_chosen))))
display "@ m2 " colsof(r(b)) " " %20.0g el(r(b), 1, 2) " " %20.0g el(r(b), 1, 4) " " %20.0g sqrt(el(r(V), 2, 2))
