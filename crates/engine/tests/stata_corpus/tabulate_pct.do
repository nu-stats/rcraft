* RCraft's Stata corpus (P2C.12): made-up data; Stata 19's results are in tabulate_pct.recorded
* commands: tabulate
* (tabulate x y with row, column, cell and nofreq, which print Stata 19's table of percentages; r() as without them, chi2 with row)
* tolerance: 1e-9
version 19
clear
set seed 3
set obs 400
generate g = 1 + floor(3*runiform())
generate h = 1 + floor(2*runiform())
generate k = floor(9*runiform())
label define gl 1 "first" 2 "second group" 3 "Secondary - complete"
label values g gl
replace h = . in 1/5
tabulate g h, row
display "@ row " r(N) " " r(r) " " r(c)
tabulate g h, column nofreq
display "@ colnf " r(N) " " r(r) " " r(c)
tabulate g h, row col cell nofreq
display "@ all " r(N) " " r(r) " " r(c)
tabulate g h, row chi2
display "@ chi2 " r(N) " " %24.17g r(chi2) " " %24.17g r(p)
tabulate g h, nofreq
display "@ nofreq " r(N) " " r(r) " " r(c)
tabulate g k, cell
display "@ wide " r(N) " " r(r) " " r(c)
tabulate g h, row missing
display "@ missing " r(N) " " r(r) " " r(c)
tabulate g h, nofreq chi2
display "@ nfchi2 " r(N) " " r(r) " " r(c) " " %24.17g r(chi2) " " %24.17g r(p)
