* RCraft's Stata corpus (packages' gaps): made-up data; Stata 19's results are in outreg2_onecol.recorded
* commands: outreg2
* (outreg2 with onecol, as BraderTuckerDuell writes it, after models of one equation, where Stata writes the files it writes without onecol: regress to a .txt and .xls, xtreg fe to a .tex, and a regress column added; the files read back as numbers)
* tolerance: 0
version 19
set linesize 255
clear
set seed 4
set obs 120
generate id = ceil(_n/6)
bysort id: generate t = _n
xtset id t
generate x = rnormal()
generate w = rnormal()
generate y = 1 + x - w + rnormal() + id/20
regress y x w
outreg2 using oc_a.xls, tstat bdec(3) tdec(2) replace onecol
xtreg y x w, fe
outreg2 using oc_b.tex, se bdec(3) tdec(3) tex(fragment) replace onecol
regress y x
outreg2 using oc_b.tex, se bdec(3) tdec(3) tex(fragment) onecol
import delimited using oc_a.txt, delimiters(tab) varnames(nonames) stringcols(_all) clear
generate len = strlen(v1) + 2 * strlen(v2)
quietly summarize len
display "@ a_rows " _N " " r(sum)
import delimited using oc_b.tex, delimiters("@") varnames(nonames) stringcols(_all) clear
generate len = strlen(v1)
generate amp = length(v1) - length(subinstr(v1, "&", "", .))
quietly summarize len
display "@ b_rows " _N " " r(sum)
quietly summarize amp
display "@ b_amp " r(sum)
erase oc_a.txt
erase oc_a.xls
erase oc_b.txt
erase oc_b.tex
