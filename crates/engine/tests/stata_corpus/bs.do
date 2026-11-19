* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in bs.recorded
* commands: bootstrap
* (bs and bstrap, the other names Stata takes for bootstrap, as BraderTuckerDuell writes them: seed() with leading zeros, level(90), if() with no space before it, e(ci_percentile), e(b) and e(N) read into matrices, the command over two lines, and an r-class command's sample as its if and in with missing values kept)
* tolerance: 1e-9
version 19
set linesize 255
clear
set obs 60
set seed 77
generate y = round(rnormal()*2 + 3, .01)
generate g = mod(_n, 2)
generate c = 1 + mod(_n, 3)
quietly bs mean = r(mean), reps(25) seed(010101)  level(90): sum y if(c==1&g==0)
matrix ci = e(ci_percentile)
matrix mean = e(b)
matrix N = e(N)
display "@ bs_ci " %24.17g ci[1,1] " " %24.17g ci[2,1]
display "@ bs_b " %24.17g mean[1,1] " " N[1,1] " " e(level) " " %24.17g _se[mean]
bstrap r(sd), reps(15) seed(3) nodots: summarize y if g == 1
display "@ bstrap " %24.17g _se[_bs_1] " " e(N) " " e(N_reps)
quietly bs mean = r(mean), reps(20) seed(010101) ///
level(90): sum y if(g==0)
matrix ci = e(ci_percentile)
local lb = ci[1,1]
local ub = ci[2,1]
matrix mean = e(b)
local mean = mean[1,1]
matrix N = e(N)
local N = N[1,1]
display "@ bs_lines " %24.17g `lb' " " %24.17g `ub' " " %24.17g `mean' " " `N'
replace y = . in 1/7
bs r(mean), reps(5) seed(1) nodots: summarize y
display "@ miss_all " e(N) " " %24.17g _se[_bs_1]
bs r(mean), reps(5) seed(1) nodots: summarize y if g==1
display "@ miss_if " e(N) " " %24.17g _se[_bs_1]
bs r(mean), reps(5) seed(1) nodots: summarize y in 10/40
display "@ in " e(N) " " %24.17g _se[_bs_1]
bs r(mean), reps(5) seed(1) nodots: summarize y if g==1 in 10/40
display "@ ifin " e(N) " " %24.17g _se[_bs_1]
display "@ next " %24.17g runiform()
