* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in xtabsorbed.recorded
* commands: xtreg
* (TZEN38's xtreg, fe with age and meeting dummies, clustered on another variable than the panel: Stata 19 then omits the column its largest-pivot sweep leaves with nothing, 'collinearity with the absorbed variable', here 102.meet; plain or clustered on the panel it omits the later one, 104.meet)
* tolerance: 1e-8
version 19
set linesize 255
clear
set seed 1002
set obs 29
generate int id = _n
generate int start = 1 + floor(runiform() * 104 * .6)
generate int len = 8 + floor(runiform() * 104)
generate int birth = 1930 + floor(runiform() * 30)
expand 104
bysort id: generate int meet = _n
keep if meet >= start & meet < start + len
drop if runiform() < .04
generate int year = 1990 + ceil(meet / 8)
generate double age = year - birth
generate double x = round(rnormal(), .01)
generate double y = round(x + .05 * age + rnormal() + mod(id, 5) / 3, .001)
generate int cl = ceil(id / 3)
xtset id meet
quietly xtreg y x age i.meet, fe cluster(cl)
display "@ absorbed " e(N) " " _b[x] " " _b[age] " " _b[102.meet] " " _b[104.meet] " " _b[60.meet] " " _b[_cons] " " e(r2) " " e(sigma_u)
quietly xtreg y x age i.meet, fe cluster(id)
display "@ panel " _b[x] " " _se[x] " " _b[age] " " _b[102.meet] " " _b[104.meet] " " _b[_cons]
quietly xtreg y x age i.meet, fe
display "@ plain " _b[age] " " _se[age] " " _b[102.meet] " " _b[104.meet] " " _b[_cons]
