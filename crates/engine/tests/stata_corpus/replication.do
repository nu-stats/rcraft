* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in replication.recorded
* commands: append erase fvset reshape
* (forms published replication packages use: erase and rm, and a file that is not there; save over a file; append with force, either way; reshape long without j(); fvset base before models, kept by save and use and by preserve and restore, not by import; cluster() and cl() for vce(cluster))
* tolerance: 1e-8
version 19
import delimited corpus.csv, clear
save rep_a, replace
erase rep_a.dta
capture erase rep_a.dta
display "@ erase_rc " _rc
save rep_b, replace
rm "rep_b.dta"
capture rm rep_b.dta
display "@ rm_rc " _rc
save rep_c, replace
capture save rep_c
display "@ save_rc " _rc
erase rep_c.dta
* append with force: a using variable of the other kind is missing
import delimited corpus.csv, clear
keep id g y
tostring g, replace
save rep_s, replace
import delimited corpus.csv, clear
keep id g y
append using rep_s, force
erase rep_s.dta
count if missing(g)
display "@ force_n " r(N)
display "@ force_rows " _N
summarize y
display "@ force_y " %24.17g r(sum)
summarize g
display "@ force_g " %24.17g r(sum)
display "@ force_gn " r(N)
import delimited corpus.csv, clear
keep id g
save rep_n, replace
tostring g, replace
append using rep_n, force
erase rep_n.dta
count if g == ""
display "@ force_s " r(N)
display "@ force_srows " _N
* reshape long without j() makes _j
import delimited corpus.csv, clear
generate double v1 = x
generate double v2 = w
keep id v1 v2
reshape long v, i(id)
summarize _j
display "@ reshape_j " %24.17g r(sum)
display "@ reshape_n " r(N)
generate double p = v * _j
summarize p
display "@ reshape_p " %24.17g r(sum)
* fvset base
import delimited corpus.csv, clear
fvset base 3 cat3
regress y i.cat3 x
display "@ fv_reg1 " %24.17g _b[1.cat3]
display "@ fv_reg2 " %24.17g _b[2.cat3]
display "@ fv_se2 " %24.17g _se[2.cat3]
regress y cat3#f
display "@ fv_cell12 " %24.17g _b[1.cat3#2.f]
display "@ fv_cell32 " %24.17g _b[3.cat3#2.f]
logit yb i.cat3 x
display "@ fv_logit " %24.17g _b[1.cat3]
display "@ fv_logit_se " %24.17g _se[1.cat3]
save rep_f, replace
import delimited corpus.csv, clear
regress y i.cat3
display "@ fv_import " %24.17g _b[2.cat3]
use rep_f, clear
regress y i.cat3
display "@ fv_use " %24.17g _b[2.cat3]
fvset base default cat3
regress y i.cat3
display "@ fv_default " %24.17g _b[2.cat3]
fvset base 2 cat3
preserve
fvset clear cat3
restore
regress y i.cat3
display "@ fv_restore " %24.17g _b[1.cat3]
erase rep_f.dta
* cluster() and cl() are vce(cluster); robust beside them changes nothing
regress y x, cluster(panel)
display "@ cl_reg " %24.17g _se[x]
regress y x, robust cl(panel)
display "@ cl_robust " %24.17g _se[x]
logit yb x, cluster(panel)
display "@ cl_logit " %24.17g _se[x]
