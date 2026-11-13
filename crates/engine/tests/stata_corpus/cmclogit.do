* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in cmclogit.recorded
* commands: cmset cmclogit margins coefplot
* (cmclogit with alternative-specific factors and case factors, vce(cluster); margins, dydx() of its factors posted with vce(unconditional); coefplot of those margins, which have an equation each)
* tolerance: 1e-7
version 19
clear
set obs 1200
generate cs = ceil(_n / 2)
generate alt = 1 + mod(_n - 1, 2)
generate id = ceil(cs / 6)
generate country = 1 + mod(id, 3)
generate actor = 1 + mod(floor(_n * 1.618), 3)
generate reform = 1 + mod(floor(_n * 2.414), 2)
generate u = 0.6 * (actor == 2) - 0.4 * (actor == 3) + 0.5 * (reform == 2) + 0.3 * (country == 2) * (alt == 2) + sin(_n * 12.9898) * 2
bysort cs (u): generate chosen = _n == 2
sort cs alt
fvset base 3 actor
cmset cs alt
cmclogit chosen i.actor i.reform, casevars(i.country) vce(cluster id) noconstant
display "@ a " e(N) " " e(N_case) " " e(N_clust) " " e(ll) " " e(chi2) " " e(df_m)
display "@ b " _b[1.actor] " " _se[1.actor] " " _b[2.reform] " " _se[2.reform] " " _b[2:2.country] " " _se[2:2.country]
margins, dydx(i.actor i.reform) post asobserved cformat(%5.3f) vce(unconditional) outcome(1) alt(1)
display "@ c " _b[1.actor:_cons] " " _se[1.actor:_cons] " " _b[2.actor:_cons] " " _se[2.actor:_cons] " " _b[2.reform:_cons] " " _se[2.reform:_cons]
coefplot, keep(*:) omitted baselevels noeqlabels headings(1.actor: = "{bf:Actor}" 1b.reform: _cons = "{bf:Reform}") gen(g_) nodraw
display "@ d " g_at[1] " " g_at[2] " " g_at[3] " " g_at[4] " " g_at[5] " " g_b[3] " " g_ll1[5] " " g_ul1[5]
coefplot, keep(2.actor:_cons _cons) noeqlabels gen(h_) nodraw
display "@ e " h_at[1] " " h_b[1] " " missing(h_at[2])
quietly cmclogit chosen i.actor, casevars(i.country)
display "@ f " e(ll) " " _b[2.actor] " " _se[2.actor] " " _b[2:_cons] " " _se[2:_cons]
