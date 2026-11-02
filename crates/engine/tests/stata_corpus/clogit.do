* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in clogit.recorded
* commands: clogit
* (clogit with one and with several positives in a group, groups that are all positive or all negative, a factor variable, a variable constant within groups, robust and clustered errors (vce(clu) and vce(r) as written in the packages), groups not nested in clusters, if, or, and predict's pc1, pu0 and xb)
* tolerance: 1e-6
version 19
clear
set obs 120
generate g = ceil(_n / 4)
generate alt = mod(_n - 1, 4) + 1
generate x = mod(_n * 7, 13) / 3
generate w = mod(_n * 5, 7)
generate a = mod(_n * 5, 3) + 1
generate c = ceil(g / 3)
generate y = (alt == 1 + mod(g * 7, 4))
replace y = (x + w / 2 > 4) if g > 25
replace y = 0 if g == 1
replace y = 1 if g == 2
clogit y x w i.a, group(g)
display "@ plain " e(N) " " e(ll) " " e(ll_0) " " e(chi2) " " e(p) " " e(r2_p) " " e(k) " " e(df_m) " " e(rank)
display "@ dropped " e(N_drop) " " e(N_group_drop)
display "@ b " _b[x] " " _b[w] " " _b[2.a] " " _b[3.a]
display "@ se " _se[x] " " _se[w] " " _se[2.a] " " _se[3.a]
clogit y x w i.a, group(g) vce(cluster c)
display "@ cluster " e(N) " " e(N_clust) " " e(chi2) " " e(ll)
display "@ cluster_se " _se[x] " " _se[w] " " _se[2.a] " " _se[3.a]
clogit y x w i.a, group(g) vce(clu c)
display "@ cluster_abbreviated " e(N_clust) " " _se[x]
clogit y x w i.a, group(g) vce(r)
display "@ robust_abbreviated " _se[x]
clogit y x w i.a, group(g) vce(robust)
display "@ robust_se " _se[x] " " _se[w] " " _se[2.a] " " _se[3.a]
display "@ robust_chi2 " e(chi2)
clogit y x w i.a, group(g) or
display "@ or " _b[x] " " _se[x]
predict p
quietly summarize p
display "@ pc1 " r(N) " " r(mean) " " r(min) " " r(max)
predict xb, xb
quietly summarize xb
display "@ xb " r(N) " " r(mean) " " r(min) " " r(max)
predict u, pu0
quietly summarize u
display "@ pu0 " r(N) " " r(mean)
capture clogit y x w, group(g) vce(cluster alt)
display "@ not_nested " _rc
generate xg = g
clogit y x xg, group(g)
display "@ flat " e(N) " " _b[x] " " e(df_m) " " e(ll)
clogit y x w if g > 2 & g <= 25, group(g)
display "@ one_positive " e(N) " " _b[x] " " _b[w] " " _se[x] " " e(ll)
clogit y x w if g > 2 & g <= 25, group(g) vce(robust)
display "@ one_positive_robust " _se[x] " " _se[w]
clogit y x w if g > 5, strata(g)
display "@ strata " e(N) " " _b[x] " " e(ll)
