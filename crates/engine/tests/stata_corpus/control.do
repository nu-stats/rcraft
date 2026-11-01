* RCraft's Stata corpus (P2C): made-up data; Stata 19's results are in control.recorded
* commands: import include local global foreach forvalues while tempvar summarize regress generate count
* tolerance: 1e-8
version 19
import delimited corpus.csv, clear asdouble
include control_setup.doh
regress $dep `controls'
display "@ b_x " %24.17g _b[x]
display "@ b_w " %24.17g _b[w]
foreach v of local controls {
  summarize `v'
  display "@ mean_`v' " %24.17g r(mean)
  local m = r(mean)
  display "@ macro_`v' " %24.17g `m'
  generate c_`v' = `v' - `m'
  summarize c_`v'
  display "@ csd_`v' " %24.17g r(sd)
}
forvalues k = 0/1 {
  count if g == `k'
  display "@ n_g`k' " %24.17g r(N)
  summarize y if g == `k'
  local s`k' = r(sd)
}
display "@ sd_diff " %24.17g `s1' - `s0'
local i = 1
local total = 0
while `i' <= 3 {
  summarize y if ord == `i'
  local total = `total' + r(N)
  local ++i
}
display "@ total " %24.17g `total'
display "@ i_after " %24.17g `i'
quietly summarize x
if r(mean) > 0 {
  local sign "positive"
}
else {
  local sign "negative"
}
display "@ sign_pos " %24.17g ("`sign'" == "positive")
foreach k of numlist 1 3 {
  quietly count if cat3 == `k'
  display "@ cat3_`k' " %24.17g r(N)
}
local n : word count `controls' y
display "@ nwords " %24.17g `n'
local third = 1/3
display "@ third " %24.17g `third'
tempvar t
generate `t' = x * w
summarize `t'
display "@ xw_mean " %24.17g r(mean)
