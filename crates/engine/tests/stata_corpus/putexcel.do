* RCraft's Stata corpus (packages' gaps): made-up workbooks; Stata 19's results are in putexcel.recorded
* commands: putexcel import
* (putexcel as a replication package fills a results workbook: set with sheet() and modify or replace, text cells from macros, numbers and ( expressions ) to 17 digits, several cells a line, a cell written over, . emptying a cell, another sheet kept, and its errors; read back with import excel)
* tolerance: 1e-15
version 19
set linesize 255
capture erase px.xlsx
putexcel set "px.xlsx", sheet("T") replace
putexcel A1 = "text" A2 = ".123"
local coeff = round(0.12345, 0.001)
putexcel A3 = "`coeff'"
local coeff : display "(" `coeff' ")"
putexcel A4 = "`coeff'"
putexcel set "px.xlsx", sheet("N") modify
putexcel A1 = (1/3) A2 = 3.5 A3 = 1e-20
putexcel a4 = 123456789012
putexcel A5 = (2 + 3)
putexcel A6 = 7
putexcel A6 = .
putexcel A7 = 9
putexcel set "px.xlsx", sheet("T") modify
putexcel A1 = "again"
import excel using "px.xlsx", sheet("T") clear
display "@ text " A[1] "|" A[2] "|" A[3] "|" A[4] " " _N
import excel using "px.xlsx", sheet("N") clear
display "@ num " %21.0g A[1] " " %21.0g A[2] " " %21.0g A[3] " " %21.0g A[4] " " %21.0g A[5] " " missing(A[6]) " " %21.0g A[7] " " _N
capture noisily putexcel set "px.xlsx", sheet("X")
display "@ exists " _rc
capture noisily putexcel XFE1 = "x"
display "@ column " _rc
putexcel clear
capture noisily putexcel A1 = "x"
display "@ noset " _rc
erase px.xlsx
