* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in outreg.recorded
* commands: outreg frmttable
* (outreg from SSC as a replication package builds its balance table: a row per variable from each regression's treatment coefficient with stars, stored and appended, frmttable of a matrix of group sizes, means and standard deviations, merged and written as a LaTeX fragment with two title rows and multicolumn cells; mata clear between two tables; read back line by line)
version 19
set linesize 255
clear
set seed 21
set obs 200
gen Z = runiform() < .5
gen age = round(30 + 10*rnormal() + 2*Z)
gen female = runiform() < .4 + .1*Z
gen double inc = exp(8 + rnormal()) 
replace inc = . in 1/7
gen hh = round(4 + 2*runiform())
label var age "Age"
label var female "Female (share)"
label var inc "Monthly income"
label var hh "Household Size"
global balvars "age female inc hh"
mata: mata clear
local i = 1
foreach var in $balvars {
    reg `var' Z, robust
    outreg, keep(Z) rtitle("`: var label `var''") stats(b) noautosumm store(row`i') starlevels(10 5 1) starloc(1)
    outreg, replay(diff) append(row`i') ctitles("",Difference) store(diff) note("")
    local ++i
}
outreg, replay(diff)
local count: word count $balvars
mat sumstat = J(`count',6,.)
local i = 1
foreach var in $balvars {
    quietly: summarize `var' if Z==0
    mat sumstat[`i',1] = r(N)
    mat sumstat[`i',2] = r(mean)
    mat sumstat[`i',3] = r(sd)
    quietly: summarize `var' if Z==1
    mat sumstat[`i',4] = r(N)
    mat sumstat[`i',5] = r(mean)
    mat sumstat[`i',6] = r(sd)
    local i = `i' + 1
}
frmttable, statmat(sumstat) store(sumstat) sfmt(g,f,f,g,f,f)
outreg using "outreg_t2.tex", replay(sumstat) merge(diff) tex nocenter note("") fragment plain replace ctitles("", Control, "", "", Treatment, "", "", "" \ "", Observations, Mean, SD, Observations, Mean, SD, Difference in Means) multicol(1,2,3;1,5,3)
mata: mata clear
local i = 1
foreach var in age female {
    reg `var' Z if hh > 4, robust
    outreg, keep(Z) rtitle("`: var label `var''") stats(b) noautosumm store(row`i') starlevels(10 5 1) starloc(1)
    outreg, replay(diff) append(row`i') ctitles("",Difference) store(diff) note("")
    local ++i
}
mat s3 = J(2,2,.)
quietly summarize age if Z==0 & hh > 4
mat s3[1,1] = r(N)
mat s3[1,2] = r(mean)
quietly summarize female if Z==0 & hh > 4
mat s3[2,1] = r(N)
mat s3[2,2] = r(mean)
frmttable, statmat(s3) store(sumstat) sfmt(g,f)
outreg using "outreg_t7.tex", replay(sumstat) merge(diff) tex nocenter note("") fragment plain replace ctitles("", Observations, Control Mean, Treatment)
import delimited using outreg_t2.tex, delimiter("|") varnames(nonames) stringcols(_all) clear
display "@ lines " _N
display "@ l1 " (v1[1] == "\begin{tabular}{lccccccc}")
display "@ l2 " (v1[2] == "\hline  & \multicolumn{3}{c}{Control} & \multicolumn{3}{c}{Treatment} & \\")
display "@ l3 " (strtrim(v1[3]) == "& Observations & Mean & SD & Observations & Mean & SD & Difference in Means\\")
display "@ l4 " (v1[4] == "\hline Age & 108 & 31.19 & 10.07 & 92 & 33.36 & 9.65 & 2.164\\")
display "@ l5 " (v1[5] == "Female (share) & 108 & 0.31 & 0.47 & 92 & 0.51 & 0.50 & 0.196***\\")
display "@ l6 " (v1[6] == "Monthly income & 106 & 5777.27 & 6221.75 & 87 & 5065.02 & 5602.64 & -712.250\\")
display "@ l7 " (v1[7] == "Household Size & 108 & 4.93 & 0.68 & 92 & 4.97 & 0.72 & 0.041\\")
display "@ l8 " (v1[8] == "\hline\end{tabular}\\")
display "@ l9 " (v1[9] == "\hfil\\")
erase outreg_t2.tex
import delimited using outreg_t7.tex, delimiter("|") varnames(nonames) stringcols(_all) clear
display "@ t7 " _N " " (v1[2] == "\hline  & Observations & Control Mean & Treatment\\") " " (v1[3] == "\hline Age & 79 & 31.32 & 2.549\\") " " (v1[4] == "Female (share) & 79 & 0.32 & 0.191**\\")
erase outreg_t7.tex
