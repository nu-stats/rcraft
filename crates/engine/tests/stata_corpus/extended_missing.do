* RCraft's Stata corpus (P2B): made-up data; Stata 19's results are in extended_missing.recorded
* commands: input generate replace count recode summarize
* functions: ceil cond floor inlist int round
* (Stata's extended missing values .a to .z: comparisons with . and .a to .z and between two variables, in Stata's order numbers < . < .a < ... < .z; copies, rows of a variable, cond(), float(), int(), round(), floor() and ceil() keep them, arithmetic and other functions give .; a value .d stored by generate and replace; recode by a missing value and into one; kept by save and use; a do-file's replace x = . if x == .d | x == .w)
* tolerance: 0
version 19
clear
input double x double y g
1 .d 1
. .d 1
.a .w 1
.d .d 2
.w . 2
.z 3 2
5 .a 1
end
save em_data, replace
use em_data, clear
erase em_data.dta
* x against a missing value written out, and a number, for each comparison
foreach m in . .a .d .z 1 {
  local i = 0
  foreach op in == != < <= > >= {
    local ++i
    quietly count if x `op' `m'
    display "@ x`i'`m' " r(N)
  }
}
* the missing value first, and two written out
quietly count if .d < x
display "@ dlt " r(N)
quietly count if . == x
display "@ deq " r(N)
quietly count if .d == .d
display "@ dd " r(N)
quietly count if . < .a
display "@ da " r(N)
quietly count if x > .a & x < .z
display "@ between " r(N)
* two variables that may both be missing
local i = 0
foreach op in == != < <= > >= {
  local ++i
  quietly count if x `op' y
  display "@ xy`i' " r(N)
}
quietly count if missing(x)
display "@ missing " r(N)
quietly count if inlist(x, .d, 1)
display "@ inlist_d " r(N)
quietly count if inlist(x, .)
display "@ inlist_dot " r(N)
quietly count if inlist(x, 5, 1)
display "@ inlist_n " r(N)
* what keeps the tag and what gives .
generate f = x
generate double c = x
generate byte b = x
generate lag = x[_n-1]
generate fl = float(x)
generate it = int(x)
generate ro = round(x)
generate r5 = round(x, 0.5)
generate flo = floor(x)
generate ce = ceil(x)
generate cn = cond(missing(x), x, 0)
generate cn2 = cond(x > 5, y, x)
generate cn3 = cond(g == 1, x + 1, y)
generate ifg = x if g == 1
generate neg = -x
generate m1 = x * 1
generate plusd = x + .d
generate ab = abs(x)
generate sq = sqrt(x)
generate mx = max(x, .)
generate mn = min(x, 5)
generate lit = .d
replace lit = .w in 2
generate r1 = x
recode r1 (.d = 9) (.w = .)
generate r3 = x
recode r3 (1 = .a)
generate r4 = x
recode r4 (. = 8)
generate r5r = x
recode r5r (.a/.w = 5)
generate r6 = x
recode r6 (missing = 7)
generate cl = x
replace cl = . if cl == .d | cl == .w
foreach v in f c b lag fl it ro r5 flo ce cn cn2 cn3 ifg neg m1 plusd ab sq mx mn lit r1 r3 r4 r5r r6 cl {
  quietly count if `v' == .
  local n0 = r(N)
  quietly count if `v' == .a
  local na = r(N)
  quietly count if `v' == .d
  local nd = r(N)
  quietly count if `v' == .w
  local nw = r(N)
  quietly count if `v' == .z
  local nz = r(N)
  quietly summarize `v'
  display "@ `v' " `n0' " " `na' " " `nd' " " `nw' " " `nz' " " r(N) " " r(sum)
}
* the rows where each tag is
quietly summarize g if lit == .w
display "@ lit_w_g " r(sum)
quietly count if r3 == .a in 1
display "@ r3_row1 " r(N)
quietly count if lag == .a in 4
display "@ lag_row4 " r(N)
