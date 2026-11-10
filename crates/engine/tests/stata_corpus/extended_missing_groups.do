* RCraft's Stata corpus (P2B): made-up data; Stata 19's results are in extended_missing_groups.recorded
* commands: input egen collapse duplicates levelsof contract merge
* (Stata's extended missing values .a to .z as groups: by and bysort, egen by(), by g: summarize, collapse by(), duplicates, levelsof with missing, contract and merge keys each take . and .a to .z as values of their own, in Stata's order . < .a < ... < .z; a .z only in the using data is not matched)
* tolerance: 0
version 19
clear
input double(g y)
1 100
. 200
.a 300
.d 400
.z 500
end
save emg_using, replace
clear
input double(id g x)
1 1 10
2 . 20
3 .a 30
4 .d 40
5 .a 50
6 . 60
7 1 70
8 .d 80
end
save emg_data, replace
use emg_data, clear
* by and bysort: one group per missing value
bysort g (id): generate n = _n
by g: generate nn = _N
by g: egen s = total(x)
by g: generate first = _n == 1
egen s2 = total(x), by(g)
egen m2 = mean(x), by(g)
egen mx2 = max(id), by(g)
duplicates tag g, generate(dt)
sort id
foreach v in n nn s first s2 m2 mx2 dt {
  generate double iw = id * `v'
  quietly summarize iw
  display "@ `v' " r(sum)
  drop iw
}
* the groups in turn: the last one's r() is .d's
by g, sort: summarize x
display "@ by_last " r(N) " " r(sum)
sort id
* duplicates
preserve
duplicates drop g, force
display "@ dup_drop " _N
quietly summarize id
display "@ dup_ids " r(sum)
restore
* levelsof
levelsof g
display "@ lv " r(r)
levelsof g, missing local(lv)
display "@ lvm " r(r)
local i = 0
foreach v of local lv {
  local ++i
  quietly count if g == `v'
  display "@ lv`i' " r(N)
}
* collapse by(): a row per missing value, in Stata's order
preserve
collapse (sum) x (count) id, by(g)
display "@ col_n " _N
generate double row = _n
generate double w = g
local p = 1000
foreach m in . .a .d {
  local ++p
  quietly replace w = `p' if g == `m'
}
generate double rw = row * w
generate double rx = row * x
quietly summarize rw
display "@ col_g " r(sum)
quietly summarize rx
display "@ col_x " r(sum)
restore
* contract
preserve
contract g
display "@ con_n " _N
generate double row = _n
generate double rf = row * _freq
quietly summarize rf
display "@ con_f " r(sum)
restore
* merge on g: . with ., .a with .a; .z only in the using data
preserve
keep if inlist(id, 1, 2, 3, 4)
merge 1:1 g using emg_using
display "@ m11 " _N
generate double w = y * _merge
quietly summarize w
display "@ m11_y " r(sum)
restore
merge m:1 g using emg_using
display "@ m1 " _N
quietly count if _merge == 3
display "@ m1_matched " r(N)
generate double iy = id * y
quietly summarize iy
display "@ m1_y " r(sum)
erase emg_using.dta
erase emg_data.dta
