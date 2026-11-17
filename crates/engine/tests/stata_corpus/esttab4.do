* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in esttab4.recorded
* commands: esttab
* (esttab, wide: each statistic beside its coefficient, with t, se and label, one model and three, in .txt files read back line by line, as counts, a weighted length sum and chosen lines)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 40
generate price = 3000 + mod(_n * 137, 97) * 61
generate mpg = 12 + mod(_n * 7, 23)
generate weight = 1800 + mod(_n * 53, 89) * 30
generate byte foreign = mod(_n, 3) == 0
label variable price "Price"
label variable mpg "Mileage (mpg)"
eststo clear
eststo: quietly regress price mpg
eststo: quietly regress price mpg weight
eststo m3: quietly regress price mpg weight foreign
esttab m3 using et4_a.txt, replace wide
esttab using et4_b.txt, replace wide
esttab m3 using et4_c.txt, replace wide se
esttab m3 using et4_d.txt, replace wide label
foreach f in et4_a.txt et4_b.txt et4_c.txt et4_d.txt {
  import delimited using `f', delimiters(tab) varnames(nonames) stringcols(_all) clear
  generate len = strlen(v1) * _n
  quietly summarize len
  local k = subinstr("`f'", ".", "_", .)
  display "@ `k' " _N " " r(sum)
}
import delimited using et4_a.txt, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ a_lines " (v1[8] == "_cons              6438.0***       (4.07)") " " (v1[2] == "                      (1)                ")
import delimited using et4_b.txt, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ b_lines " (v1[6] == "weight                                          -0.430         (-1.22)       -0.431         (-1.20)")
import delimited using et4_c.txt, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ c_lines " (v1[8] == "_cons              6438.0***     (1583.5)")
import delimited using et4_d.txt, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ d_lines " (v1[5] == "Mileage (mpg)               29.88          (0.72)")
erase et4_a.txt
erase et4_b.txt
erase et4_c.txt
erase et4_d.txt
