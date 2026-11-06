* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in outreg2tex.recorded
* commands: outreg2
* (outreg2 as TZEN38 runs it after xtreg: se tex(fragment) label ctitle() keep() addtext() addstat(), a re column and an fe one with the factor variables kept out, an interaction a##b in keep(); the .txt and the .tex read back line by line and cell by cell)
* tolerance: 0
version 19
set linesize 255
clear
set seed 11
set obs 240
generate int id = ceil(_n/8)
bysort id: generate int meet = _n
xtset id meet
generate double Inflation = round(rnormal()*2 + 2, .01)
generate double Unemp = round(runiform()*5 + 3, .01)
generate double inflexp = round(rnormal() + 1, .01)
generate double age = 40 + mod(_n, 25)
generate double Tone = round(.3*Inflation - .2*Unemp + .1*Inflation*inflexp + rnormal() + id/50, .001)
label variable Inflation "Inflation"
label variable Unemp "Unemployment_Rate %"
label variable inflexp "Inflation experiences"
label variable Tone "Tone index"
quietly xtreg Tone Inflation Unemp age i.meet, cluster(id)
outreg2 using "orx1.tex", bdec(3) tdec(3) se tex(fragment) lab replace ctitle(Overall) keep(Inflation Unemp) addtext(Prof Exp FE, \checkmark, Age, \checkmark, Meeting FE, No) addstat(R-squared, e(r2_o))
quietly xtreg Tone Inflation Unemp age i.meet, fe cluster(id)
outreg2 using "orx1.tex", bdec(3) tdec(3) se tex(fragment) lab ctitle(Policy go-round) keep(Inflation Unemp) addtext(Prof Exp FE, No, Age, No, Meeting FE, \checkmark)
quietly xtreg Tone c.Inflation##c.inflexp Unemp age i.meet, fe cluster(id)
outreg2 using "orx2.tex", bdec(3) tdec(3) se tex(fragment) lab replace ctitle(Overall) keep(c.Inflation##c.inflexp Unemp) addtext(Meeting FE, \checkmark)
quietly regress Tone Inflation Unemp
outreg2 using "orx3.tex", bdec(2) tdec(2) se tex(fragment) replace addstat(Small, 0.0123456, Big, 12345.678)
import delimited using orx1.txt, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ txt1 " _N " " (v2[6] == "Unemployment_Rate %") " " (v3[4] == "0.445***") " " (v4[5] == "(0.037)") " " (v1[12] == "R-squared") " " (v1[13] == "Number of id") " " (v4[14] == "No") " " (v1[17] == "Robust standard errors in parentheses")
import delimited using orx1.tex, delimiters("@") varnames(nonames) stringcols(_all) clear
display "@ tex1 " _N " " (v1[1] == "\begin{tabular}{lcc} \hline") " " (v1[3] == "VARIABLES & Overall & Policy go-round \\ \hline") " " (v1[7] == "Unemployment\_Rate \% & -0.192*** & -0.210*** \\") " " (strtrim(v1[17]) == "Meeting FE & No & \checkmark \\ \hline") " " (v1[19] == "\multicolumn{3}{c}{ *** p$<$0.01, ** p$<$0.05, * p$<$0.1} \\")
import delimited using orx2.tex, delimiters("@") varnames(nonames) stringcols(_all) clear
display "@ tex2 " _N " " (v1[7] == "Inflation experiences & 0.126 \\") " " (v1[9] == "c.Inflation\#c.inflexp & 0.044 \\") " " (v1[17] == "Number of id & 30 \\") " " (v1[18] == "R-squared & 0.560 \\")
import delimited using orx3.tex, delimiters("@") varnames(nonames) stringcols(_all) clear
display "@ tex3 " _N " " (strlen(strtrim(v1[6])) == 11) " " (v1[14] == "Small & 0.0123 \\") " " (strtrim(v1[15]) == "Big & 12346 \\ \hline") " " (v1[16] == "\multicolumn{2}{c}{ Standard errors in parentheses} \\")
erase orx1.tex
erase orx1.txt
erase orx2.tex
erase orx2.txt
erase orx3.tex
erase orx3.txt
