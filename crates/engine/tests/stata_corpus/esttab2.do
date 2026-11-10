* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in esttab2.recorded
* commands: esttab estpost
* (esttab's stats() with fmt() and labels(), varlabels(), mgroups() with pattern() and prefix() suffix() span, starlevels(), compress, nogaps, nomtitles, nonotes; estpost summarize's e() and esttab, cells() of it with fmt(), in .tex and .txt files read back line by line, as counts, a weighted length sum and chosen lines)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 60
generate x = mod(_n * 7, 11) / 3
generate w = mod(_n * 5, 13) / 4
generate g = 1 + mod(_n, 3)
generate y = 1 + 0.5 * x - 0.3 * w + g / 4 + mod(_n * 17, 23) / 10
generate z = mod(_n * 3, 7) if _n > 6
label variable y "Outcome"
label variable w "Second_one"
eststo clear
eststo m1: regress y x
estadd scalar cm = 0.5
eststo m2: regress y x w
estadd scalar cm = -1.25
esttab m1 m2 using et2_a.tex, replace label b(%9.3f) se(%9.3f) keep(x) stats(N r2 cm, fmt(0 2 2) labels("Observations" "$ R^2$" "Control Mean")) varlabel(x "Treatment") starlevels(* .1 ** .05 *** .01) nomtitles mgroups("G one" "G two", pattern(1 1) prefix(\multicolumn{@span}{c}{) suffix(}) span) nogaps nonotes compress
esttab m1 m2 using et2_b.tex, replace label mgroups("Both", pattern(1 0)) varlabels(x "X_var") stats(N cm)
esttab m1 m2 using et2_c.txt, replace mgroups("G one" "G two", pattern(1 1) span) compress starlevels(+ .1 * .05) nogaps
eststo clear
estpost summarize y x w z if g > 1
display "@ ep " e(N) " " e(k) " " el(e(mean),1,1) " " el(e(sd),1,2) " " el(e(min),1,3) " " el(e(max),1,3) " " el(e(count),1,4) " " el(e(sum),1,2) " " el(e(Var),1,3) " " el(e(sum_w),1,4)
esttab using et2_d.tex, replace cells("mean(fmt(%8.2f)) sd(fmt(%8.2f)) min max count(fmt(%8.0f))")
esttab using et2_e.txt, replace label cells("mean(fmt(%8.3f)) sd")
foreach f in et2_a.tex et2_b.tex et2_c.txt et2_d.tex et2_e.txt {
  import delimited using `f', delimiters(tab) varnames(nonames) stringcols(_all) clear
  generate len = strlen(v1) * _n
  quietly summarize len
  local k = subinstr("`f'", ".", "_", .)
  display "@ `k' " _N " " r(sum)
}
import delimited using et2_a.tex, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ a_lines " (v1[8] == "Treatment       &    0.601***&    0.585***\\") " " (v1[12] == "$ R^2$          &     0.41   &     0.52   \\")
import delimited using et2_d.tex, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ d_lines " (v1[9] == "y           &        3.13&        1.01&        1.33&        5.22&          40\\") " " (v1[14] == "\(N\)       &          40&            &            &            &            \\")
erase et2_a.tex
erase et2_b.tex
erase et2_c.txt
erase et2_d.tex
erase et2_e.txt
