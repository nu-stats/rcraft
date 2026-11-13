* RCraft's Stata corpus (P2C.9 continued): Stata 19's results are in fmtround.recorded
* commands: display
* (%f formats of values next to a half: .005 is a little above one, so it rounds up; exact halves to even)
* tolerance: 0
version 19
display "@ a " %9.2f 0.005 " " %9.2f -0.005 " " %9.2f 0.025 " " %9.2f 0.015 " " %9.2f 1.005
display "@ b " %9.2f 2.675 " " %9.2f 0.125 " " %9.2f 0.045 " " %9.2f 12.345 " " %9.1f 0.35
display "@ c " %9.1f 0.25000000000000006 " " %9.0f 2.5 " " %9.0f 0.5000000000000001 " " %9.3f 1.0005
