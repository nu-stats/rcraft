* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in xtcollinear.recorded
* commands: xtreg
* (xtreg re with variables the constant and the ones before them explain (shares that add to 1: the last is omitted), with more columns than panels (sigma_u is 0 and re is least squares), and with fewer clusters than slopes (the Wald test is missing))
version 19
set linesize 255
clear
set seed 21
set obs 400
gen int id = ceil(_n/10)
bysort id: gen int meet = _n
xtset id meet
gen double a = round(runiform(), .01)
gen double f = round(runiform()*(1-a), .01)
gen double n2 = round(runiform()*(1-a-f), .01)
gen double gv = 1 - a - f - n2
gen double age = 30 + mod(id, 20) + meet
gen double x = round(rnormal(), .01)
gen double y = round(x + 2*a - f + .1*age + rnormal() + mod(id,5)/3, .001)
quietly xtreg y x a f n2 gv age i.meet, cluster(id)
display "@ shares " _b[x] " " _se[x] " " _b[a] " " _b[n2] " " _b[gv] " " _se[gv] " " _b[age] " " _b[5.meet] " " _b[_cons] " " e(chi2) " " e(sigma_u) " " e(sigma_e)
clear
set obs 120
gen int id = ceil(_n/20)
bysort id: gen int t = _n
xtset id t
gen int g = ceil(id/2)
gen double x = round(rnormal(), .01)
gen double y = round(x + t/10 + rnormal() + id/3, .001)
quietly xtreg y x i.t, cluster(g)
display "@ narrow " _b[x] " " _se[x] " " _b[5.t] " " _b[_cons] " " e(sigma_u) " " e(sigma_e) " " (e(chi2) == .) " " e(N_clust) " " e(r2_o)
