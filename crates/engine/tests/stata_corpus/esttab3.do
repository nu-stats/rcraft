* RCraft's Stata corpus (P2C.9 continued): made-up data; Stata 19's results are in esttab3.recorded
* commands: esttab
* (esttab's prehead() posthead() prefoot() postfoot() with @title, fragment, mgroups(..., erepeat()), mlabels() with span prefix() suffix() and without, alignment(), addnotes() between the notes, noobs in .tex, in .tex and .txt files read back line by line, as counts, a weighted length sum and chosen lines)
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 60
generate x = mod(_n * 7, 11) / 3
generate w = mod(_n * 5, 13) / 4
generate g = 1 + mod(_n, 3)
generate y = 1 + 0.5 * x - 0.3 * w + g / 4 + mod(_n * 17, 23) / 10
generate y2 = 2 - 0.2 * x + mod(_n * 11, 19) / 7
label variable y "Outcome"
label variable w "Second_one"
eststo clear
eststo m1: regress y x
estadd scalar cm = 0.5
eststo m2: regress y x w
estadd scalar cm = -1.25
eststo m3: regress y2 x
estadd scalar cm = 2
esttab m1 m2 m3 using et3_a.tex, replace label b(%9.3f) se(%9.3f) keep(x w) varlabel(x "Treatment" w "$\times$ W") stats(N r2 cm, fmt(0 2 3) labels("Observations" "R-squared" "Control Mean")) nomtitles nonumbers starlevels(* .1 ** .05 *** .01) prehead("\begin{tabular}{l*{3}{c}}" "\hline\hline" "& (1) & (2) & (3) \\") posthead("\hline") prefoot("\hline") postfoot("\hline\hline" "\end{tabular}") nogaps nonotes compress
esttab m1 m2 m3 using et3_b.tex, replace label b(%9.3f) se(%9.3f) mtitles("One" "Two" "Three") mgroups("First" "Second", pattern(1 0 1) prefix(\multicolumn{@span}{c}{) suffix(}) span erepeat(\cmidrule(lr){@span})) starlevels(* .1 ** .05 *** .01) nogaps nonotes compress
esttab m1 m2 m3 using et3_c.tex, booktabs label b(%9.2f) se(%9.2f) mlabels("(1)" "(2)" "(3)", noti nonum span prefix(\multicolumn{@span}{c}{) suffix(})) varlabels(x "X var" _cons "Constant") alignment(D{.}{.}{-1}) nonumber replace nonotes addnote("N = 3; * p $<$ 0.10.") star(* 0.10 ** 0.05 *** 0.01) noobs title(A title \label{tab})
esttab m1 m2 using et3_d.tex, replace fragment mlabels("A" "B") prefoot("PF")
esttab m1 m2 using et3_e.tex, replace title(Tt) prehead("% @title" "\begin{tabular}{lcc}") posthead("\hline") mlabels("A", prefix(<) suffix(>))
esttab m1 m2 m3 using et3_f.txt, replace mlabels("A" "B" "C", span) mgroups("G", pattern(1 0 0) span erepeat(-@span-)) addnotes("Note one" "Note two") prefoot("PF")
foreach f in et3_a.tex et3_b.tex et3_c.tex et3_d.tex et3_e.tex et3_f.txt {
  import delimited using `f', delimiters(tab) varnames(nonames) stringcols(_all) clear
  generate len = strlen(v1) * _n
  quietly summarize len
  local k = subinstr("`f'", ".", "_", .)
  display "@ `k' " _N " " r(sum)
}
import delimited using et3_b.tex, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ b_lines " (v1[5] == "                &\multicolumn{2}{c}{First}&\multicolumn{1}{c}{Second}\\\cmidrule(lr){2-3}\cmidrule(lr){4-4}")
import delimited using et3_c.tex, delimiters(tab) varnames(nonames) stringcols(_all) clear
display "@ c_lines " (v1[4] == "\begin{tabular}{l*{3}{D{.}{.}{-1}}}") " " (v1[6] == "                    &\multicolumn{1}{c}{(1)}&\multicolumn{1}{c}{(2)}&\multicolumn{1}{c}{(3)}\\")
erase et3_a.tex
erase et3_b.tex
erase et3_c.tex
erase et3_d.tex
erase et3_e.tex
erase et3_f.txt
