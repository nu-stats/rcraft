* RCraft's Stata corpus (P2B): made-up data; Stata 19's results are in extended_missing_commands.recorded
* commands: input egen collapse sort gsort tabulate tab1 tab2 fre recode
* (Stata's extended missing values .a to .z in commands: egen's row functions (rowmax and rowfirst of a row with no number give the last variable's value, rowlast the first's, rowmin .), egen group() and tag() with missing (each missing value a group of its own), by-group statistics (.), collapse (max) and (min) of a group with no number (its largest and smallest missing value), (first), (last), (firstnm) and (lastnm) (the value itself), sort and gsort (. < .a < ... < .z, gsort -x puts them last in descending order, mfirst first), tabulate, tab1 and tab2 with missing (a row for each) and generate(), fre's rows of missing values, recode ranges from a number to a missing value)
* tolerance: 0
version 19
clear
input double(id g a b c)
1 1 1 .d 3
2 1 . .a .z
3 2 .d .w .
4 2 .z . .a
5 3 . . .
6 3 .b .b .b
7 4 2 .c 1
8 5 .z .d .
end
save emc_data, replace
use emc_data, clear
erase emc_data.dta
* row functions
egen rmax = rowmax(a b c)
egen rmin = rowmin(a b c)
egen rmean = rowmean(a b c)
egen rtot = rowtotal(a b c)
egen rtotm = rowtotal(a b c), missing
egen rmed = rowmedian(a b c)
egen rfirst = rowfirst(a b c)
egen rlast = rowlast(a b c)
egen rnm = rownonmiss(a b c)
egen rmiss = rowmiss(a b c)
egen r2max = rowmax(b c)
egen r2min = rowmin(b c)
egen r2first = rowfirst(b c)
egen r2last = rowlast(c b)
* groups, tags and statistics by group
by g, sort: egen gmax = max(a)
by g: egen gmin = min(a)
by g: egen gmean = mean(a)
by g: egen gtot = total(a)
by g: egen gtotm = total(a), missing
by g: egen gmed = median(a)
sort id
egen grp = group(a), missing
egen grp2 = group(a)
egen grpb = group(g b), missing
egen tg = tag(a), missing
egen tg2 = tag(a)
* each variable as numbers: . is 1001, .a 1002, ... in Stata's order, with the
* row number, so a value in the wrong row or the wrong missing value shows
foreach v in a b c rmax rmin rmean rtot rtotm rmed rfirst rlast rnm rmiss r2max r2min r2first r2last gmax gmin gmean gtot gtotm gmed grp grp2 grpb tg tg2 {
  generate double w = `v'
  local p = 1000
  foreach m in . .a .b .c .d .w .z {
    local ++p
    quietly replace w = `p' if `v' == `m'
  }
  generate double iw = id * w
  quietly summarize w
  local s = r(sum)
  local n = r(N)
  quietly summarize iw
  display "@ `v' " `n' " " `s' " " r(sum)
  drop w iw
}
* recode ranges by Stata's order (min and max are the numbers')
local i = 0
foreach r in "1/.b = 9" "2/. = 8" ".b/max = 6" "min/.d = 5" ".d/1 = 4" "1/.z = 3" "min/.a = 96" "max/.z = 92" "0/2 = 99" {
  local ++i
  generate double r = a
  recode r (`r')
  generate double w = r
  local p = 1000
  foreach m in . .a .b .c .d .w .z {
    local ++p
    quietly replace w = `p' if r == `m'
  }
  generate double iw = id * w
  quietly summarize w
  local s = r(sum)
  quietly summarize iw
  display "@ recode`i' " `s' " " r(sum)
  drop r w iw
}
* sort and gsort: where each row lands
sort a id
generate double pos = _n
generate double ip = id * pos
quietly summarize ip
display "@ sort_a " r(sum)
drop pos ip
gsort -a id
generate double pos = _n
generate double ip = id * pos
quietly summarize ip
display "@ gsort_desc " r(sum)
drop pos ip
gsort -a id, mfirst
generate double pos = _n
generate double ip = id * pos
quietly summarize ip
display "@ gsort_mfirst " r(sum)
drop pos ip
gsort b -id
generate double pos = _n
generate double ip = id * pos
quietly summarize ip
display "@ gsort_b " r(sum)
drop pos ip
sort id
* tabulate with missing: a row for each missing value
tabulate a, missing
display "@ tab_a " r(N) " " r(r)
tabulate a
display "@ tab_a_nm " r(N) " " r(r)
tabulate a b, missing
display "@ tab_ab " r(N) " " r(r) " " r(c)
tabulate b c, missing
display "@ tab_bc " r(N) " " r(r) " " r(c)
tabulate a, missing sort
display "@ tab_sort " r(N) " " r(r)
tab1 a b, missing
display "@ tab1 " r(N) " " r(r)
tab2 a b, missing
display "@ tab2 " r(N) " " r(r) " " r(c)
tabulate a, missing generate(tm)
local k = 0
foreach v of varlist tm* {
  local ++k
  generate double iw = id * `v' * `k'
  quietly summarize iw
  display "@ tgen`k' " r(N) " " r(sum)
  drop iw
}
drop tm*
* fre: a row for each missing value
fre a
display "@ fre_a " r(N) " " r(N_valid) " " r(N_missing) " " r(r) " " r(r_valid) " " r(r_missing)
fre b
display "@ fre_b " r(N) " " r(N_valid) " " r(N_missing) " " r(r) " " r(r_valid) " " r(r_missing)
* collapse: the value itself, or the largest and smallest missing value
collapse (max) mxa=a mxb=b mxc=c (min) mna=a mnb=b mnc=c (first) fa=a fb=b (last) la=a lb=b (firstnm) fna=a fnb=b fnc=c (lastnm) lna=a lnb=b lnc=c (mean) mea=a (sum) sa=a (count) na=a, by(g)
generate double id = g
foreach v in mxa mxb mxc mna mnb mnc fa fb la lb fna fnb fnc lna lnb lnc mea sa na {
  generate double w = `v'
  local p = 1000
  foreach m in . .a .b .c .d .w .z {
    local ++p
    quietly replace w = `p' if `v' == `m'
  }
  generate double iw = id * w
  quietly summarize w
  local s = r(sum)
  quietly summarize iw
  display "@ `v' " `s' " " r(sum)
  drop w iw
}
