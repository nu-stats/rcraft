* RCraft's Stata corpus (P2C.12as, batch N): made-up data; Stata 19's results are in unique_distinct.recorded
* commands: unique distinct
* (unique and distinct from SSC: counts of distinct values and of combinations, missing values and empty strings, strings, if and in, joint, missing, min() and max(), abbrev(); unique's by() with its new variable placed on the first row of each group in sort order, generate(), detail; their error codes)
* tolerance: 0
version 19
set linesize 255
clear
set seed 17
set obs 90
generate id = _n
generate byte a = floor(runiform() * 9)
generate byte b = floor(runiform() * 4)
generate int c = floor(runiform() * 40)
generate str2 s = word("x y z w", 1 + floor(runiform() * 4))
generate double d = round(rnormal(), .1)
replace a = . in 5/7
replace b = . in 20
replace s = "" in 11/12
replace d = . in 30
quietly unique a
display "@ u_a " r(N) " " r(sum) " " r(unique)
quietly unique a b
display "@ u_ab " r(N) " " r(sum) " " r(unique)
quietly unique s
display "@ u_s " r(N) " " r(sum) " " r(unique)
quietly unique d c
display "@ u_dc " r(N) " " r(sum) " " r(unique)
quietly unique a if b == 1
display "@ u_if " r(N) " " r(sum) " " r(unique)
quietly unique a b in 10/50
display "@ u_in " r(N) " " r(sum) " " r(unique)
quietly unique a if id > 1000
display "@ u_none " r(N) " " r(sum) " " r(unique)
unique a, detail
display "@ u_det " r(N) " " r(sum) " " r(unique)
unique a, by(b)
quietly count if !missing(_Unique)
display "@ ub_n " r(N)
quietly summarize _Unique
display "@ ub_s " r(N) " " r(sum) " " r(min) " " r(max)
drop _Unique
unique id a, by(b)
quietly summarize id if !missing(_Unique)
display "@ ubid " r(N) " " r(sum)
drop _Unique
unique id if a > 2, by(b s)
quietly summarize id if !missing(_Unique)
display "@ ubid2 " r(N) " " r(sum)
quietly summarize _Unique
display "@ ubid2_s " r(N) " " r(sum)
drop _Unique
unique a c, by(b s) generate(nn)
quietly summarize nn
display "@ ub2 " r(N) " " r(sum) " " r(min) " " r(max)
unique s if a > 2, by(b)
quietly summarize _Unique
display "@ ubif " r(N) " " r(sum)
drop _Unique
unique d, by(s) detail
display "@ ubdet " r(N) " " r(sum)
quietly summarize _Unique
display "@ ubdet_s " r(N) " " r(sum)
quietly unique a, generate(zz)
capture confirm variable zz
display "@ u_nogen " _rc
capture unique a, by(b) generate(id)
display "@ rc_def " _rc
capture unique
display "@ rc_none " _rc
capture unique nosuch
display "@ rc_nov " _rc
capture unique a, by(nosuch)
display "@ rc_by " _rc
quietly distinct a
display "@ d_a " r(ndistinct) " " r(N)
quietly distinct a b c
display "@ d_abc " r(ndistinct) " " r(N)
quietly distinct s d
display "@ d_sd " r(ndistinct) " " r(N)
quietly distinct a b s, missing
display "@ d_miss " r(ndistinct) " " r(N)
quietly distinct s, missing
display "@ d_smiss " r(ndistinct) " " r(N)
quietly distinct a b c, joint
display "@ d_joint " r(ndistinct) " " r(N)
quietly distinct a b s d, joint missing
display "@ d_jm " r(ndistinct) " " r(N)
quietly distinct a b c, min(5)
display "@ d_min " r(ndistinct) " " r(N)
quietly distinct a b c, max(5)
display "@ d_max " r(ndistinct) " " r(N)
quietly distinct a b c, min(40) max(2)
display "@ d_swap " r(ndistinct) " " r(N)
quietly distinct a b, min(1) max(3)
display "@ d_minmax " r(ndistinct) " " r(N)
quietly distinct a if b == 1
display "@ d_if " r(ndistinct) " " r(N)
quietly distinct b in 10/50
display "@ d_in " r(ndistinct) " " r(N)
quietly distinct a if id > 1000
display "@ d_none " r(ndistinct) " " r(N)
quietly distinct
display "@ d_all " r(ndistinct) " " r(N)
quietly distinct id a, abbrev(2)
display "@ d_abbrev " r(ndistinct) " " r(N)
capture distinct nosuch
display "@ drc_nov " _rc
capture distinct a, by(b)
display "@ drc_by " _rc
capture distinct a, abbrev(x)
display "@ drc_abbrev " _rc
capture distinct a, min(x)
display "@ drc_min " _rc
