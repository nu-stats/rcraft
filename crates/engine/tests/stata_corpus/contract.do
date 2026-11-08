* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in contract.recorded
* commands: contract recast
* (contract as YFBPT6 counts respondents by country and age group or gender: missing values as categories, nomiss, zero, the frequency and percent options, float, fweights, if and in, and its errors; recast to double, to float with and without force, and to the integer types)
version 19
set linesize 255
clear
set seed 2468
set obs 250
generate long id = _n
generate byte a = 1 + floor(runiform()*4)
replace a = . if runiform() < .08
generate byte b = 1 + (runiform() > .6)
replace b = . if runiform() < .05
generate double u = runiform()
generate double v = runiform()
generate str2 s = cond(u < .1, "", "k" + string(1 + floor(v*3)))
generate byte w = 1 + floor(runiform()*3)
replace w = . in 7
generate double x = runiform()*1000 - 300
preserve
contract a b
display "@ plain " _N " " a[1] " " b[1] " " _freq[1] " " a[_N] " " b[_N] " " _freq[_N]
summarize _freq
display "@ plain_sum " r(sum) " " r(min) " " r(max)
restore
preserve
contract a b, freq(n) cfreq(cf) percent(p) cpercent(cp)
display "@ all " _N " " n[3] " " cf[3] " " %21x p[3] " " %21x cp[3] " " %21x cp[_N]
display "@ all2 " a[5] " " b[5] " " n[5] " " %21x p[5] " " %21x cp[5]
restore
preserve
contract a b, nomiss
display "@ nomiss " _N " " _freq[1] " " a[_N] " " b[_N] " " _freq[_N]
restore
preserve
contract a b, zero
display "@ zero " _N " " _freq[_N] " " a[_N] " " b[_N]
summarize _freq
display "@ zero_sum " r(sum) " " r(min)
restore
preserve
contract a b, zero nomiss
display "@ zeronomiss " _N
restore
preserve
contract s a, percent(p) float format(%6.1f)
display "@ float " _N " [" s[1] "] " a[1] " " _freq[1] " " %21x p[1] " [" s[_N] "]"
restore
preserve
contract s, nomiss
display "@ snomiss " _N " [" s[1] "] " _freq[1]
restore
preserve
contract a [fw=w], percent(p)
display "@ fw " _N " " _freq[1] " " %21x p[1] " " _freq[_N]
restore
preserve
contract b if a == 2
display "@ if " _N " " _freq[1] " " _freq[2]
restore
preserve
contract a in 20/60
display "@ in " _N " " _freq[1]
restore
preserve
capture contract a, freq(a)
display "@ freqa " _rc " " _N
capture contract a, percent(id)
display "@ pctid " _rc " " _N
capture contract a if id > 1000
display "@ none " _rc " " _N
restore
recast double b
recast float x
display "@ nf " %21x x[1]
recast float x, force
display "@ f " %21x x[1] " " %21x x[2]
recast int x, force
display "@ int " x[1] " " x[2] " " x[3]
recast byte x, force
summarize x
display "@ byte " r(N) " " r(min) " " r(max)
