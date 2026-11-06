* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in cluster2.recorded
* commands: cluster2
* (cluster2, Mitchell Petersen's add-on from his web page, not on SSC, as the replication packages run it: one row per firm-year and several, a subsample whose variance comes out below 0, and a constant alone; its F and degrees of freedom from the cell-clustered or robust fit)
version 19
set linesize 255
clear
set seed 77
set obs 300
gen int firm = ceil(_n/10)
bysort firm: gen int year = _n
gen double x = round(rnormal() + firm/30, .001)
gen double z = round(runiform(), .001)
gen double y = round(1 + .5*x - .2*z + rnormal() + mod(firm,7)/5 + year/10, .001)
replace y = . in 5
cluster2 y x z, fcluster(firm) tcluster(year)
display "@ a " %20.0g _b[x] " " %20.0g _se[x] " " %20.0g _se[z] " " %20.0g _se[_cons] " " %20.0g e(F) " " e(df_r) " " %20.0g e(r2_a) " " %20.0g e(ll)
clear
set seed 78
set obs 400
gen int firm = ceil(_n/20)
gen int year = ceil(runiform()*6)
gen double x = round(rnormal() + firm/30, .001)
gen double y = round(1 + .5*x + rnormal() + mod(firm,7)/5 + year/10, .001)
replace x = . in 7
cluster2 y x, fcluster(firm) tcluster(year)
display "@ b " %20.0g _b[x] " " %20.0g _se[x] " " %20.0g _se[_cons] " " %20.0g e(F) " " e(df_r) " " e(N)
quietly cluster2 y x if firm < 3, fcluster(firm) tcluster(year)
matrix V = e(V)
display "@ c " %20.0g _se[x] " " %20.0g V[2,2] " " %20.0g e(F) " " e(df_r)
quietly cluster2 y, fcluster(firm) tcluster(year)
display "@ d " %20.0g _se[_cons] " " e(F) " " e(df_r) " " e(df_m)
