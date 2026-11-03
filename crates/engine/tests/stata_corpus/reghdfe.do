* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in reghdfe.recorded
* commands: reghdfe
* (reghdfe, the SSC package, as the replication packages run it: one and two absorbed variables and noabsorb; singletons dropped and kept; unadjusted, robust, clustered and two-way clustered errors, with a fixed effect nested in the clusters or not; its degrees of freedom and R-squareds)
* tolerance: 1e-6
version 19
set linesize 255
clear
set seed 2026
set obs 600
generate id = ceil(_n / 6)
replace id = 101 in 600
generate t = mod(_n - 1, 6) + 1
generate c = ceil(id / 5)
generate q = ceil(runiform() * 15)
generate x = rnormal() + id / 50
generate w = rnormal() + t / 4
generate y = 1 + 0.5 * x - 0.2 * w + rnormal() * 2 + id / 20 + t / 3 + q / 5
reghdfe y x w, absorb(id)
display "@ one " e(N) " " e(num_singletons) " " _b[x] " " _b[w] " " _b[_cons] " " _se[x] " " _se[w] " " _se[_cons]
display "@ one_e " e(df_r) " " e(df_a) " " e(F) " " e(r2) " " e(r2_a) " " e(r2_within) " " e(r2_a_within) " " e(rmse) " " e(ll) " " e(ll_0)
reghdfe y x w, absorb(id) vce(robust)
display "@ robust " _se[x] " " _se[w] " " _se[_cons] " " e(df_r) " " e(F)
reghdfe y x w, absorb(id) cluster(c)
display "@ nested " _se[x] " " _se[w] " " _se[_cons] " " e(df_r) " " e(df_a) " " e(N_clust) " " e(F)
reghdfe y x w, absorb(id) cluster(q)
display "@ not_nested " _se[x] " " _se[w] " " _se[_cons] " " e(df_r) " " e(df_a) " " e(N_clust)
reghdfe y x w, absorb(id t)
display "@ two " _b[x] " " _b[w] " " _b[_cons] " " _se[x] " " _se[w] " " e(df_r) " " e(df_a) " " e(r2_a) " " e(r2_a_within)
reghdfe y x w, absorb(id t) cluster(c q)
display "@ two_way " _se[x] " " _se[w] " " _se[_cons] " " e(df_r) " " e(df_a) " " e(N_clust1) " " e(N_clust2) " " e(F)
reghdfe y x w, absorb(t) cluster(c q)
display "@ t_two_way " _b[x] " " _se[x] " " _se[w] " " _se[_cons] " " e(df_r) " " e(df_a)
reghdfe y x w, noabsorb cluster(c q)
display "@ none " _b[x] " " _b[_cons] " " _se[x] " " _se[_cons] " " e(df_r) " " e(df_a) " " e(r2_a)
reghdfe y x w, absorb(id t) cluster(c)
display "@ two_nested " _se[x] " " _se[w] " " _se[_cons] " " e(df_r) " " e(df_a)
reghdfe y x w, absorb(id t) keepsingletons cluster(q)
display "@ keep " e(N) " " _b[x] " " _se[x] " " e(df_r) " " e(df_a)
reghdfe y x w if t > 1, absorb(id) cluster(c)
display "@ if " e(N) " " _b[x] " " _se[x] " " e(df_r)
