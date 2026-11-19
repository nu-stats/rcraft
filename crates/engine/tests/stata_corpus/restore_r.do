* RCraft's Stata corpus (P2C.12, batch I follow-up): made-up data; Stata 19's results are in restore_r.recorded
* commands: preserve restore
* (preserve and restore leave r() and e() as the last command left them, whatever the data were)
* tolerance: 1e-8
version 19
set linesize 255
import delimited corpus.csv, clear
summarize x
preserve
display "@ pre " %24.17g r(mean) " " r(N)
summarize x if x > 0
replace x = x + 100
restore
display "@ post " %24.17g r(mean) " " r(N) " " _N " " %24.17g x[1]
regress y x
preserve
summarize w
restore
display "@ rest " %24.17g r(mean) " " r(N)
regress y x
preserve
replace w = 0
restore
display "@ e " e(N) " " %24.17g _b[x]
