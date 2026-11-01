* RCraft's Stata corpus (P2C.16): made-up data; Stata 19's results are in bsample.recorded
* commands: bsample
* (bsample with Stata's own draws: the rows drawn and their order, a smaller sample, if, cluster() and idcluster(), and the draws after it)
* tolerance: 1e-9
version 19
import delimited corpus.csv, clear
set seed 1601
bsample
generate double pos = _n * id
summarize pos
display "@ plain_pos " %24.17g r(sum) " " %24.17g r(N)
summarize y
display "@ plain_y " %24.17g r(mean) " " %24.17g r(sd)
regress y x w
display "@ plain_reg " %24.17g _b[x] " " %24.17g _se[w]
display "@ plain_next " %24.17g runiform()
import delimited corpus.csv, clear
set seed 77
bsample 25
generate double pos = _n * id
summarize pos
display "@ small_pos " %24.17g r(sum) " " %24.17g r(N)
display "@ small_next " %24.17g runiform()
import delimited corpus.csv, clear
set seed 4242
bsample if g == 1
generate double pos = _n * id
summarize pos
display "@ if_pos " %24.17g r(sum) " " %24.17g r(N)
display "@ if_next " %24.17g runiform()
import delimited corpus.csv, clear
set seed 31
bsample, cluster(cat3) idcluster(copy)
summarize copy
display "@ cl_copy " %24.17g r(sum) " " %24.17g r(N)
summarize y
display "@ cl_y " %24.17g r(mean) " " %24.17g r(max)
generate double pos = _n * id
summarize pos
display "@ cl_pos " %24.17g r(sum)
display "@ cl_next " %24.17g runiform()
import delimited corpus.csv, clear
set seed 9
forvalues r = 1/5 {
    preserve
    bsample
    quietly summarize y
    display "@ loop_`r' " %24.17g r(mean)
    restore
}
display "@ loop_next " %24.17g runiform()
