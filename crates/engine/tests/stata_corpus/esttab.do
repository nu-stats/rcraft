* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in esttab.recorded
* commands: esttab
* (esttab, the estout package's table: its .tex (booktabs and plain) and .txt files read back line by line, as counts, a weighted length sum and chosen lines: a3 and fixed formats, t, se and p, stars, label with value labels and a long label, base levels, ar2, r2, scalars(), title(), mtitles(), nonumbers, keep(), order())
* tolerance: 0
version 19
set linesize 255
clear
set obs 60
generate x = mod(_n * 7, 11) / 3
generate w = mod(_n * 5, 13) / 4
generate g = 1 + mod(_n, 3)
label define gl 1 "low" 2 "middle group" 3 "high"
label values g gl
generate y = 1 + 0.5 * x - 0.3 * w + g / 4 + mod(_n * 17, 23) / 10
label variable x "A rather long label for exposure"
label variable y "Outcome"
eststo clear
eststo m1: regress y x
eststo m2: regress y x w i.g
esttab m1 m2 using et_a.tex, replace booktabs
esttab m1 m2 using et_b.tex, replace label se ar2 b(%9.2f) se(%9.3f) title("Parties")
esttab m1 m2 using et_c.txt, replace label p r2 star(* 0.10 ** 0.05 *** 0.01) mtitles("First" "Second") nonumbers
esttab m1 m2 using et_d.txt, replace scalars(F rmse ll) keep(x w) order(w x) nobaselevels
foreach f in et_a.tex et_b.tex et_c.txt et_d.txt {
  import delimited using `f', delimiters(tab) varnames(nonames) stringcols(_all) clear
  generate len = strlen(v1) * _n
  quietly summarize len
  local k = subinstr("`f'", ".", "_", .)
  display "@ `k' " _N " " r(sum)
}
import delimited using et_a.tex, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ a_lines " (v1[8] == "x           &       0.601\sym{***}&       0.598\sym{***}\\") " " (v1[20] == "3.g         &                     &       0.540\sym{*}  \\")
import delimited using et_c.txt, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ c_lines " (strtrim(v1[2]) == "First          Second") " " (v1[_N] == "* p<0.10, ** p<0.05, *** p<0.01")
erase et_a.tex
erase et_b.tex
erase et_c.txt
erase et_d.txt
