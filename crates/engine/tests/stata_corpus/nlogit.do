* RCraft's Stata corpus (batch MV): made-up data; Stata 19's results are in nlogit.recorded
* commands: nlogitgen nlogittree nlogit predict summarize
* (a two-level nested logit: the tree, type-level variables with a base, alternative constants, technique(nr) at the optimum, the default technique's log likelihood, predicted probabilities)
* tolerance: 5e-5
version 19
set linesize 255
clear
set seed 539
set obs 1200
generate int id = ceil(_n / 4)
generate byte alt = mod(_n - 1, 4) + 1
generate double price = 1 + runiform() * 3 + alt / 2
generate double inc = 10 + mod(id * 13, 17)
generate double u = -0.8 * price + 0.5 * (alt == 2) + inc / 20 * (alt == 3) - 0.3 * (alt == 4) - ln(-ln(runiform()))
bysort id (u): generate byte choice = _n == _N
sort id alt
label define al 1 "car" 2 "bus" 3 "train" 4 "plane"
label values alt al

nlogitgen type = alt(ground: car | bus, other: train | plane)
forvalues i = 1/1200 {
    if mod(`i', 100) == 1 {
        display "@ type `i' " type[`i']
    }
}
tabulate type alt
nlogittree alt type, choice(choice)
nlogit choice price || type: inc, base(ground) || alt:, noconstant case(id) technique(nr)
display "@ a_sc " e(N) " " e(N_case) " " e(rank) " " e(df_m) " " e(df_c) " " e(k_alt) " " e(alt_min) " " e(alt_max) " " e(levels) " " e(rum)
display "@ a_ll " %17.0g e(ll) " " %17.0g e(ll_c) " " %17.0g e(chi2) " " %17.0g e(p) " " %17.0g e(chi2_c) " " %17.0g e(p_c)
display "@ a_txt " ("`e(depvar)'" == "choice") ("`e(case)'" == "id") ("`e(altvar)'" == "alt") ("`e(altvar1)'" == "type") ("`e(alteqs1)'" == "ground other") ("`e(alteqs)'" == "car bus train plane")
matrix b = e(b)
display "@ a_b " %17.0g b[1,1] " " %17.0g b[1,2] " " %17.0g b[1,3] " " %17.0g b[1,4]
matrix V = e(V)
forvalues i = 1/4 {
    display "@ a_V `i' " %17.0g V[`i',1] " " %17.0g V[`i',2] " " %17.0g V[`i',3] " " %17.0g V[`i',4]
}
nlogit choice price || type:, base(ground) || alt:, case(id) technique(nr) nolog
display "@ b_sc " e(N) " " e(rank) " " e(df_m) " " e(k_alt)
display "@ b_ll " %17.0g e(ll) " " %17.0g e(ll_c) " " %17.0g e(chi2) " " %17.0g e(chi2_c) " " %17.0g e(p_c)
matrix b = e(b)
display "@ b_b " %17.0g b[1,1] " " %17.0g b[1,2] " " %17.0g b[1,3] " " %17.0g b[1,4] " " %17.0g b[1,5] " " %17.0g b[1,6]
matrix V = e(V)
forvalues i = 1/6 {
    display "@ b_V `i' " %17.0g V[`i',1] " " %17.0g V[`i',2] " " %17.0g V[`i',3] " " %17.0g V[`i',4] " " %17.0g V[`i',5] " " %17.0g V[`i',6]
}
predict q*
summarize q1
display "@ b_q1 " r(N) " " %17.0g r(mean) " " %17.0g r(sd) " " %17.0g r(min) " " %17.0g r(max)
summarize q2
display "@ b_q2 " r(N) " " %17.0g r(mean) " " %17.0g r(sd) " " %17.0g r(min) " " %17.0g r(max)
forvalues i = 1/8 {
    display "@ b_q `i' " %17.0g q1[`i'] " " %17.0g q2[`i']
}
nlogit choice price || type: inc, base(ground) || alt:, noconstant case(id)
display "@ c_ll " %17.0g e(ll) " " e(N) " " e(rank)
nlogit choice price || type:, base(ground) || alt:, case(id) nolog
display "@ d_ll " %17.0g e(ll) " " e(N) " " e(rank)
