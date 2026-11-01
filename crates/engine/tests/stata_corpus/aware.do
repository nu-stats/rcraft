* RCraft's Stata corpus (P2C part two): made-up data; Stata 19's results are in aware.recorded
* commands: import foreach regress summarize unab levelsof label
* tolerance: 1e-8
version 19
import delimited corpus.csv, clear asdouble
foreach v of varlist x-z {
  quietly summarize `v'
  display "@ mean_`v' " %24.17g r(mean)
}
regress y x-z
display "@ b_z " %24.17g _b[z]
unab vars : x-z cnt
local n : word count `vars'
display "@ nvars " %24.17g `n'
levelsof g, local(gs)
display "@ levels_r " %24.17g r(r)
display "@ levels_N " %24.17g r(N)
foreach k of local gs {
  quietly summarize y if g == `k'
  display "@ ymean_`k' " %24.17g r(mean)
}
levelsof ord if g == 1, local(os)
display "@ ord_r " %24.17g r(r)
local first : word 1 of `os'
display "@ ord_first " %24.17g `first'
levelsof x if id < 4
local second : word 2 of `r(levels)'
display "@ x_second " %24.17g `second'
label variable x "The x"
local lab : variable label x
display "@ lab_ok " %24.17g ("`lab'" == "The x")
summarize c*
display "@ c_mean " %24.17g r(mean)
