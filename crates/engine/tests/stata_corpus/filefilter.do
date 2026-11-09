* RCraft's Stata corpus (packages' gaps): made-up files; Stata 19's results are in filefilter.recorded
* commands: filefilter
* (filefilter as a replication package edits the LaTeX table it wrote: \BS and \n codes, a line added inside a match, matches counted left to right without overlap, r(occurrences), r(bytes_from), r(bytes_to), an empty from(), decimal codes, and its errors; the file read back line by line)
version 19
set linesize 255
tempname fh
file open `fh' using ff_in.tex, write replace
file write `fh' "\hline  & \multicolumn{3}{c}{Control} & \\" _n
file write `fh' " & Observations & Mean & Difference\\" _n
file write `fh' "aaaa and aaa" _n
file close `fh'
filefilter ff_in.tex ff_mid.tex, from(`"\BShline  & \BSmulticolumn{3}{c}{Control} & \BS\BS"') to(`"\BShline\n\BShline  & X \BS\BS \n\BScmidrule(lr){2-4} \BS\BS"') replace
display "@ first " r(occurrences) " " r(bytes_from) " " r(bytes_to)
filefilter ff_mid.tex ff_out.tex, from(`" & Observations & Mean & Difference\BS\BS"') to(`"& Observations & Mean \BS\BS"') replace
display "@ second " r(occurrences) " " r(bytes_from) " " r(bytes_to)
filefilter ff_out.tex ff_aa.tex, from("aa") to("\066d") replace
display "@ overlap " r(occurrences) " " r(bytes_to)
filefilter ff_out.tex ff_none.tex, from("zzz") to("y") replace
display "@ none " r(occurrences)
capture filefilter ff_out.tex ff_none.tex, from("a") to("b")
display "@ exists " _rc
capture filefilter ff_nothere.tex ff_x.tex, from("a") to("b") replace
display "@ missing " _rc
capture filefilter ff_out.tex ff_out.tex, from("a") to("b") replace
display "@ same " _rc
capture filefilter ff_out.tex ff_x.tex, from("\h") to("b") replace
display "@ badcode " _rc
import delimited using ff_aa.tex, delimiter("|") varnames(nonames) stringcols(_all) clear
display "@ lines " _N
display "@ l1 " (v1[1] == "\hline")
display "@ l2 " (strtrim(v1[2]) == "\hline  & X \\")
display "@ l3 " (v1[3] == "\cmidrule(lr){2-4} \\")
display "@ l4 " (v1[4] == "& Observations & Mean \\")
display "@ l5 " (v1[5] == "BB and Ba")
erase ff_in.tex
erase ff_mid.tex
erase ff_out.tex
erase ff_aa.tex
erase ff_none.tex
