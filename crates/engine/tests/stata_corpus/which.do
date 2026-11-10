* RCraft's Stata corpus (P2C.9 continued): Stata 19's results are in which.recorded
* commands: which
* (capture which and quietly which: _rc 0 for a built-in command and for estout's esttab and estout, 111 for a name that is no command; quietly leaves _rc as it was)
* tolerance: 0
version 19
capture which regress
display "@ a " _rc
capture which nosuchcmd_xyz
display "@ b " _rc
quietly which regress
display "@ c " _rc
capture which esttab
display "@ d " _rc
capture which estout
display "@ e " _rc
local n = 0
foreach p in summarize nosuchcmd_abc generate {
    capture which `p'
    if _rc local n = `n' + 1
}
display "@ f " `n'
