* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in epitab.recorded
* commands: cs csi cc cci ir iri
* (risk, odds and rate tables: Wald and log intervals, Woolf's and the test-based odds-ratio interval, the exact Poisson rate ratio, attributable and prevented fractions, chi2, Fisher's exact p, mid-p; a nonzero value counts as exposed)
* tolerance: 1e-9
version 19
clear
set linesize 255
set obs 240
generate id = _n
generate exposed = mod(id*7, 10) < 4
generate case = mod(id*13 + exposed*5, 10) < 3 + 2*exposed
generate pt = 50 + mod(id*17, 40)
generate e3 = mod(id, 3)
replace case = . in 1/4
quietly cs case exposed
display "@ cs " %24.17g r(rd) " " %24.17g r(lb_rd) " " %24.17g r(ub_rr) " " %24.17g r(lb_afe) " " %24.17g r(afp) " " %24.17g r(chi2)
quietly cs case e3, exact level(90)
display "@ cse " %24.17g r(lb_rr) " " %24.17g r(p_exact) " " %24.17g r(p1_exact)
quietly csi 10 30 90 70
display "@ csi " %24.17g r(afe) " " %24.17g r(lb_afe) " " %24.17g r(afp)
quietly cc case exposed, woolf
display "@ cc " %24.17g r(or) " " %24.17g r(lb_or) " " %24.17g r(ub_afe) " " %24.17g r(afp) " " %24.17g r(p)
quietly cci 30 20 50 100, tb
display "@ cci " %24.17g r(lb_or) " " %24.17g r(ub_or) " " %24.17g r(lb_afe)
quietly cci 15 40 60 50, woolf level(99)
display "@ ccp " %24.17g r(afe) " " %24.17g r(ub_afe) " " %24.17g r(afp)
quietly ir case exposed pt
display "@ ir " %24.17g r(irr) " " %24.17g r(lb_irr) " " %24.17g r(ub_ird) " " %24.17g r(p_twosided_midp) " " %24.17g r(p_upper_exact)
quietly iri 10 30 3000 2500, level(90)
display "@ iri " %24.17g r(ub_irr) " " %24.17g r(afe) " " %24.17g r(afp) " " %24.17g r(p_lower_midp)
