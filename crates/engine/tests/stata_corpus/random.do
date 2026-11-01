* RCraft's Stata corpus (P2C.11): made-up data; Stata 19's results are in random.recorded
* commands: expand generate replace set
* functions: rgamma rnormal runiform
* (Stata's own random draws: set seed, runiform(), rnormal() and rgamma() in display, macros, scalars, generate, replace, keep, expand, if blocks and loops; the rows an if or in selects; arguments that give missing; by groups)
* tolerance: 0
version 19
clear
set seed 20261101
display "@ u_first " %24.17g runiform()
display "@ u_ab " %24.17g runiform(-3, 7)
display "@ u_bad " %24.17g runiform(2, 1)
display "@ n_first " %24.17g rnormal()
display "@ n_ms " %24.17g rnormal(10, 3)
display "@ n_s0 " %24.17g rnormal(4, 0)
display "@ g_half " %24.17g rgamma(.5, 2)
display "@ g_one " %24.17g rgamma(1, 3)
display "@ g_big " %24.17g rgamma(2.5, 1)
display "@ g_bad " %24.17g rgamma(0, 1)
local r = runiform()
display "@ local_u " %24.17g `r'
scalar s = rnormal()
display "@ scalar_n " %24.17g s
quietly display runiform()
display "@ after_quiet " %24.17g runiform()
if runiform() < 2 {
    display "@ if_block " %24.17g runiform()
}
forvalues i = 1/3 {
    display "@ loop_`i' " %24.17g rnormal()
}
set obs 400
generate id = _n
generate double u = runiform()
generate double z = rnormal()
generate double g1 = rgamma(1, 3)
generate double g2 = rgamma(.3, 1)
generate double g3 = rgamma(30, .5)
generate double ua = runiform(-1, 1)
generate f = rnormal()
generate grp = mod(_n, 3)
generate double sel = runiform() if grp == 1
replace sel = rnormal(5, cond(grp == 0, 0, 2)) in 2/9
generate double m = rnormal(cond(mod(_n, 7) == 0, ., 1), 1)
generate double c = cond(grp == 1, runiform(), -1)
generate double k = runiform() if _N > 5
display "@ u_1 " %24.17g u[1]
display "@ u_400 " %24.17g u[400]
display "@ z_1 " %24.17g z[1]
display "@ z_257 " %24.17g z[257]
display "@ z_400 " %24.17g z[400]
display "@ g1_9 " %24.17g g1[9]
display "@ g2_100 " %24.17g g2[100]
display "@ g2_400 " %24.17g g2[400]
display "@ g3_33 " %24.17g g3[33]
display "@ g3_400 " %24.17g g3[400]
display "@ ua_5 " %24.17g ua[5]
display "@ f_7 " %24.17g f[7]
display "@ sel_1 " %24.17g sel[1]
display "@ sel_2 " %24.17g sel[2]
display "@ sel_3 " %24.17g sel[3]
display "@ sel_4 " %24.17g sel[4]
display "@ sel_10 " %24.17g sel[10]
display "@ sel_400 " %24.17g sel[400]
display "@ m_7 " %24.17g m[7]
display "@ m_8 " %24.17g m[8]
display "@ m_400 " %24.17g m[400]
display "@ c_4 " %24.17g c[4]
display "@ c_400 " %24.17g c[400]
display "@ k_1 " %24.17g k[1]
display "@ k_400 " %24.17g k[400]
display "@ data_and_draw " %24.17g u[1] + runiform()
sort grp id
by grp: generate double first = runiform() if _n == 1
display "@ first_1 " %24.17g first[1]
display "@ first_mid " %24.17g first[134]
display "@ first_last " %24.17g first[268]
preserve
keep if runiform() < .5
display "@ kept_N " %24.17g _N
display "@ kept_u_1 " %24.17g u[1]
restore
display "@ restored_N " %24.17g _N
expand 1 + (runiform() < .3) if grp == 2
display "@ expand_N " %24.17g _N
display "@ after_expand " %24.17g runiform()
set seed -7
display "@ seed_neg " %24.17g runiform()
set seed 7
display "@ seed_pos " %24.17g runiform()
* the 18,482nd rgamma(2.5) draw from here falls in G4PE's left tail at or below 0
clear
set obs 20000
set seed 20261101
generate double ru = runiform()
generate double rz = rnormal()
generate double ga = rgamma(.5, 1)
generate double gb = rgamma(1, 2)
generate double gc = rgamma(2.5, 1)
display "@ gc_18481 " %24.17g gc[18481]
display "@ gc_18482 " %24.17g gc[18482]
display "@ gc_20000 " %24.17g gc[20000]
display "@ tail_next " %24.17g runiform()
