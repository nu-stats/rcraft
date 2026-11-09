* RCraft's Stata corpus (Step 3): made-up files; Stata 19's results are in fileread.recorded
* commands: file
* (file read: a line into a local, without its end, r(status) for each ending (CR LF, LF, CR, none), r(eof) and an empty local past the end, an empty line, a tab and quotes kept, a loop over a file's lines, r() left by file open and close, a write-only handle r(198), a missing file r(601), a handle not open r(111))
* tolerance: 1e-8
version 19
set linesize 255
clear
set obs 3
generate x = _n
capture erase fileread_1.txt
file open w using fileread_1.txt, write binary
file write w %1bu (97) %1bu (13) %1bu (10) %1bu (98) %1bu (10) %1bu (10) %1bu (99) %1bu (13) %1bu (100) %1bu (9) %1bu (34) %1bu (32) %1bu (101) %1bu (13) %1bu (10) %1bu (102)
file close w
quietly summarize x
file open h using fileread_1.txt, read
display "@ open_leaves_r " r(N) " " r(mean)
local k = 0
local all ""
file read h line
while r(eof) == 0 {
  local k = `k' + 1
  local st = ("`r(status)'" == "win") + 2 * ("`r(status)'" == "unix") + 3 * ("`r(status)'" == "mac") + 4 * ("`r(status)'" == "none")
  display "@ line`k' " length(`"`line'"') " " `st' " " r(eof)
  local all `"`all'|`line'"'
  file read h line
}
display "@ lines " `k' " " r(eof) " " ("`r(status)'" == "eof") " " ("`line'" == "")
display "@ text " (`"`all'"' == `"|a|b||c|d"' + char(9) + `"" e|f"')
file read h again
display "@ past_end " r(eof) " " ("`again'" == "")
file close h
display "@ close_leaves_r " r(eof)
capture erase fileread_2.txt
file open w using fileread_2.txt, write
file write w "one" _n "  two  " _n _n "four"
file close w
file open h using fileread_2.txt, read
file read h a
file read h b
file read h c
file read h d
file read h e
display "@ written " ("`a'" == "one") " " ("`b'" == "  two  ") " " ("`c'" == "") " " ("`d'" == "four") " " ("`e'" == "") " " r(eof)
file close h
file open w using fileread_3.txt, write replace
capture file read w x
display "@ write_only " _rc
file close w
capture file open h using fileread_nosuch.txt, read
display "@ not_there " _rc
capture file read nosuch x
display "@ no_handle " _rc
erase fileread_1.txt
erase fileread_2.txt
erase fileread_3.txt
