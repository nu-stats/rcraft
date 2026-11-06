* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in xtcollinear.recorded
* commands: xtreg
* (xtreg re with variables the constant and the ones before them explain (shares that add to 1: the last is omitted), with more columns than panels (sigma_u is 0 and re is least squares), and with fewer clusters than slopes (the Wald test is missing); xtreg fe with a variable collinear with the time dummies within the panels, exactly (the later column omitted, as Stata does), and with a variable named twice (o. after it in e(b)); regress with g#c.x and no g, every level estimated in e(b))
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
clear
set seed 21
set obs 400
generate int id = ceil(_n/10)
bysort id: generate int meet = _n
xtset id meet
generate double age = 30 + mod(id, 20) + meet
generate double x = round(rnormal(), .01)
generate double y = round(x + .1*age + rnormal() + mod(id,5)/3, .001)
quietly xtreg y x age i.meet, fe
display "@ within " _b[x] " " _se[x] " " _b[age] " " _b[2.meet] " " _b[9.meet] " " _b[10.meet] " " _b[_cons] " " e(F) " " e(r2_w) " " e(sigma_u)
replace age = round(30 + mod(id, 20) + meet/3 + runiform(), .01)
quietly xtreg y x age age i.meet, fe cluster(id)
matrix b = e(b)
display "@ twice " colsof(b) " " b[1,1] " " b[1,2] " " b[1,3] " " b[1,4] " " b[1,5] " " _se[x] " " e(N_g)
generate byte g = mod(_n, 2)
quietly regress y g#c.x age
matrix b = e(b)
display "@ gcx " colsof(b) " " b[1,1] " " b[1,2] " " b[1,3] " " _b[0.g#c.x] " " _b[1.g#c.x]
